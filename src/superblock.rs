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
use core::sync::atomic::AtomicU32;
use spin::Mutex;

pub type DevDesc = u32;
pub struct SuperBlock {
    /// 块设备描述符
    pub dev_desc: DevDesc,
    pub device: Arc<Mutex<dyn Device>>,
    /// 块大小
    pub block_size: usize,
    /// 超级快是否脏
    pub dirty_flag: bool,
    /// 文件最大长度
    pub file_max_bytes: usize,
    /// 挂载标志
    pub mount_flag: MountFlags,
    /// 魔数
    pub magic: u32,
    /// 描述符引用计数
    pub ref_count: u32,
    ///
    pub ref_active: AtomicU32,
    /// 所属文件系统类型，避免循环引用
    pub file_system_type: Weak<Mutex<FileSystemType>>,
    /// 超级快操作
    pub super_block_ops: SuperBlockOps,
    /// 文件系统根节点
    pub root_inode: Arc<Mutex<DirEntry>>,
    /// 脏inode
    pub dirty_inode: Vec<Arc<Mutex<Inode>>>,
    /// 需要同步到磁盘的inode
    pub sync_inode: Vec<Arc<Mutex<Inode>>>,
    /// 打开的文件对象
    pub files: Vec<Arc<Mutex<File>>>,
    /// 块设备名称
    pub blk_dev_name: String,
    /// 其它数据
    pub data: Box<dyn DataOps>,
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
}

unsafe impl Sync for SuperBlock {}
unsafe impl Send for SuperBlock {}

pub trait Device {
    fn read(&self, buf: &mut [u8], offset: usize) -> Result<usize, ()>;
    fn write(&self, buf: &[u8], offset: usize) -> Result<usize, ()>;
}
pub trait DataOps {}

pub struct SuperBlockOps {
    pub alloc_inode: fn(super_blk: Arc<Mutex<SuperBlock>>) -> Arc<Mutex<Inode>>,
    //Deallocates the given inode
    pub destroy_inode: fn(inode: Arc<Mutex<Inode>>),
    //Writes the given inode to disk
    pub write_inode: fn(inode: Arc<Mutex<Inode>>, flag: u32),
    //Makes the given inode dirty
    pub dirty_inode: fn(inode: Arc<Mutex<Inode>>),
    //Deletes the given inode from the disk
    pub delete_inode: fn(inode: Arc<Mutex<Inode>>),
    //Called by the VFS on unmount to release the given superblock object
    pub free_super: fn(super_blk: Arc<Mutex<SuperBlock>>),
    //Writes the given SuperBlock to disk
    pub write_super: fn(super_blk: Arc<Mutex<SuperBlock>>),
    //Synchronizes filesystem metadata with the on-disk filesystem
    pub sync_fs: fn(super_blk: Arc<Mutex<SuperBlock>>),
    //lock the fs
    pub freeze_fs: fn(super_blk: Arc<Mutex<SuperBlock>>),
    //unlock the fs
    pub unfreeze_fs: fn(super_blk: Arc<Mutex<SuperBlock>>),
    //Called by the VFS to obtain filesystem statistics
    pub stat_fs: fn(dentry: Arc<Mutex<DirEntry>>) -> StrResult<StatFs>,
    //Called by the VFS to release the inode and clear any pages containing related data.
    pub clear_inode: fn(inode: Arc<Mutex<Inode>>),
}

pub struct StatFs {}

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
    return match f {
        None => Err("NoFsType"),
        Some((index, _)) => {
            lock.remove(index);
            Ok(())
        }
    };
}

pub fn lookup_filesystem(name: &str) -> Option<Arc<Mutex<FileSystemType>>> {
    let lock = ALL_FS.read();
    let fs_type = lock
        .iter()
        .find(|fs_type| fs_type.lock().name == name)
        .map(|fs_type| fs_type.clone());
    fs_type
}
