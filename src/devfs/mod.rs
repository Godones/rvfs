use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use crate::devfs::dev::{devfs_get_super_blk, devfs_kill_super_blk};
use crate::file::FileMode;
use crate::info::VfsTimeSpec;
use crate::inode::InodeMode;
use crate::superblock::{DataOps, Device, FileSystemAttr, FileSystemType, FileSystemTypeInner};
use spin::{Mutex, MutexGuard};

mod dev;

pub const DEVFS_TYPE: FileSystemType = FileSystemType {
    name: "devfs",
    fs_flags: FileSystemAttr::empty(),
    get_super_blk: devfs_get_super_blk,
    kill_super_blk: devfs_kill_super_blk,
    inner: Mutex::new(FileSystemTypeInner {
        super_blk_s: Vec::new(),
    }),
};

#[derive(Debug)]
pub struct DevNode {
    mode: InodeMode,
    number: usize,
    inner: Mutex<DevNodeInner>,
}
#[derive(Debug)]
#[allow(unused)]
pub struct DevNodeInner {
    access_time: VfsTimeSpec,
    data_modify_time: VfsTimeSpec,
    create_time: VfsTimeSpec,
    meta_modify_time: VfsTimeSpec,
    uid: usize,
    gid: usize,
    dev_type: DevType,
    may_delete: bool,
    name: String,
    parent: Weak<DevNode>,
    perm: FileMode,
}

impl DevNode {
    pub fn access_inner(&self) -> MutexGuard<DevNodeInner> {
        self.inner.lock()
    }
    pub fn new(
        mode: InodeMode,
        number: usize,
        name: String,
        dev_type: DevType,
        perm: FileMode,
    ) -> Self {
        Self {
            mode,
            number,
            inner: Mutex::new(DevNodeInner {
                access_time: VfsTimeSpec::default(),
                data_modify_time: VfsTimeSpec::default(),
                create_time: VfsTimeSpec::default(),
                meta_modify_time: VfsTimeSpec::default(),
                uid: 0,
                gid: 0,
                dev_type,
                may_delete: true,
                name,
                parent: Weak::new(),
                perm,
            }),
        }
    }
}

#[derive(Debug)]
pub struct DevDir {
    children: Vec<Arc<DevNode>>,
    inactive: bool,
}

impl DevDir {
    pub fn empty() -> Self {
        Self {
            children: Vec::new(),
            inactive: false,
        }
    }
}

#[derive(Debug)]
pub enum DevType {
    Dev(u32),
    Dir(DevDir),
    SymLink(String),
    Regular,
}

impl DataOps for Arc<DevNode> {
    fn device(&self, _name: &str) -> Option<Arc<dyn Device>> {
        None
    }
    fn data(&self) -> *const u8 {
        self as *const _ as *const u8
    }
}
