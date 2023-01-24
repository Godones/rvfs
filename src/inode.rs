use crate::dentrry::DirEntry;
use crate::file::FileOps;
use crate::mount::VfsMount;
use crate::superblock::{Device, SuperBlock};
use alloc::rc::Weak;
use alloc::string::String;
use alloc::sync::Arc;
use bitflags::bitflags;
use spin::Mutex;

bitflags! {
    pub struct InodeFlags:u32{
        // 目录被删除了(但是内存中还存在)
        const S_DEL = 0x0;
    }
    pub struct InodeMode:u32{
        const S_IFLNK = 0x0;
        const S_DIR = 0x1;
    }
}

pub struct Inode {
    /// 文件节点编号
    pub number: u32,
    pub hard_links: u32,
    pub state: u32,
    pub flags: InodeFlags,
    pub uid: u32,
    pub gid: u32,
    pub device: u32,
    pub inode_ops: Arc<dyn InodeOps>,
    pub file_ops: Arc<dyn FileOps>,
    /// 如果是块设备
    pub blk_dev: Option<Arc<dyn Device>>,
    pub blk_size_bits: u8,
    pub mode: InodeMode,
    pub file_size: usize,
    pub version: usize,
    pub blk_count: usize,
    pub super_blk: Weak<Arc<Mutex<SuperBlock>>>,
}

pub trait InodeOps {}

bitflags! {
    pub struct LookUpFlags:u32{
        const READ_LINK = 0;
    }
}
const MAX_NESTED_LINKS: usize = 5;

pub struct LookUpData {
    /// 查找标志
    pub flags: LookUpFlags,
    ///  查找到的目录对象
    pub dentry: Arc<Mutex<DirEntry>>,
    /// 已经安装的文件系统对象
    pub mnt: Arc<Mutex<VfsMount>>,
    /// 路径名最后一个分量的类型。如PATHTYPE_NORMAL
    pub path_type: u32,
    /// 符号链接查找的嵌套深度
    pub nested_count: u32,
    /// 嵌套关联路径名数组。
    pub symlink_names: [String; MAX_NESTED_LINKS],
}

pub fn path_walk(mount_dir: &str, flags: LookUpFlags) -> Result<LookUpData, &'static str> {
    unimplemented!()
}

pub fn path_release(lookup_data: &LookUpData) {
    unimplemented!()
}
