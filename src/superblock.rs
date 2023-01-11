use alloc::boxed::Box;
use alloc::collections::LinkedList;
use alloc::sync::Arc;
use core::sync::atomic::AtomicU32;
use spin::Mutex;
use crate::dentrry::DirEntry;
use crate::file::File;
use crate::inode::Inode;
use crate::mount::MountFlag;


pub struct SuperBlock {
    /// 块设备描述符
    pub device: Arc<dyn Device>,
    /// 块大小
    pub block_size: usize,
    /// 块大小bit
    pub block_size_bits: u8,
    /// 超级快是否脏
    pub dirty_flag: bool,
    /// 文件最大长度
    pub file_max_bytes: usize,
    /// 挂载标志
    pub mount_flag:MountFlag,
    /// 魔数
    pub magic: u32,
    /// 描述符引用计数
    pub ref_count:u32,
    ///
    pub ref_active:AtomicU32,
    /// 文件系统类型
    pub file_system_type:Arc<Mutex<FileSystemType>>,
    /// 超级快操作
    pub super_block_ops: Arc<dyn SuperBlockOps>,
    /// 文件系统根节点
    pub root_inode: Arc<DirEntry>,
    /// 脏inode
    pub dirty_inode: LinkedList<Inode>,
    /// 需要同步到磁盘的inode
    pub sync_inode: LinkedList<Inode>,
    /// 打开的文件对象
    pub files:LinkedList<File>,
    /// 块设备名称
    pub blk_dev_name:&'static str,
    /// 其它数据
    pub data:Box<dyn DataOps>,
}

unsafe impl Sync for SuperBlock{}
unsafe impl Send for SuperBlock{}

pub trait Device{
    fn read(&self, buf: &mut [u8], offset: usize) -> Result<usize, ()>;
    fn write(&self, buf: &[u8], offset: usize) -> Result<usize, ()>;
}
pub trait DataOps{}

pub trait SuperBlockOps {
    fn alloc_inode(&self,super_blk: Arc<Mutex<SuperBlock>>) -> Arc<Inode>;
    fn destroy_inode(&self, inode: Arc<Inode>);
    fn write_inode(&self, inode: Arc<Inode>,flag:bool);
    fn dirty_inode(&self, inode: Arc<Inode>);
    fn delete_inode(&self, inode: Arc<Inode>);
    fn put_super(&self,super_blk:Arc<Mutex<SuperBlock>>);
    fn write_super(&self,super_blk:Arc<Mutex<SuperBlock>>);
    fn sync_fs(&self,super_blk:Arc<Mutex<SuperBlock>>);
    fn freeze_fs(&self,super_blk:Arc<Mutex<SuperBlock>>);
    fn unfreeze_fs(&self,super_blk:Arc<Mutex<SuperBlock>>);
    fn stat_fs(&self,dentry:Arc<Mutex<DirEntry>>,buf:&mut StatFs);
    fn clear_inode(&self,inode:Arc<Inode>);
}

pub struct StatFs{

}
pub struct FileSystemType {
    pub name: &'static str,
    pub fs_flags:FileSystemAttr ,
    pub get_sb: Option<fn(fs_type:Arc<Mutex<FileSystemType>>)->Arc<Mutex<SuperBlock>>>,
    pub kill_sb:Option<fn(fs_type:Arc<Mutex<FileSystemType>>)>,
    pub super_blk_s:LinkedList<Arc<Mutex<SuperBlock>>>,
}

pub enum FileSystemAttr{
    RequireDev //位于物理磁盘上
}