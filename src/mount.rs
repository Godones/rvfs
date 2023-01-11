use alloc::collections::LinkedList;
use alloc::sync::Arc;
use spin::Mutex;
use crate::dentrry::DirEntry;
use crate::superblock::SuperBlock;

pub enum MountFlag{

}
/// 挂载点描述符
pub struct VfsMount{
    pub flag:MountFlag,
    pub dev_name:&'static str,
    // pub hash_table
    pub parent:Arc<Mutex<VfsMount>>,
    pub mount_point:Arc<Mutex<DirEntry>>,
    pub root:Arc<Mutex<DirEntry>>,
    pub super_block:Arc<Mutex<SuperBlock>>,
    pub child:LinkedList<Arc<Mutex<VfsMount>>>,
}