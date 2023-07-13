use crate::dentry::{path_walk, LookUpFlags};
use crate::file::{open_dentry, vfs_open_file, File, FileMode, OpenFlags};
use crate::info::{ProcessFs, VfsTime};
use crate::inode::{simple_statfs, Inode, InodeMode};
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

#[derive(Debug, Clone)]
#[repr(C)]
pub struct KStat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    __pad: u64,
    pub st_size: u64,
    pub st_blksize: u32,
    __pad2: u32,
    pub st_blocks: u64,
    pub st_atime_sec: u64,
    pub st_atime_nsec: u64,
    pub st_mtime_sec: u64,
    pub st_mtime_nsec: u64,
    pub st_ctime_sec: u64,
    pub st_ctime_nsec: u64,
    unused: u64,
} //128

/// get file attribute
pub fn vfs_getattr<T: ProcessFs>(file_name: &str, _flag: StatFlags) -> StrResult<KStat> {
    // now we ignore flag
    // assert!(flag.is_empty());
    let file = vfs_open_file::<T>(file_name, OpenFlags::O_RDONLY, FileMode::FMODE_RDWR)?;
    let inode = file.f_dentry.access_inner().d_inode.clone();
    let attr = generic_get_file_attribute(inode);
    Ok(attr)
}

pub fn vfs_getattr_by_file(file: Arc<File>) -> StrResult<KStat> {
    let inode = file.f_dentry.access_inner().d_inode.clone();
    let attr = generic_get_file_attribute(inode);
    Ok(attr)
}

fn generic_get_file_attribute(inode: Arc<Inode>) -> KStat {
    let inner = inode.access_inner();

    // TODO！ update dir size
    const PER_DIR_ENTRY_SIZE: usize = 256;

    let size = match inode.mode {
        InodeMode::S_DIR => inner.file_size * PER_DIR_ENTRY_SIZE,
        _ => inner.file_size,
    };
    let st_blocks = if inode.blk_size == 0{
        0
    }else {
        (inner.file_size / inode.blk_size as usize) as u64
    };

    KStat {
        st_dev: inode.dev_desc as u64,
        st_ino: inode.number as u64,
        st_mode: inode.mode.bits(),
        st_nlink: inner.hard_links,
        st_uid: inner.uid,
        st_gid: inner.gid,
        st_rdev: 0,
        __pad: 0,
        st_size: size as u64,
        st_blksize: inode.blk_size,
        __pad2: 0,
        st_blocks ,
        st_atime_sec: 0,
        st_atime_nsec: 0,
        st_mtime_sec: 0,
        st_mtime_nsec: 0,
        st_ctime_sec: 0,
        st_ctime_nsec: 0,
        unused: 0,
    }
}

/// get file system info according to file name
pub fn vfs_statfs<T: ProcessFs>(file_name: &str) -> StrResult<StatFs> {
    let lookup_data = path_walk::<T>(file_name, LookUpFlags::empty())?;
    let sb_blk = lookup_data.mnt.super_block.clone();
    let statfs = sb_blk.super_block_ops.stat_fs;
    let res = statfs(sb_blk.clone());
    if res.is_ok() {
        return res;
    }
    simple_statfs(sb_blk)
}

pub fn vfs_statfs_by_file(file: Arc<File>) -> StrResult<StatFs> {
    let sb_blk = file
        .f_dentry
        .access_inner()
        .d_inode
        .super_blk
        .upgrade()
        .unwrap();
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

pub fn vfs_getxattr_by_file(file: Arc<File>, key: &str, value: &mut [u8]) -> StrResult<usize> {
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

pub fn vfs_listxattr_by_file(file: Arc<File>, buf: &mut [u8]) -> StrResult<usize> {
    ddebug!("vfs_listxattr_by_file");
    let inode = file.f_dentry.access_inner().d_inode.clone();
    let list_attr = inode.inode_ops.list_attr;
    let len = list_attr(file.f_dentry.clone(), buf)?;
    ddebug!("vfs_listxattr_by_file end");
    Ok(len)
}

pub fn vfs_set_time<T: ProcessFs>(file_name: &str, _time: [VfsTime; 3]) -> StrResult<()> {
    ddebug!("vfs_set_time");
    let _lookup_data = open_dentry::<T>(file_name, OpenFlags::O_RDONLY, FileMode::FMODE_READ)?;
    ddebug!("vfs_set_time end");
    Ok(())
}

bitflags! {
    pub struct StatFlags:u32{
        const AT_EMPTY_PATH = 0x1000;
        const AT_NO_AUTOMOUNT = 0x800;
        const AT_SYMLINK_NOFOLLOW = 0x100;
    }
}
