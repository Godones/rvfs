use crate::dentry::DirEntry;
use crate::file::File;
use crate::inode::Inode;
use crate::mount::MountFlags;
use crate::{StrResult, ALL_FS};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use bitflags::bitflags;
use core::fmt::{Debug, Formatter};
use core::ptr::null;
use spin::{Mutex, MutexGuard};

pub type DevDesc = u32;

#[derive(Debug)]
pub struct SuperBlock {
    /// 块设备描述符
    pub dev_desc: DevDesc,
    pub device: Option<Arc<dyn Device>>,
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
    pub file_system_type: Weak<FileSystemType>,
    /// 超级快操作
    pub super_block_ops: SuperBlockOps,
    /// 块设备名称
    pub blk_dev_name: String,
    /// 其它数据
    pub data: Option<Box<dyn DataOps>>,
    pub inner: Mutex<SuperBlockInner>,
}
#[derive(Debug)]
pub struct SuperBlockInner {
    /// 脏inode
    pub dirty_inode: Vec<Arc<Inode>>,
    /// 需要同步到磁盘的inode
    pub sync_inode: Vec<Arc<Inode>>,
    /// 打开的文件对象
    pub files: Vec<Arc<File>>,
    /// 文件系统根节点
    pub root: Arc<DirEntry>,
}

impl SuperBlockInner {
    pub fn empty() -> Self {
        Self {
            dirty_inode: Vec::new(),
            sync_inode: Vec::new(),
            files: Vec::new(),
            root: Arc::new(DirEntry::empty()),
        }
    }
}

impl SuperBlock {
    #[doc(hidden)]
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
            blk_dev_name: String::new(),
            data: None,
            inner: Mutex::new(SuperBlockInner::empty()),
        }
    }
    pub fn access_inner(&self) -> MutexGuard<SuperBlockInner> {
        self.inner.lock()
    }
}

impl SuperBlock {
    pub fn insert_dirty_inode(&self, inode: Arc<Inode>) {
        self.access_inner().dirty_inode.push(inode);
    }
    pub fn insert_sync_inode(&self, inode: Arc<Inode>) {
        self.access_inner().sync_inode.push(inode);
    }
    pub fn insert_file(&self, file: Arc<File>) {
        self.access_inner().files.push(file);
    }
    pub fn remove_file(&self, file: Arc<File>) {
        self.access_inner().files.retain(|f| !Arc::ptr_eq(f, &file));
    }
    pub fn remove_inode(&self, inode: Arc<Inode>) {
        let mut inner = self.inner.lock();
        inner.dirty_inode.retain(|i| !Arc::ptr_eq(i, &inode));
        inner.sync_inode.retain(|i| !Arc::ptr_eq(i, &inode));
    }
    pub fn update_root(&self, root: Arc<DirEntry>) {
        self.access_inner().root = root;
    }
}

unsafe impl Sync for SuperBlock {}
unsafe impl Send for SuperBlock {}

pub trait Device: Debug + Sync + Send {
    fn read(&self, buf: &mut [u8], offset: usize) -> Result<usize, ()>;
    fn write(&self, buf: &[u8], offset: usize) -> Result<usize, ()>;
    fn size(&self) -> usize;
    fn flush(&self){}
}
pub trait DataOps: Debug {
    fn device(&self, name: &str) -> Option<Arc<dyn Device>>;
    fn data(&self) -> *const u8 {
        null()
    }
}

pub struct SuperBlockOps {
    pub alloc_inode: fn(super_blk: Arc<SuperBlock>) -> StrResult<Arc<Inode>>,
    /// Writes the given inode to disk
    pub write_inode: fn(inode: Arc<Inode>, flag: u32) -> StrResult<()>,
    /// Makes the given inode dirty
    pub dirty_inode: fn(inode: Arc<Inode>) -> StrResult<()>,
    /// Deletes the given inode from the disk
    pub delete_inode: fn(inode: Arc<Inode>) -> StrResult<()>,
    /// Writes the given SuperBlock to disk
    pub write_super: fn(super_blk: Arc<SuperBlock>) -> StrResult<()>,
    /// Synchronizes filesystem metadata with the on-disk filesystem
    pub sync_fs: fn(super_blk: Arc<SuperBlock>) -> StrResult<()>,
    /// lock the fs
    pub freeze_fs: fn(super_blk: Arc<SuperBlock>) -> StrResult<()>,
    /// unlock the fs
    pub unfreeze_fs: fn(super_blk: Arc<SuperBlock>) -> StrResult<()>,
    /// Called by the VFS to obtain filesystem statistics
    pub stat_fs: fn(super_blk: Arc<SuperBlock>) -> StrResult<StatFs>,
}
impl Debug for SuperBlockOps {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SuperBlockOps").finish()
    }
}
impl SuperBlockOps {
    pub const fn empty() -> Self {
        SuperBlockOps {
            alloc_inode: |_| Err("Not support"),
            write_inode: |_, _| Err("Not support"),
            dirty_inode: |_| Err("Not support"),
            delete_inode: |_| Err("Not support"),
            write_super: |_| Err("Not support"),
            sync_fs: |_| Err("Not support"),
            freeze_fs: |_| Err("Not support"),
            unfreeze_fs: |_| Err("Not support"),
            stat_fs: |_| Err("Not support"),
        }
    }
}

#[derive(Debug, Default)]
pub struct StatFs {
    pub fs_type: u32,
    pub block_size: u64,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub total_inodes: u64,
    pub name_len: u32,
    pub name: String,
}

/// 文件系统类型
pub struct FileSystemType {
    pub name: &'static str,
    pub fs_flags: FileSystemAttr,
    pub get_super_blk: fn(
        fs_type: Arc<FileSystemType>,
        flags: MountFlags,
        dev_name: &str,
        data: Option<Box<dyn DataOps>>,
    ) -> StrResult<Arc<SuperBlock>>,
    pub kill_super_blk: fn(super_blk: Arc<SuperBlock>),
    pub inner: Mutex<FileSystemTypeInner>,
}

pub struct FileSystemTypeInner {
    pub super_blk_s: Vec<Arc<SuperBlock>>,
}

impl Debug for FileSystemType {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FileSystemType")
            .field("name", &self.name)
            .field("fs_flags", &self.fs_flags)
            .finish()
    }
}
type F1 = fn(
    fs_type: Arc<FileSystemType>,
    flags: MountFlags,
    dev_name: &str,
    data: Option<Box<dyn DataOps>>,
) -> StrResult<Arc<SuperBlock>>;

type F2 = fn(super_blk: Arc<SuperBlock>);

impl FileSystemType {
    pub const fn new(
        name: &'static str,
        fs_attr: FileSystemAttr,
        get_super_blk: F1,
        kill_super_blk: F2,
    ) -> Self {
        FileSystemType {
            name,
            fs_flags: fs_attr,
            get_super_blk,
            kill_super_blk,
            inner: Mutex::new(FileSystemTypeInner {
                super_blk_s: Vec::new(),
            }),
        }
    }
    pub fn access_inner(&self) -> MutexGuard<FileSystemTypeInner> {
        self.inner.lock()
    }
    pub fn insert_super_blk(&self, super_blk: Arc<SuperBlock>) {
        self.access_inner().super_blk_s.push(super_blk);
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
    let fs_type = lock.iter().find(|fs_type| fs_type.name == fs.name);
    if fs_type.is_some() {
        return Err("fs exist");
    }
    lock.push(Arc::new(fs));
    Ok(())
}
/// 卸载文件系统
pub fn unregister_filesystem(fs_type: FileSystemType) -> Result<(), &'static str> {
    let mut lock = ALL_FS.write();
    let f = lock
        .iter()
        .enumerate()
        .find(|(_, t)| fs_type.name == t.name);
    match f {
        None => Err("NoFsType"),
        Some((index, _)) => {
            lock.remove(index);
            Ok(())
        }
    }
}

pub fn lookup_filesystem(name: &str) -> Option<Arc<FileSystemType>> {
    let lock = ALL_FS.read();
    let fs_type = lock.iter().find(|fs_type| fs_type.name == name).cloned();
    fs_type
}

// 查找超级块
pub fn find_super_blk(
    fs_type: Arc<FileSystemType>,
    test: Option<&dyn Fn(Arc<SuperBlock>) -> bool>,
) -> StrResult<Arc<SuperBlock>> {
    if test.is_none() {
        return Err("No SuperBlk");
    }
    let test = test.unwrap();
    // 根据用户传入的函数，查找超级块
    let inner = fs_type.access_inner();
    let super_blk = inner
        .super_blk_s
        .iter()
        .find(|&super_blk| test(super_blk.clone()));
    match super_blk {
        None => Err("No SuperBlk"),
        Some(super_blk) => Ok(super_blk.clone()),
    }
}
