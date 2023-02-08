use crate::dentrry::{DirEntry, LookUpData};
use crate::file::{FileMode, FileOps};
use crate::superblock::{Device, SuperBlock};
use crate::{wwarn, StatFs, StrResult};
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::sync::Weak;
use bitflags::bitflags;
use core::fmt::{Debug, Formatter};

use crate::stat::FileAttribute;
use spin::Mutex;

bitflags! {
    pub struct InodeFlags:u32{
        // 目录被删除了(但是内存中还存在)
        const S_DEL = 0x1;
    }
    pub struct InodeMode:u32{
        const S_IFLNK = 0x1;
        const S_DIR = 0x2;
        const S_FILE = 0x4;
    }
}

#[derive(Debug)]
pub struct Inode {
    /// 文件节点编号
    pub number: u32,
    pub hard_links: u32,
    pub state: u32,
    pub flags: InodeFlags,
    pub uid: u32,
    pub gid: u32,
    pub dev_desc: u32,
    pub inode_ops: InodeOps,
    pub file_ops: FileOps,
    /// 如果是块设备
    pub blk_dev: Option<Arc<Mutex<dyn Device>>>,
    pub blk_size: u32,
    pub mode: InodeMode,
    pub file_size: usize,
    pub blk_count: usize,
    pub super_blk: Weak<Mutex<SuperBlock>>,
}
impl Inode {
    pub fn empty() -> Self {
        Self {
            number: 0,
            hard_links: 0,
            state: 0,
            flags: InodeFlags::empty(),
            uid: 0,
            gid: 0,
            dev_desc: 0,
            inode_ops: InodeOps::empty(),
            file_ops: FileOps::empty(),
            blk_dev: None,
            mode: InodeMode::empty(),
            file_size: 0,
            blk_count: 0,
            super_blk: Weak::new(),
            blk_size: 0,
        }
    }
}

pub struct InodeOps {
    pub follow_link:
        fn(dentry: Arc<Mutex<DirEntry>>, lookup_data: &mut LookUpData) -> StrResult<()>,
    pub lookup: fn(
        dentry: Arc<Mutex<DirEntry>>,
        lookup_data: &mut LookUpData,
    ) -> StrResult<Arc<Mutex<DirEntry>>>,
    /// 在某一目录下，为与目录项对象相关的普通文件创建一个新的磁盘索引节点。
    pub create:
        fn(dir: Arc<Mutex<Inode>>, dentry: Arc<Mutex<DirEntry>>, mode: FileMode) -> StrResult<()>,
    /// mkdir(dir, dentry, mode)  在某个目录下，为与目录项对应的目录创建一个新的索引节点
    pub mkdir:
        fn(dir: Arc<Mutex<Inode>>, dentry: Arc<Mutex<DirEntry>>, mode: FileMode) -> StrResult<()>,
    /// 在某个目录下，创建一个硬链接
    pub link: fn(
        old_dentry: Arc<Mutex<DirEntry>>,
        dir: Arc<Mutex<Inode>>,
        new_dentry: Arc<Mutex<DirEntry>>,
    ) -> StrResult<()>,
    /// 在某个目录下，删除一个硬链接
    pub unlink: fn(dir: Arc<Mutex<Inode>>, dentry: Arc<Mutex<DirEntry>>) -> StrResult<()>,

    pub get_attr: fn(dentry: Arc<Mutex<DirEntry>>) -> StrResult<FileAttribute>,
}
impl Debug for InodeOps {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InodeOps").finish()
    }
}

impl InodeOps {
    pub const fn empty() -> Self {
        Self {
            follow_link: |_, _| Err("NOT SUPPORT"),
            lookup: |_, _| Err("NOT SUPPORT"),
            create: |_, _, _| Err("NOT SUPPORT"),
            mkdir: |_, _, _| Err("NOT SUPPORT"),
            link: |_, _, _| Err("NOT SUPPORT"),
            unlink: |_, _| Err("NOT SUPPORT"),
            get_attr: |_| Err("NOT SUPPORT"),
        }
    }
}

pub fn generic_delete_inode(inode: Arc<Mutex<Inode>>) {
    let sb_blk = inode.lock().super_blk.upgrade().unwrap();
    // 从超级快中删除inode
    sb_blk.lock().remove_inode(inode);
}

pub fn simple_statfs(sb_blk: Arc<Mutex<SuperBlock>>) -> StrResult<StatFs> {
    let stat = StatFs {
        fs_type: sb_blk.lock().magic,
        block_size: sb_blk.lock().block_size as u32,
        name: sb_blk.lock().blk_dev_name.to_string(),
    };
    Ok(stat)
}

/// 创建一个inode
pub fn create_tmp_inode_from_sb_blk(
    sb_blk: Arc<Mutex<SuperBlock>>,
) -> StrResult<Arc<Mutex<Inode>>> {
    use crate::warn;
    wwarn!("create_tmp_inode_from_sb_blk");
    let create_func = sb_blk.lock().super_block_ops.alloc_inode;
    let res = create_func(sb_blk.clone());
    let inode = match res {
        // 如果文件系统不支持，则需要直接创建
        Ok(inode) => inode,
        Err("Not support") => Arc::new(Mutex::new(Inode::empty())),
        _ => return Err("create inode failed"),
    };
    let mut inode_lk = inode.lock();
    // 设置inode的超级块
    inode_lk.super_blk = Arc::downgrade(&sb_blk);
    // 设置inode的块大小
    inode_lk.blk_size = sb_blk.lock().block_size;
    // 设置inode的块设备
    inode_lk.blk_dev = sb_blk.lock().device.clone();
    // 设置硬链接数
    inode_lk.hard_links = 1;
    drop(inode_lk);
    wwarn!("create_tmp_inode_from_sb_blk end");
    Ok(inode)
}
