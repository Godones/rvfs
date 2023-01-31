#![feature(linked_list_remove)]
#![feature(const_mut_refs)]
#![no_std]
//! virtual file system framework

mod dentrry;
mod file;
mod inode;
mod mount;
mod ramfs;
mod superblock;

extern crate alloc;
use crate::dentrry::DirEntry;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
pub use mount::*;
use spin::{Mutex, RwLock};
use superblock::SuperBlock;
pub use superblock::*;

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
