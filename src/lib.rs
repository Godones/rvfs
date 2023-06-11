#![feature(const_mut_refs)]
#![feature(const_weak_new)]
#![cfg_attr(not(test), no_std)]
#![feature(error_in_core)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::type_complexity, clippy::too_many_arguments)]
//! virtual file system framework
#[macro_use]
extern crate downcast;
pub mod dentry;
pub mod devfs;
pub mod file;
pub mod info;
pub mod inode;
pub mod link;
pub mod mount;
pub mod path;
pub mod ramfs;
pub mod stat;
pub mod superblock;

extern crate alloc;
extern crate log;
use crate::dentry::DirEntry;
use crate::mount::{do_kernel_mount, MountFlags, VfsMount};
use crate::ramfs::rootfs::ROOTFS_TYPE;
use crate::superblock::{register_filesystem, FileSystemType};
use alloc::sync::Arc;
use alloc::vec::Vec;
use info::{ProcessFs, ProcessFsInfo, VfsTime};
use lazy_static::lazy_static;
pub use log::{debug, info, warn};
use spin::{Mutex, RwLock};

pub type StrResult<T> = Result<T, &'static str>;

lazy_static! {
    pub static ref GLOBAL_HASH_MOUNT: RwLock<Vec<Arc<VfsMount>>> = RwLock::new(Vec::new());
}
lazy_static! {
    pub static ref ALL_FS: RwLock<Vec<Arc<FileSystemType>>> = RwLock::new(Vec::new());
}

/// 初始化虚拟文件系统
pub fn mount_rootfs() -> Arc<VfsMount> {
    // 注册内存文件系统
    ddebug!("init_vfs");
    register_filesystem(ROOTFS_TYPE).unwrap();
    // 生成内存文件系统的超级块
    let mnt = do_kernel_mount(
        "rootfs",
        MountFlags::MNT_NO_DEV,
        "root",
        MountFlags::MNT_NO_DEV,
        None,
    )
    .unwrap();

    GLOBAL_HASH_MOUNT.write().push(mnt.clone());
    ddebug!("init_vfs end");
    mnt
}

/// this function is used to init process file system info,but for test
pub fn init_process_info(mnt: Arc<VfsMount>) {
    PROCESS_FS_CONTEXT.lock().cwd = mnt.root.clone();
    PROCESS_FS_CONTEXT.lock().root = mnt.root.clone();
    PROCESS_FS_CONTEXT.lock().cmnt = mnt.clone();
    PROCESS_FS_CONTEXT.lock().rmnt = mnt;
}

pub struct ProcessFsContext {
    /// 当前工作目录
    pub cwd: Arc<DirEntry>,
    /// 根目录
    pub root: Arc<DirEntry>,
    /// 当前挂载点
    pub cmnt: Arc<VfsMount>,
    /// 根挂载点
    pub rmnt: Arc<VfsMount>,
}

lazy_static! {
    pub static ref PROCESS_FS_CONTEXT: Mutex<ProcessFsContext> = Mutex::new(ProcessFsContext {
        cwd: Arc::new(DirEntry::empty()),
        root: Arc::new(DirEntry::empty()),
        cmnt: Arc::new(VfsMount::empty()),
        rmnt: Arc::new(VfsMount::empty()),
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

    fn current_time() -> VfsTime {
        VfsTime::new(0, 0, 0, 0, 0, 0)
    }
}

#[macro_export]
macro_rules! iinfo {
    ($t:expr) => {
        $crate::debug!("[{}] [{}] :{}", file!(), $t, line!());
    };
}

#[macro_export]
macro_rules! wwarn {
    ($t:expr) => {
        $crate::warn!("[{}] [{}] :{}", file!(), $t, line!());
    };
}

#[macro_export]
macro_rules! ddebug {
    ($t:expr) => {
        $crate::debug!("[{}] [{}] :{}", file!(), $t, line!());
    };
}
