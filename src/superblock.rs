use crate::dentrry::DirEntry;
use crate::file::File;
use crate::inode::Inode;
use crate::mount::MountFlags;
use crate::{StrResult, ALL_FS};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;
use bitflags::bitflags;
use core::fmt::{Debug, Formatter};

use spin::Mutex;

pub type DevDesc = u32;

#[derive(Debug)]
pub struct SuperBlock {
    /// 块设备描述符
    pub dev_desc: DevDesc,
    pub device: Option<Arc<Mutex<dyn Device>>>,
    /// 块大小
    pub block_size: u32,
    /// 超级快是否脏
    pub dirty_flag: bool,
    /// 文件最大长度
    pub file_max_bytes: usize,
    /// 挂载标志
    pub mount_flag: MountFlags,
    /// 魔数
    pub magic: u32,
    /// 所属文件系统类型，避免循环引用
    pub file_system_type: Weak<Mutex<FileSystemType>>,
    /// 超级快操作
    pub super_block_ops: SuperBlockOps,
    /// 文件系统根节点
    pub root: Arc<Mutex<DirEntry>>,
    /// 脏inode
    pub dirty_inode: Vec<Arc<Mutex<Inode>>>,
    /// 需要同步到磁盘的inode
    pub sync_inode: Vec<Arc<Mutex<Inode>>>,
    /// 打开的文件对象
    pub files: Vec<Arc<Mutex<File>>>,
    /// 块设备名称
    pub blk_dev_name: String,
    /// 其它数据
    pub data: Option<Box<dyn DataOps>>,
}

impl SuperBlock {
    pub fn empty() -> Self {
        Self {
            dev_desc: 0,
            device: None,
            block_size: 0,
            dirty_flag: false,
            file_max_bytes: 0,
            mount_flag: MountFlags::empty(),
            magic: 0,
            file_system_type: Weak::new(),
            super_block_ops: SuperBlockOps::empty(),
            root: Arc::new(Mutex::new(DirEntry::empty())),
            dirty_inode: Vec::new(),
            sync_inode: Vec::new(),
            files: Vec::new(),
            blk_dev_name: String::new(),
            data: None,
        }
    }
}

impl SuperBlock {
    pub fn insert_dirty_inode(&mut self, inode: Arc<Mutex<Inode>>) {
        self.dirty_inode.push(inode);
    }
    pub fn insert_sync_inode(&mut self, inode: Arc<Mutex<Inode>>) {
        self.sync_inode.push(inode);
    }
    pub fn insert_file(&mut self, file: Arc<Mutex<File>>) {
        self.files.push(file);
    }
    pub fn remove_file(&mut self, file: Arc<Mutex<File>>) {
        self.files.retain(|f| !Arc::ptr_eq(f, &file));
    }
    pub fn remove_inode(&mut self, inode: Arc<Mutex<Inode>>) {
        self.dirty_inode.retain(|i| !Arc::ptr_eq(i, &inode));
        self.sync_inode.retain(|i| !Arc::ptr_eq(i, &inode));
    }
}

unsafe impl Sync for SuperBlock {}
unsafe impl Send for SuperBlock {}

pub trait Device: Debug {
    fn read(&self, buf: &mut [u8], offset: usize) -> Result<usize, ()>;
    fn write(&self, buf: &[u8], offset: usize) -> Result<usize, ()>;
}
pub trait DataOps: Debug {}

pub struct SuperBlockOps {
    pub alloc_inode: fn(super_blk: Arc<Mutex<SuperBlock>>) -> StrResult<Arc<Mutex<Inode>>>,
    /// Writes the given inode to disk
    pub write_inode: fn(inode: Arc<Mutex<Inode>>, flag: u32),
    /// Makes the given inode dirty
    pub dirty_inode: fn(inode: Arc<Mutex<Inode>>),
    /// Deletes the given inode from the disk
    pub delete_inode: fn(inode: Arc<Mutex<Inode>>),
    /// Writes the given SuperBlock to disk
    pub write_super: fn(super_blk: Arc<Mutex<SuperBlock>>),
    /// Synchronizes filesystem metadata with the on-disk filesystem
    pub sync_fs: fn(super_blk: Arc<Mutex<SuperBlock>>),
    /// lock the fs
    pub freeze_fs: fn(super_blk: Arc<Mutex<SuperBlock>>),
    /// unlock the fs
    pub unfreeze_fs: fn(super_blk: Arc<Mutex<SuperBlock>>),
    /// Called by the VFS to obtain filesystem statistics
    pub stat_fs: fn(super_blk: Arc<Mutex<SuperBlock>>) -> StrResult<StatFs>,
}
impl Debug for SuperBlockOps {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SuperBlockOps").finish()
    }
}
impl SuperBlockOps {
    pub fn empty() -> Self {
        SuperBlockOps {
            alloc_inode: |_| Err("Not support"),
            write_inode: |_, _| {},
            dirty_inode: |_| {},
            delete_inode: |_| {},
            write_super: |_| {},
            sync_fs: |_| {},
            freeze_fs: |_| {},
            unfreeze_fs: |_| {},
            stat_fs: |_| Err("Not support"),
        }
    }
}

pub struct StatFs {
    pub fs_type: u32,
    pub block_size: u32,
    pub name: String,
}

/// 文件系统类型
pub struct FileSystemType {
    pub name: &'static str,
    pub fs_flags: FileSystemAttr,
    pub super_blk_s: Vec<Arc<Mutex<SuperBlock>>>,
    pub get_super_blk: fn(
        fs_type: Arc<Mutex<FileSystemType>>,
        flags: MountFlags,
        dev_name: &str,
        data: Option<Box<dyn DataOps>>,
    ) -> StrResult<Arc<Mutex<SuperBlock>>>,
    pub kill_super_blk: fn(super_blk: Arc<Mutex<SuperBlock>>),
}

impl Debug for FileSystemType {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FileSystemType")
            .field("name", &self.name)
            .field("fs_flags", &self.fs_flags)
            .finish()
    }
}
impl FileSystemType {
    pub fn insert_super_blk(&mut self, super_blk: Arc<Mutex<SuperBlock>>) {
        self.super_blk_s.push(super_blk);
    }
}

bitflags! {
    pub struct FileSystemAttr:u32{
        const FS_REQUIRES_DEV = 0x00000001;

    }
}

/// 注册文件系统
pub fn register_filesystem(fs: FileSystemType) -> Result<(), &'static str> {
    // 检查此文件系统类型是否已经注册
    let mut lock = ALL_FS.write();
    let fs_type = lock.iter().find(|fs_type| fs_type.lock().name == fs.name);
    if fs_type.is_some() {
        return Err("fs exist");
    }
    lock.push(Arc::new(Mutex::new(fs)));
    Ok(())
}
/// 卸载文件系统
pub fn unregister_filesystem(fs_type: FileSystemType) -> Result<(), &'static str> {
    let mut lock = ALL_FS.write();
    let f = lock
        .iter()
        .enumerate()
        .find(|(_, t)| fs_type.name == t.lock().name);
    match f {
        None => Err("NoFsType"),
        Some((index, _)) => {
            lock.remove(index);
            Ok(())
        }
    }
}

pub fn lookup_filesystem(name: &str) -> Option<Arc<Mutex<FileSystemType>>> {
    let lock = ALL_FS.read();
    let fs_type = lock
        .iter()
        .find(|fs_type| fs_type.lock().name == name)
        .cloned();
    fs_type
}

// 查找超级块
pub fn find_super_blk(
    fs_type: Arc<Mutex<FileSystemType>>,
    test: Option<&dyn Fn(Arc<Mutex<SuperBlock>>) -> bool>,
) -> StrResult<Arc<Mutex<SuperBlock>>> {
    if test.is_none() {
        return Err("No SuperBlk");
    }
    let test = test.unwrap();
    let lock = fs_type.lock();
    // 根据用户传入的函数，查找超级块
    let super_blk = lock
        .super_blk_s
        .iter()
        .find(|&super_blk| test(super_blk.clone()));
    match super_blk {
        None => Err("No SuperBlk"),
        Some(super_blk) => Ok(super_blk.clone()),
    }
}
