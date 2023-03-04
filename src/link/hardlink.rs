use crate::dentry::{DirEntry, find_file_indir, LookUpFlags, path_walk, PathType};
use crate::info::ProcessFs;
use crate::inode::{InodeFlags, InodeMode};
use alloc::sync::Arc;
use log::info;
use crate::{StrResult, wwarn};

/// decrease the hard link count of a file
/// * name: the path of the file
pub fn vfs_unlink<T: ProcessFs>(name: &str) -> StrResult<()> {
    // 查找文件
    let mut lookup_data = path_walk::<T>(name, LookUpFlags::NOLAST)?;
    // 判断是否是目录
    let inode = lookup_data.dentry.access_inner().d_inode.clone();
    if lookup_data.path_type == PathType::PATH_ROOT {
        return Err("Can not delete root directory");
    }
    if inode.mode != InodeMode::S_DIR {
        return Err("It is not a directory");
    }
    // 搜索子目录
    let last = lookup_data.last.clone();
    let dentry = lookup_data.dentry.clone();
    let inode = dentry.access_inner().d_inode.clone();
    let sub_dentry = find_file_indir(&mut lookup_data, &last);
    if sub_dentry.is_err() {
        return Err("The file does not exist");
    }
    let (_, sub_dentry) = sub_dentry.unwrap();
    // 判断是否是目录
    let sub_inode = sub_dentry.access_inner().d_inode.clone();
    if sub_inode.mode == InodeMode::S_DIR {
        return Err("It is a directory");
    }
    // 调用函数删除文件
    let unlink = inode.inode_ops.unlink;
    unlink(inode.clone(), sub_dentry.clone())?;

    // mark the inode as deleted
    sub_dentry.access_inner().d_inode.access_inner().flags = InodeFlags::S_INVALID;

    dentry.remove_child(&last);

    Ok(())
}

/// create a hard link
/// * old: the path of the old file
/// * new: the path of the new file
pub fn vfs_link<T: ProcessFs>(old: &str, new: &str) -> StrResult<()> {
    wwarn!("vfs_link");
    // 查找old的inode
    let old_lookup_data = path_walk::<T>(old, LookUpFlags::READ_LINK)?;
    // 判断是否是目录
    let old_inode = old_lookup_data.dentry.access_inner().d_inode.clone();
    if old_inode.mode == InodeMode::S_DIR {
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
    let inode = new_lookup_data.dentry.access_inner().d_inode.clone();
    let dentry = new_lookup_data.dentry.clone();
    // 搜索子目录
    let sub_dentry = find_file_indir(&mut new_lookup_data, &last);
    if sub_dentry.is_ok() {
        return Err("The file already exists");
    }

    let target_dentry = Arc::new(DirEntry::from_lookup_data(&new_lookup_data));
    // 调用函数创建一个链接文件
    let do_link = inode.inode_ops.link;
    do_link(
        old_lookup_data.dentry.clone(),
        inode.clone(),
        target_dentry.clone(),
    )?;
    // 确保文件系统完成功能再加入到缓存中
    dentry.insert_child(target_dentry);
    wwarn!("vfs_link: ok");
    Ok(())
}
