use crate::inode::Inode;
use alloc::collections::LinkedList;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use bitflags::bitflags;
use spin::Mutex;
bitflags! {
    pub struct DirFlags:u32{
        const IN_HASH = 0x0;
    }
}

const SHORT_FNAME_LEN: usize = 35;
pub struct DirEntry {
    pub d_flags: DirFlags,
    /// 指向一个inode对象
    pub d_inode: Arc<Mutex<Inode>>,
    /// 父节点
    pub parent: Weak<Mutex<DirEntry>>,
    pub d_ops: DirEntryOps,
    pub d_name: String,
    pub child: LinkedList<Arc<Mutex<DirEntry>>>,
    pub mount_count: u32,
    /// 短文件名
    pub short_name: [u8; SHORT_FNAME_LEN],
}


pub struct  DirEntryOps {
    pub d_hash: fn( dentry: Arc<Mutex<DirEntry>>, name: &str) -> usize,
    pub d_compare: fn ( dentry: Arc<Mutex<DirEntry>>, name: &str) -> bool,
    pub d_delete: fn (dentry: Arc<Mutex<DirEntry>>),
    /// 默认什么都不做
    pub d_release: fn (dentry: Arc<Mutex<DirEntry>>),
    /// 丢弃目录项对应的索引节点
    pub d_iput: fn(dentry: Arc<Mutex<DirEntry>>, inode: Arc<Mutex<Inode>>),
}
