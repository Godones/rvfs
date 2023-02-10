use crate::info::ProcessFs;
use crate::inode::{Inode, InodeMode};
use crate::{path_walk, LookUpFlags, StrResult};
use alloc::sync::Arc;
use spin::Mutex;

#[derive(Debug, Clone)]
pub struct FileAttribute {
    pub dev: u32,
    pub ino: u32,
    pub i_mod: InodeMode,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    // pub rdev: u32,
    pub size: usize,
    pub blksize: u32,
    pub blocks: usize,
}

/// 读取文件的状态信息
pub fn vfs_stat<T: ProcessFs>(file_name: &str) -> StrResult<FileAttribute> {
    let lookup_data = path_walk::<T>(file_name, LookUpFlags::empty())?;
    let inode = lookup_data.dentry.lock().d_inode.clone();
    let get_attr = inode.lock().inode_ops.get_attr;
    let res = get_attr(lookup_data.dentry);
    if res.is_ok() {
        return Ok(res.unwrap());
    }
    let attr = generic_get_file_attribute(inode);
    Ok(attr)
}
/// 读取文件的状态信息
///
/// 在文件系统未实现此功能时默认调用
fn generic_get_file_attribute(inode: Arc<Mutex<Inode>>) -> FileAttribute {
    let sb_blk = inode.lock().super_blk.upgrade().unwrap();
    let sb_blk = sb_blk.lock();
    let inode = inode.lock();
    
    FileAttribute {
        dev: sb_blk.dev_desc,
        ino: inode.number,
        i_mod: inode.mode,
        nlink: inode.hard_links,
        uid: inode.uid,
        gid: inode.gid,
        size: inode.file_size,
        blksize: inode.blk_size,
        blocks: inode.blk_count,
    }
}
