use crate::dentrry::{DirEntry, LookUpData};
use crate::superblock::{Device, SuperBlock};
use alloc::sync::Weak;
use alloc::sync::Arc;
use bitflags::bitflags;
use spin::Mutex;
use crate::StrResult;

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
    pub inode_ops: InodeOps,
    pub file_ops: InodeOps,
    /// 如果是块设备
    pub blk_dev: Option<Arc<dyn Device>>,
    pub blk_size_bits: u8,
    pub mode: InodeMode,
    pub file_size: usize,
    pub version: usize,
    pub blk_count: usize,
    pub super_blk: Weak<Arc<Mutex<SuperBlock>>>,
}

pub struct InodeOps {
    pub follow_link: fn(dentry: Arc<Mutex<DirEntry>>, lookup_data:&mut LookUpData) -> StrResult<()>,
    pub lookup: fn(dentry: Arc<Mutex<DirEntry>>, lookup_data:&mut LookUpData) -> StrResult<Arc<Mutex<DirEntry>>>,
}
