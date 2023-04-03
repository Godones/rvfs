use crate::dentry::{path_walk, LookUpFlags};
use crate::file::{vfs_open_file, OpenFlags, FileMode, File};
use crate::info::{ProcessFs, VfsTime};
use crate::inode::{Inode, InodeMode, simple_statfs};
use crate::superblock::StatFs;
use crate::{ddebug, StrResult};
use alloc::sync::Arc;
use bitflags::bitflags;

#[derive(Debug, Clone)]
#[repr(C)]
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
pub fn vfs_getattr<T: ProcessFs>(file_name: &str,flag:StatFlags) -> StrResult<FileAttribute> {
    // now we ignore flag
    assert!(flag.is_empty());
    let file = vfs_open_file::<T>(file_name, OpenFlags::O_RDONLY, FileMode::FMODE_READ)?;
    let inode = file.f_dentry.access_inner().d_inode.clone();
    let attr = generic_get_file_attribute(inode);
    Ok(attr)
}

pub fn vfs_getattr_by_file(file: Arc<File>) -> StrResult<FileAttribute> {
    let inode = file.f_dentry.access_inner().d_inode.clone();
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
    simple_statfs(sb_blk)
}


pub fn vfs_statfs_by_file(file: Arc<File>) -> StrResult<StatFs> {
    let sb_blk = file.f_dentry.access_inner().d_inode.super_blk.upgrade().unwrap();
    simple_statfs(sb_blk)
}

// set file attribute
pub fn vfs_setxattr<T: ProcessFs>(file_name: &str, key: &str, value: &[u8]) -> StrResult<()> {
    ddebug!("vfs_setxattr");
    let lookup_data = path_walk::<T>(file_name, LookUpFlags::empty())?;
    let inode = lookup_data.dentry.access_inner().d_inode.clone();
    let set_attr = inode.inode_ops.set_attr;
    set_attr(lookup_data.dentry, key, value)?;
    ddebug!("vfs_setxattr end");
    Ok(())
}

pub fn vfs_setxattr_by_file(file: Arc<File>, key: &str, value: &[u8]) -> StrResult<()> {
    ddebug!("vfs_setxattr_by_file");
    let inode = file.f_dentry.access_inner().d_inode.clone();
    let set_attr = inode.inode_ops.set_attr;
    set_attr(file.f_dentry.clone(), key, value)?;
    ddebug!("vfs_setxattr_by_file end");
    Ok(())
}

pub fn vfs_getxattr<T: ProcessFs>(
    file_name: &str,
    key: &str,
    value: &mut [u8],
) -> StrResult<usize> {
    ddebug!("vfs_getxattr");
    let lookup_data = path_walk::<T>(file_name, LookUpFlags::empty())?;
    let inode = lookup_data.dentry.access_inner().d_inode.clone();
    let get_attr = inode.inode_ops.get_attr;
    let len = get_attr(lookup_data.dentry, key, value)?;
    ddebug!("vfs_getxattr end");
    Ok(len)
}


pub fn vfs_getxattr_by_file(
    file:Arc<File>,
    key: &str,
    value: &mut [u8],
) -> StrResult<usize> {
    ddebug!("vfs_getxattr_by_file");
    let inode = file.f_dentry.access_inner().d_inode.clone();
    let get_attr = inode.inode_ops.get_attr;
    let len = get_attr(file.f_dentry.clone(), key, value)?;
    ddebug!("vfs_getxattr_by_file end");
    Ok(len)
}


pub fn vfs_removexattr<T: ProcessFs>(file_name: &str, key: &str) -> StrResult<()> {
    ddebug!("vfs_removexattr");
    let lookup_data = path_walk::<T>(file_name, LookUpFlags::empty())?;
    let inode = lookup_data.dentry.access_inner().d_inode.clone();
    let remove_attr = inode.inode_ops.remove_attr;
    remove_attr(lookup_data.dentry, key)?;
    ddebug!("vfs_removexattr end");
    Ok(())
}

pub fn vfs_removexattr_by_file(file: Arc<File>, key: &str) -> StrResult<()> {
    ddebug!("vfs_removexattr_by_file");
    let inode = file.f_dentry.access_inner().d_inode.clone();
    let remove_attr = inode.inode_ops.remove_attr;
    remove_attr(file.f_dentry.clone(), key)?;
    ddebug!("vfs_removexattr_by_file end");
    Ok(())
}


pub fn vfs_listxattr<T: ProcessFs>(file_name: &str, buf: &mut [u8]) -> StrResult<usize> {
    ddebug!("vfs_listxattr");
    let lookup_data = path_walk::<T>(file_name, LookUpFlags::empty())?;
    let inode = lookup_data.dentry.access_inner().d_inode.clone();
    let list_attr = inode.inode_ops.list_attr;
    let len = list_attr(lookup_data.dentry, buf)?;
    ddebug!("vfs_listxattr end");
    Ok(len)
}

pub fn vfs_listxattr_by_file(file:Arc<File>,buf:&mut [u8])->StrResult<usize>{
    ddebug!("vfs_listxattr_by_file");
    let inode = file.f_dentry.access_inner().d_inode.clone();
    let list_attr = inode.inode_ops.list_attr;
    let len = list_attr(file.f_dentry.clone(), buf)?;
    ddebug!("vfs_listxattr_by_file end");
    Ok(len)
}


bitflags! {
    pub struct StatFlags:u32{
        const AT_EMPTY_PATH = 0x1000;
        const AT_NO_AUTOMOUNT = 0x800;
        const AT_SYMLINK_NOFOLLOW = 0x100;
    }
}