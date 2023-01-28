#![feature(linked_list_remove)]
#![no_std]
//! virtual file system framework

mod dentrry;
mod file;
mod inode;
mod mount;
mod superblock;

extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::{Mutex, RwLock};
use superblock::SuperBlock;
pub use mount::*;
pub use superblock::*;
use crate::dentrry::DirEntry;

pub type StrResult<T> = Result<T, &'static str>;



lazy_static! {
    pub static ref SUPERBLOCKS: Mutex<Vec<SuperBlock>> = Mutex::new(Vec::new());
}

lazy_static! {
    pub static ref GLOBAL_DIRENTRY: RwLock<Vec<Arc<Mutex<DirEntry>>>> = RwLock::new(Vec::new());
}
lazy_static! {
    pub static ref GLOBAL_HASH_MOUNT: RwLock<Vec<Arc<Mutex<VfsMount>>>> = RwLock::new(Vec::new());
}
lazy_static! {
    pub static ref ALL_FS: RwLock<Vec<Arc<Mutex<FileSystemType>>>> = RwLock::new(Vec::new());
}
