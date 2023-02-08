use crate::dentrry::DirEntry;
use crate::info::ProcessFs;
use crate::inode::InodeMode;
use crate::{find_file_indir, path_walk, wwarn, LookUpFlags, PathType, StrResult};
use alloc::sync::Arc;
use logger::{info, warn};
use spin::Mutex;
/// 删除文件
pub fn vfs_unlink<T: ProcessFs>(name: &str) -> StrResult<()> {
    // 查找文件
    let mut lookup_data = path_walk::<T>(name, LookUpFlags::NOLAST)?;
    // 判断是否是目录
    let inode = lookup_data.dentry.lock().d_inode.clone();
    if lookup_data.path_type == PathType::PATH_ROOT {
        return Err("Can not delete root directory");
    }
    if inode.lock().mode != InodeMode::S_DIR {
        return Err("It is not a directory");
    }
    // 搜索子目录
    let last = lookup_data.last.clone();
    let dentry = lookup_data.dentry.clone();
    let inode = dentry.lock().d_inode.clone();
    let sub_dentry = find_file_indir(&mut lookup_data, &last);
    if sub_dentry.is_err() {
        return Err("The file does not exist");
    }
    let (_, sub_dentry) = sub_dentry.unwrap();
    // 判断是否是目录
    let sub_inode = sub_dentry.lock().d_inode.clone();
    if sub_inode.lock().mode == InodeMode::S_DIR {
        return Err("It is a directory");
    }
    // 调用函数删除文件
    let unlink = inode.lock().inode_ops.unlink;
    unlink(inode.clone(), sub_dentry.clone())?;
    dentry.lock().remove_child(&last);

    Ok(())
}

/// 创建硬链接链接
///
/// 将系统调用功能下放至vfs层，由vfs层实现大部分逻辑
pub fn vfs_link<T: ProcessFs>(old: &str, new: &str) -> StrResult<()> {
    wwarn!("vfs_link");
    // 查找old的inode
    let old_lookup_data = path_walk::<T>(old, LookUpFlags::READ_LINK)?;
    // 判断是否是目录
    let old_inode = old_lookup_data.dentry.lock().d_inode.clone();
    if old_inode.lock().mode == InodeMode::S_DIR {
        return Err("It is a directory");
    }
    // 查找new的inode
    // 如果没有找到则新建一个
    let new_lookup_data = path_walk::<T>(new, LookUpFlags::NOLAST);
    if new_lookup_data.is_err() {
        return Err("vfs_link: new path not found");
    }
    let mut new_lookup_data = new_lookup_data.unwrap();
    info!(
        "vfs_link: new_lookup_data.path_type = {:?}",
        new_lookup_data.path_type
    );
    if new_lookup_data.path_type != PathType::PATH_NORMAL {
        return Err("It is not a normal path");
    }
    // 判断是否在同一个文件系统下面
    let old_mnt = &old_lookup_data.mnt;
    let new_mnt = &new_lookup_data.mnt;
    if !Arc::ptr_eq(old_mnt, new_mnt) {
        return Err("It is not in the same file system");
    }

    let last = new_lookup_data.last.clone();
    let inode = new_lookup_data.dentry.lock().d_inode.clone();
    let dentry = new_lookup_data.dentry.clone();
    // 搜索子目录
    let sub_dentry = find_file_indir(&mut new_lookup_data, &last);
    if sub_dentry.is_ok() {
        return Err("The file already exists");
    }
    // 调用函数创建一个链接文件
    let target_dentry = Arc::new(Mutex::new(DirEntry::empty()));
    // 设置目录名
    target_dentry.lock().d_name = last;
    // 设置父子关系
    target_dentry.lock().parent = Arc::downgrade(&dentry);
    dentry.lock().insert_child(target_dentry.clone());

    let do_link = inode.lock().inode_ops.link;
    do_link(
        old_lookup_data.dentry.clone(),
        inode.clone(),
        target_dentry.clone(),
    )?;
    wwarn!("vfs_link: ok");
    Ok(())
}
