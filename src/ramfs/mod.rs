use crate::dentrry::DirEntry;
use crate::file::{generic_file_mmap, FileOps};
use crate::inode::{
    create_tmp_inode_from_sb_blk, generic_delete_inode, simple_statfs, Inode, InodeMode, InodeOps,
};
use crate::superblock::{FileSystemType, SuperBlock};
use crate::{
    find_super_blk, wwarn, DataOps, File, FileMode, FileSystemAttr, MountFlags, StrResult,
    SuperBlockOps,
};
use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use core::sync::atomic::AtomicU32;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use logger::{info, warn};
use spin::Mutex;

static INODE_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(Clone)]
pub struct RamFsInode {
    // 节点号
    number: u32,
    data: Vec<u8>,
    // 类型
    mode: InodeMode,
    hard_link: u32,
    // 读写权限
    attr: FileMode,
}

impl RamFsInode {
    pub fn new(mode: InodeMode, attr: FileMode, number: u32) -> Self {
        let h_link = if mode == InodeMode::S_DIR { 2 } else { 1 };
        Self {
            number,
            data: Vec::new(),
            mode,
            hard_link: h_link,
            attr,
        }
    }
}

lazy_static! {
    static ref RAM_FS: Mutex<HashMap<u32, RamFsInode>> = Mutex::new(HashMap::new());
}

pub const fn root_fs_type() -> FileSystemType {
    let fs_type = FileSystemType {
        name: "ramfs",
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

const fn root_fs_inode_ops() -> InodeOps {
    let mut ops = InodeOps::empty();
    ops.mkdir = rootfs_mkdir;
    ops.create = rootfs_create;
    ops
}

const fn root_fs_file_ops() -> FileOps {
    let mut ops = FileOps::empty();
    ops.read = root_fs_read_file;
    ops.write = root_fs_write_file;
    ops.mmap = generic_file_mmap;
    ops.open = |_| Ok(());
    ops
}

const RAM_BLOCK_SIZE: u32 = 4096;
const RAM_FILE_MAX_SIZE: usize = 4096;
const RAM_MAGIC: u32 = 0x12345678;

/// 创建一个内存文件系统的超级块
fn create_ram_super_blk(
    fs_type: Arc<Mutex<FileSystemType>>,
    flags: MountFlags,
    dev_name: &str,
    data: Option<Box<dyn DataOps>>,
) -> StrResult<Arc<Mutex<SuperBlock>>> {
    let sb_blk = SuperBlock {
        dev_desc: 0,
        device: None,
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
    wwarn!("rootfs_get_super_blk");
    let find_sb_blk = find_super_blk(fs_type.clone(), None);
    let sb_blk = match find_sb_blk {
        // 找到了旧超级快
        Ok(sb_blk) => sb_blk,
        Err(_) => {
            // 没有找到旧超级快需要重新分配
            info!("create new super block for ramfs");
            let sb_blk = create_ram_super_blk(fs_type.clone(), flags, dev_name, data)?;
            sb_blk
        }
    };
    let inode = create_ram_fs_root_inode(sb_blk.clone(), InodeMode::S_DIR)?;
    // 根目录硬链接计数不用自增1
    inode.lock().hard_links -= 1;
    // 创建目录项
    let dentry = create_ram_fs_root_dentry(None, inode)?;
    sb_blk.lock().root = dentry;
    // 将sb_blk插入到fs_type的链表中
    fs_type.lock().insert_super_blk(sb_blk.clone());
    wwarn!("rootfs_get_super_blk end");
    Ok(sb_blk)
}

fn rootfs_kill_super_blk(_super_blk: Arc<Mutex<SuperBlock>>) {}

/// 创建内存文件系统的inode
fn create_ram_fs_root_inode(
    sb_blk: Arc<Mutex<SuperBlock>>,
    mode: InodeMode,
) -> StrResult<Arc<Mutex<Inode>>> {
    let inode = create_tmp_inode_from_sb_blk(sb_blk)?;
    let mut inode_lk = inode.lock();
    inode_lk.mode = mode;
    inode_lk.blk_count = 0;
    // 设置inode的编号
    inode_lk.number = INODE_COUNT.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
    // TODO 设置uid/gid
    match mode {
        InodeMode::S_DIR => {
            inode_lk.inode_ops = root_fs_inode_ops();
            inode_lk.file_ops = root_fs_file_ops();
            inode_lk.hard_links += 1
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
    // 插入根inode
    let mut ram_inode = RamFsInode::new(mode, FileMode::FMODE_WRITE, 0);
    ram_inode.hard_link -= 1;
    RAM_FS.lock().insert(0, ram_inode);
    Ok(inode)
}

fn create_ram_fs_root_dentry(
    parent: Option<Arc<Mutex<DirEntry>>>,
    inode: Arc<Mutex<Inode>>,
) -> StrResult<Arc<Mutex<DirEntry>>> {
    let mut dentry = DirEntry::empty();
    if parent.is_some() {
        dentry.parent = Arc::downgrade(&(parent.unwrap()));
    }
    dentry.d_inode = inode;
    dentry.d_name = "/".to_string();
    Ok(Arc::new(Mutex::new(dentry)))
}

fn rootfs_create_inode(
    dir: Arc<Mutex<Inode>>,
    mode: InodeMode,
    attr: FileMode,
) -> StrResult<Arc<Mutex<Inode>>> {
    wwarn!("rootfs_mkdir");
    let number = INODE_COUNT.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
    // 创建raminode
    let ram_inode = RamFsInode::new(mode, attr, number);
    RAM_FS.lock().insert(number, ram_inode.clone());

    // 根据ramfs的inode创建inode
    let sb_blk = dir.lock().super_blk.upgrade().unwrap().clone();
    // 创建inode
    let inode = create_tmp_inode_from_sb_blk(sb_blk)?;
    let mut inode_lock = inode.lock();
    // 根据raminode 设置inode的属性
    inode_lock.number = ram_inode.number;
    inode_lock.hard_links = ram_inode.hard_link;
    inode_lock.mode = ram_inode.mode;
    inode_lock.inode_ops = match ram_inode.mode {
        InodeMode { .. } => root_fs_inode_ops(),
    };
    // TODO 根据文件类型设置inode的操作
    inode_lock.file_ops = match ram_inode.mode {
        InodeMode { .. } => root_fs_file_ops(),
    };
    inode_lock.file_size = ram_inode.data.len();
    drop(inode_lock);
    Ok(inode)
}

/// 创建内存文件系统的目录并返回目录项
/// * dir: 父目录的inode
/// * dentry: 需要填充的目录项
/// * attr: 目录的属性
fn rootfs_mkdir(
    dir: Arc<Mutex<Inode>>,
    dentry: Arc<Mutex<DirEntry>>,
    attr: FileMode,
) -> StrResult<()> {
    wwarn!("rootfs_mkdir");
    let inode = rootfs_create_inode(dir, InodeMode::S_DIR, attr)?;
    dentry.lock().d_inode = inode;
    wwarn!("rootfs_mkdir end");
    Ok(())
}

/// 创建内存文件系统的文件并返回目录项
fn rootfs_create(
    dir: Arc<Mutex<Inode>>,
    dentry: Arc<Mutex<DirEntry>>,
    mode: FileMode,
) -> StrResult<()> {
    wwarn!("rootfs_create");
    let inode = rootfs_create_inode(dir, InodeMode::S_FILE, mode)?;
    dentry.lock().d_inode = inode;
    wwarn!("rootfs_create end");
    Ok(())
}

fn root_fs_read_file(file: Arc<Mutex<File>>, buf: &mut [u8], offset: u64) -> StrResult<usize> {
    let dentry = &mut file.lock().f_dentry;
    let inode = &mut dentry.lock().d_inode;
    // 获取inode的编号
    let number = inode.lock().number;
    let mut binding = RAM_FS.lock();
    let ram_inode = binding.get_mut(&number).unwrap();
    let read_len = core::cmp::min(
        buf.len(),
        ram_inode.data.len().saturating_sub(offset as usize),
    );
    unsafe {
        core::ptr::copy(
            ram_inode.data.as_ptr().add(offset as usize),
            buf.as_mut_ptr(),
            read_len,
        );
    }
    Ok(read_len)
}

fn root_fs_write_file(file: Arc<Mutex<File>>, buf: &[u8], offset: u64) -> StrResult<usize> {
    let dentry = &mut file.lock().f_dentry;
    let inode = &mut dentry.lock().d_inode;
    // 获取inode的编号
    let number = inode.lock().number;
    let mut binding = RAM_FS.lock();
    let ram_inode = binding.get_mut(&number).unwrap();
    if offset as usize + buf.len() > ram_inode.data.len() {
        ram_inode.data.resize(offset as usize + buf.len(), 0);
    }
    unsafe {
        core::ptr::copy(
            buf.as_ptr(),
            ram_inode.data.as_mut_ptr().add(offset as usize),
            buf.len(),
        );
    }
    Ok(buf.len())
}
