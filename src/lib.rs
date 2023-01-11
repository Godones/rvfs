//! virtual file system framework
#![no_std]

mod superblock;
mod dentrry;
mod inode;
mod file;
mod mount;

extern crate alloc;

use lazy_static::lazy_static;
use spin::Mutex;
use alloc::collections::LinkedList;
use alloc::sync::Arc;
use superblock::SuperBlock;
use hashbrown::HashMap;
use crate::mount::VfsMount;
use crate::superblock::FileSystemType;


lazy_static!{
    pub static ref SUPERBLOCKS: Mutex<LinkedList<SuperBlock>> = Mutex::new(LinkedList::new());
}
lazy_static!{
    pub static ref GLOBAL_HASH_MOUNT:Mutex<HashMap<usize,Arc<Mutex<VfsMount>>>> = Mutex::new(HashMap::new());
}
lazy_static!{
    pub static ref ALL_FS:Mutex<LinkedList<Arc<Mutex<FileSystemType>>>> = Mutex::new(LinkedList::new());
}

// pub static SUPER_BLOCKS: Mutex<LinkedList<SuperBlock>> = Mutex::new(LinkedList::new());
/// 注册文件系统
///
///
pub fn register_filesystem(fs: Arc<Mutex<FileSystemType>>){
    // 通过检查
    ALL_FS.lock().push_back(fs);
}