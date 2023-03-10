use crate::info::{ProcessFs, VfsTime};
use crate::inode::{Inode, InodeMode};
use alloc::string::ToString;
use alloc::sync::Arc;
use crate::dentry::{LookUpFlags, path_walk};
use crate::{StrResult, wwarn};
use crate::superblock::StatFs;

#[derive(Debug, Clone)]
pub struct FileAttribute {
    pub dev: u32,
    pub ino: usize,
    pub i_mod: InodeMode,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    // pub rdev: u32,
    pub size: usize,
    pub blksize: u32,
    pub blocks: usize,
    pub atime: VfsTime,
    pub mtime: VfsTime,
    pub ctime: VfsTime,
}

/// get file attribute
pub fn vfs_getattr<T: ProcessFs>(file_name: &str) -> StrResult<FileAttribute> {
    let lookup_data = path_walk::<T>(file_name, LookUpFlags::empty())?;
    let inode = lookup_data.dentry.access_inner().d_inode.clone();
    let attr = generic_get_file_attribute(inode);
    Ok(attr)
}
/// 读取文件的状态信息
///
/// 在文件系统未实现此功能时默认调用
fn generic_get_file_attribute(inode: Arc<Inode>) -> FileAttribute {
    let sb_blk = inode.super_blk.upgrade().unwrap();
    let sb_blk = sb_blk;
    let inode = inode;

    let inner = inode.access_inner();
    FileAttribute {
        dev: sb_blk.dev_desc,
        ino: inode.number,
        i_mod: inode.mode,
        nlink: inner.hard_links,
        uid: inner.uid,
        gid: inner.gid,
        size: inner.file_size,
        blksize: inode.blk_size,
        blocks: inner.file_size / sb_blk.block_size as usize,
        atime: Default::default(),
        mtime: Default::default(),
        ctime: Default::default(),
    }
}
/// get file system info according to file name
pub fn vfs_statfs<T: ProcessFs>(file_name: &str) -> StrResult<StatFs> {
    let lookup_data = path_walk::<T>(file_name, LookUpFlags::empty())?;
    let sb_blk = lookup_data.mnt.super_block.clone();
    let fs_type = sb_blk.file_system_type.upgrade().unwrap();

    let sb_blk = sb_blk;
    let stat_fs = StatFs {
        fs_type: sb_blk.magic,
        block_size: sb_blk.block_size as u64,
        total_blocks: 0,
        free_blocks: 0,
        total_inodes: 0,
        name_len: 0,
        name: fs_type.name.to_string(),
    };
    Ok(stat_fs)
}

// set file attribute
pub fn vfs_setxattr<T: ProcessFs>(file_name: &str, key: &str, value: &[u8]) -> StrResult<()> {
    wwarn!("vfs_setxattr");
    let lookup_data = path_walk::<T>(file_name, LookUpFlags::empty())?;
    let inode = lookup_data.dentry.access_inner().d_inode.clone();
    let set_attr = inode.inode_ops.set_attr;
    set_attr(lookup_data.dentry, key, value)?;
    wwarn!("vfs_setxattr end");
    Ok(())
}

pub fn vfs_getxattr<T: ProcessFs>(
    file_name: &str,
    key: &str,
    value: &mut [u8],
) -> StrResult<usize> {
    wwarn!("vfs_getxattr");
    let lookup_data = path_walk::<T>(file_name, LookUpFlags::empty())?;
    let inode = lookup_data.dentry.access_inner().d_inode.clone();
    let get_attr = inode.inode_ops.get_attr;
    let len = get_attr(lookup_data.dentry, key, value)?;
    wwarn!("vfs_getxattr end");
    Ok(len)
}

pub fn vfs_removexattr<T: ProcessFs>(file_name: &str, key: &str) -> StrResult<()> {
    wwarn!("vfs_removexattr");
    let lookup_data = path_walk::<T>(file_name, LookUpFlags::empty())?;
    let inode = lookup_data.dentry.access_inner().d_inode.clone();
    let remove_attr = inode.inode_ops.remove_attr;
    remove_attr(lookup_data.dentry, key)?;
    wwarn!("vfs_removexattr end");
    Ok(())
}

pub fn vfs_listxattr<T: ProcessFs>(file_name: &str, buf: &mut [u8]) -> StrResult<usize> {
    wwarn!("vfs_listxattr");
    let lookup_data = path_walk::<T>(file_name, LookUpFlags::empty())?;
    let inode = lookup_data.dentry.access_inner().d_inode.clone();
    let list_attr = inode.inode_ops.list_attr;
    let len = list_attr(lookup_data.dentry, buf)?;
    wwarn!("vfs_listxattr end");
    Ok(len)
}
