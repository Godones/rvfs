use crate::dentrry::{DirEntry, DirFlags};
use crate::inode::{generic_delete_inode, simple_statfs, Inode, create_inode, InodeMode, InodeOps};
use crate::superblock::{FileSystemType, SuperBlock};
use crate::{
    find_super_blk, DataOps, Device, FileSystemAttr, MountFlags, StrResult, SuperBlockOps,
};
use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter};
use lazy_static::lazy_static;
use spin::Mutex;
use crate::file::{FileOps, generic_file_mmap, generic_file_read, generic_file_write, read_file};

const fn root_fs_type() -> FileSystemType {
    let fs_type = FileSystemType {
        name: "rootfs",
        fs_flags: FileSystemAttr::empty(),
        super_blk_s: Vec::new(),
        get_super_blk: rootfs_get_super_blk,
        kill_super_blk: rootfs_kill_super_blk,
    };
    fs_type
}

const fn root_fs_sb_blk_ops() -> SuperBlockOps {
    SuperBlockOps {
        alloc_inode: |_| Err("Not support"),
        write_inode: |_, _| {},
        dirty_inode: |_| {},
        delete_inode: generic_delete_inode,
        write_super: |_| {},
        sync_fs: |_| {},
        freeze_fs: |_| {},
        unfreeze_fs: |_| {},
        stat_fs: simple_statfs,
    }
}

const fn root_fs_inode_ops()->InodeOps{
    let ops = InodeOps::empty();
    ops
}

const fn root_fs_file_ops()->FileOps{
    let mut ops = FileOps::empty();
    ops.read = generic_file_read;
    ops.write = generic_file_write;
    ops.mmap = generic_file_mmap;
    ops
}



struct FakeRamDevice;

impl Device for FakeRamDevice {
    fn read(&self, buf: &mut [u8], offset: usize) -> Result<usize, ()> {
        Ok(0)
    }
    fn write(&self, buf: &[u8], offset: usize) -> Result<usize, ()> {
        Ok(0)
    }
}

const RAM_BLOCK_SIZE: u32 = 4096;
const RAM_FILE_MAX_SIZE: usize = 4096;
const RAM_MAGIC: u32 = 0x12345678;

fn create_ram_super_blk(
    fs_type: Arc<Mutex<FileSystemType>>,
    flags: MountFlags,
    dev_name: &str,
    data: Option<Box<dyn DataOps>>,
) -> StrResult<Arc<Mutex<SuperBlock>>> {
    let sb_blk = SuperBlock {
        dev_desc: 0,
        device: Arc::new(Mutex::new(FakeRamDevice)),
        block_size: RAM_BLOCK_SIZE,
        dirty_flag: false,
        file_max_bytes: RAM_FILE_MAX_SIZE,
        mount_flag: flags,
        magic: RAM_MAGIC,
        file_system_type: Arc::downgrade(&fs_type),
        super_block_ops: root_fs_sb_blk_ops(),
        root: Arc::new(Mutex::new(DirEntry::empty())),
        dirty_inode: vec![],
        sync_inode: vec![],
        files: vec![],
        blk_dev_name: dev_name.to_string(),
        data,
    };
    let sb_blk = Arc::new(Mutex::new(sb_blk));
    Ok(sb_blk)
}

fn rootfs_get_super_blk(
    fs_type: Arc<Mutex<FileSystemType>>,
    flags: MountFlags,
    dev_name: &str,
    data: Option<Box<dyn DataOps>>,
) -> StrResult<Arc<Mutex<SuperBlock>>> {
    let find_sb_blk = find_super_blk(fs_type.clone(), None);
    // 找到了旧超级快
    let sb_blk = match find_sb_blk {
        Ok(sb_blk) => sb_blk,
        Err(_) => {
            // 没有找到旧超级快需要重新分配
            let sb_blk = create_ram_super_blk(fs_type, flags, dev_name, data)?;
            sb_blk
        }
    };
    let inode = create_ram_fs_inode(sb_blk.clone(),InodeMode::S_DIR)?;
    let dentry =create_ram_fs_dentry(None,inode)?;
    sb_blk.lock().root = dentry;
    Ok(sb_blk)
}

fn rootfs_kill_super_blk(super_blk: Arc<Mutex<SuperBlock>>) {}

/// 创建内存文件系统的inode
fn create_ram_fs_inode(sb_blk: Arc<Mutex<SuperBlock>>,mode:InodeMode) -> StrResult<Arc<Mutex<Inode>>> {
    let inode = create_inode(sb_blk)?;
    let mut inode_lk = inode.lock();
    inode_lk.mode = mode;
    inode_lk.blk_size = RAM_BLOCK_SIZE;
    inode_lk.blk_count = 0;
    // TODO 设置uid/gid
    match mode {
        InodeMode::S_DIR => {
            inode_lk.inode_ops = root_fs_inode_ops();
            inode_lk.file_ops = root_fs_file_ops();
            inode_lk.hard_links +=1
        }
        InodeMode::S_FILE => {
            inode_lk.inode_ops = root_fs_inode_ops();
            inode_lk.file_ops = root_fs_file_ops()
        }
        _ => {
            return Err("Not support");
        }
    }
    drop(inode_lk);
    Ok(inode)
}

fn create_ram_fs_dentry(
    parent: Option<Arc<Mutex<DirEntry>>>,
    inode: Arc<Mutex<Inode>>,
) -> StrResult<Arc<Mutex<DirEntry>>> {
    let mut dentry = DirEntry::empty();
    if parent.is_some(){
        dentry.parent = Arc::downgrade(&(parent.unwrap()));
    }
    dentry.d_inode = inode;
    dentry.d_name = "/".to_string();
    Ok(Arc::new(Mutex::new(dentry)))
}