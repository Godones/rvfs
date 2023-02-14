#![feature(linked_list_remove)]
#![feature(const_mut_refs)]
#![no_std]
//! virtual file system framework

mod dentrry;
mod file;
mod info;
mod inode;
mod link;
mod mount;
pub mod ramfs;
mod stat;
mod superblock;

#[macro_use]
extern crate log;
extern crate alloc;
use crate::dentrry::DirEntry;
use alloc::sync::Arc;
use alloc::vec::Vec;

use lazy_static::lazy_static;
pub use mount::*;
use spin::{Mutex, RwLock};
use superblock::SuperBlock;

use crate::info::{ProcessFs, ProcessFsInfo};
use crate::ramfs::rootfs::root_fs_type;
pub use log::{info,warn,error};
pub use dentrry::*;
pub use file::*;
pub use superblock::*;

pub use link::*;
pub use stat::*;
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

/// 初始化虚拟文件系统
pub fn init_vfs() {
    // 注册内存文件系统
    iinfo!("init_vfs");
    register_filesystem(root_fs_type()).unwrap();
    // 生成内存文件系统的超级块
    let mnt = do_kernel_mount("rootfs", MountFlags::MNT_NO_DEV, "", None).unwrap();
    // info!("[init_vfs] mnt: {:#?}", mnt);
    // 设置进程的文件系统相关信息
    // for test
    PROCESS_FS_CONTEXT.lock().cwd = mnt.lock().root.clone();
    PROCESS_FS_CONTEXT.lock().root = mnt.lock().root.clone();
    PROCESS_FS_CONTEXT.lock().cmnt = mnt.clone();
    PROCESS_FS_CONTEXT.lock().rmnt = mnt.clone();
    iinfo!("init_vfs end");
    GLOBAL_HASH_MOUNT.write().push(mnt);
}

pub struct ProcessFsContext {
    /// 当前工作目录
    pub cwd: Arc<Mutex<DirEntry>>,
    /// 根目录
    pub root: Arc<Mutex<DirEntry>>,
    /// 当前挂载点
    pub cmnt: Arc<Mutex<VfsMount>>,
    /// 根挂载点
    pub rmnt: Arc<Mutex<VfsMount>>,
}

lazy_static! {
    pub static ref PROCESS_FS_CONTEXT: Mutex<ProcessFsContext> = Mutex::new(ProcessFsContext {
        cwd: Arc::new(Mutex::new(DirEntry::empty())),
        root: Arc::new(Mutex::new(DirEntry::empty())),
        cmnt: Arc::new(Mutex::new(VfsMount::empty())),
        rmnt: Arc::new(Mutex::new(VfsMount::empty())),
    });
}

pub struct FakeFSC;

impl ProcessFs for FakeFSC {
    fn get_fs_info() -> ProcessFsInfo {
        let lock = PROCESS_FS_CONTEXT.lock();
        ProcessFsInfo::new(
            lock.rmnt.clone(),
            lock.root.clone(),
            lock.cwd.clone(),
            lock.cmnt.clone(),
        )
    }
    fn check_nested_link() -> bool {
        false
    }
    fn update_link_data() {}
    fn max_link_count() -> u32 {
        0
    }
}

#[macro_export]
macro_rules! iinfo {
    ($t:expr) => {
        info!("[{}] [{}] :{}", file!(), $t, line!());
    };
}

#[macro_export]
macro_rules! wwarn {
    ($t:expr) => {
        warn!("[{}] [{}] :{}", file!(), $t, line!());
    };
}
