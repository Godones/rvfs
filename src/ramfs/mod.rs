pub mod rootfs;
pub mod tmpfs;

use crate::dentrry::DirEntry;
use crate::file::FileOps;
use crate::inode::{create_tmp_inode_from_sb_blk, simple_statfs, Inode, InodeMode, InodeOps};
use crate::superblock::{FileSystemType, SuperBlock};
use crate::{find_super_blk, wwarn, DataOps, File, FileMode, MountFlags, StrResult, SuperBlockOps};
use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use hashbrown::HashMap;

use logger::{info, warn};
use spin::Mutex;

#[derive(Clone)]
pub struct RamFsInode {
    // 节点号
    number: u32,
    data: Vec<u8>,
    // 类型
    mode: InodeMode,
    hard_links: u32,
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
            hard_links: h_link,
            attr,
        }
    }
}

const fn root_fs_sb_blk_ops() -> SuperBlockOps {
    SuperBlockOps {
        alloc_inode: |_| Err("Not support"),
        write_inode: |_, _| {},
        dirty_inode: |_| {},
        delete_inode: |_| {},
        write_super: |_| {},
        sync_fs: |_| {},
        freeze_fs: |_| {},
        unfreeze_fs: |_| {},
        stat_fs: simple_statfs,
    }
}

const RAM_BLOCK_SIZE: u32 = 4096;
const RAM_FILE_MAX_SIZE: usize = 4096;
const RAM_MAGIC: u32 = 0x12345678;

/// 创建一个内存文件系统的超级块
fn create_simple_ram_super_blk(
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

fn ramfs_simple_super_blk(
    fs_type: Arc<Mutex<FileSystemType>>,
    flags: MountFlags,
    dev_name: &str,
    data: Option<Box<dyn DataOps>>,
) -> StrResult<Arc<Mutex<SuperBlock>>> {
    wwarn!("ramfs_simple_super_blk");
    let find_sb_blk = find_super_blk(fs_type.clone(), None);
    let sb_blk = match find_sb_blk {
        // 找到了旧超级快
        Ok(sb_blk) => sb_blk,
        Err(_) => {
            // 没有找到旧超级快需要重新分配
            info!("create new super block for ramfs");
            let sb_blk = create_simple_ram_super_blk(fs_type.clone(), flags, dev_name, data)?;
            sb_blk
        }
    };
    wwarn!("ramfs_simple_super_blk end");
    Ok(sb_blk)
}

fn ramfs_kill_super_blk(_super_blk: Arc<Mutex<SuperBlock>>) {}

/// 创建内存文件系统的根inode
fn ramfs_create_root_inode(
    fs: Arc<Mutex<HashMap<u32, RamFsInode>>>,
    sb_blk: Arc<Mutex<SuperBlock>>,
    mode: InodeMode,
    inode_ops: InodeOps,
    file_ops: FileOps,
    number: u32,
) -> StrResult<Arc<Mutex<Inode>>> {
    let inode = create_tmp_inode_from_sb_blk(sb_blk)?;
    let mut inode_lk = inode.lock();
    inode_lk.mode = mode;
    inode_lk.blk_count = 0;
    // 设置inode的编号
    assert_eq!(number, 0);
    inode_lk.number = number;
    // TODO 设置uid/gid
    match mode {
        InodeMode::S_DIR => {
            inode_lk.inode_ops = inode_ops;
            inode_lk.file_ops = file_ops;
            inode_lk.hard_links += 1
        }
        _ => panic!("root inode must be dir"),
    }
    drop(inode_lk);
    // 插入根inode
    let mut ram_inode = RamFsInode::new(mode, FileMode::FMODE_WRITE, 0);
    ram_inode.hard_links -= 1;
    fs.lock().insert(0, ram_inode);
    Ok(inode)
}

fn ramfs_create_root_dentry(
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

fn ramfs_create_inode(
    fs: Arc<Mutex<HashMap<u32, RamFsInode>>>,
    dir: Arc<Mutex<Inode>>,
    mode: InodeMode,
    attr: FileMode,
    number: u32,
    inode_ops: InodeOps,
    file_ops: FileOps,
) -> StrResult<Arc<Mutex<Inode>>> {
    wwarn!("ramfs_create_inode");
    // 创建raminode
    let ram_inode = RamFsInode::new(mode, attr, number);
    fs.lock().insert(number, ram_inode.clone());

    // 根据ramfs的inode创建inode
    let sb_blk = dir.lock().super_blk.upgrade().unwrap().clone();
    // 创建inode
    let inode = create_tmp_inode_from_sb_blk(sb_blk)?;
    let mut inode_lock = inode.lock();
    // 根据raminode 设置inode的属性
    inode_lock.number = ram_inode.number;
    inode_lock.hard_links = ram_inode.hard_links;
    inode_lock.mode = ram_inode.mode;
    inode_lock.inode_ops = match ram_inode.mode {
        InodeMode { .. } => inode_ops,
    };
    // TODO 根据文件类型设置inode的操作
    inode_lock.file_ops = match ram_inode.mode {
        InodeMode { .. } => file_ops,
    };
    inode_lock.file_size = ram_inode.data.len();
    drop(inode_lock);
    wwarn!("ramfs_create_inode end");
    Ok(inode)
}

/// 创建内存文件系统的目录并返回目录项
/// * dir: 父目录的inode
/// * dentry: 需要填充的目录项
/// * attr: 目录的属性
fn ramfs_mkdir(
    fs: Arc<Mutex<HashMap<u32, RamFsInode>>>,
    dir: Arc<Mutex<Inode>>,
    dentry: Arc<Mutex<DirEntry>>,
    attr: FileMode,
    number: u32,
    inode_ops: InodeOps,
    file_ops: FileOps,
) -> StrResult<()> {
    wwarn!("ramfs_mkdir");
    let inode = ramfs_create_inode(fs, dir, InodeMode::S_DIR, attr, number, inode_ops, file_ops)?;
    dentry.lock().d_inode = inode;
    wwarn!("ramfs_mkdir end");
    Ok(())
}

/// 创建内存文件系统的文件并返回目录项
fn ramfs_create(
    fs: Arc<Mutex<HashMap<u32, RamFsInode>>>,
    dir: Arc<Mutex<Inode>>,
    dentry: Arc<Mutex<DirEntry>>,
    mode: FileMode,
    number: u32,
    inode_ops: InodeOps,
    file_ops: FileOps,
) -> StrResult<()> {
    wwarn!("rootfs_create");
    let inode = ramfs_create_inode(
        fs,
        dir,
        InodeMode::S_FILE,
        mode,
        number,
        inode_ops,
        file_ops,
    )?;
    dentry.lock().d_inode = inode;
    wwarn!("rootfs_create end");
    Ok(())
}

fn ramfs_read_file(
    fs: Arc<Mutex<HashMap<u32, RamFsInode>>>,
    file: Arc<Mutex<File>>,
    buf: &mut [u8],
    offset: u64,
) -> StrResult<usize> {
    let dentry = &mut file.lock().f_dentry;
    let inode = &mut dentry.lock().d_inode;
    // 获取inode的编号
    let number = inode.lock().number;
    let mut binding = fs.lock();
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

fn ramfs_write_file(
    fs: Arc<Mutex<HashMap<u32, RamFsInode>>>,
    file: Arc<Mutex<File>>,
    buf: &[u8],
    offset: u64,
) -> StrResult<usize> {
    let dentry = &mut file.lock().f_dentry;
    let inode = &mut dentry.lock().d_inode;
    // 获取inode的编号
    let number = inode.lock().number;
    let mut binding = fs.lock();
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

fn ramfs_link(
    fs: Arc<Mutex<HashMap<u32, RamFsInode>>>,
    old_dentry: Arc<Mutex<DirEntry>>,
    dir: Arc<Mutex<Inode>>,
    new_dentry: Arc<Mutex<DirEntry>>,
) -> StrResult<()> {
    wwarn!("ramfs_link");
    let old_inode = old_dentry.lock().d_inode.clone();
    let mut old_inode_lock = old_inode.lock();
    old_inode_lock.hard_links += 1;
    let inode_number = old_inode_lock.number;
    let mut binding = fs.lock();
    let ram_inode = binding.get_mut(&inode_number).unwrap();
    ram_inode.hard_links += 1;

    drop(old_inode_lock);
    new_dentry.lock().d_inode = old_inode;
    let dir_lock = dir.lock();
    assert_eq!(dir_lock.mode, InodeMode::S_DIR);
    // TODO dir目录下需要增加一个(磁盘)目录项
    wwarn!("ramfs_link end");
    Ok(())
}

fn ramfs_unlink(
    fs: Arc<Mutex<HashMap<u32, RamFsInode>>>,
    dir: Arc<Mutex<Inode>>,
    dentry: Arc<Mutex<DirEntry>>,
) -> StrResult<()> {
    wwarn!("ramfs_unlink");
    let dir_lock = dir.lock();
    assert_eq!(dir_lock.mode, InodeMode::S_DIR);
    let _name = dentry.lock().d_name.clone();
    // TODO dir目录下需要删除一个(磁盘)目录项
    let inode = dentry.lock().d_inode.clone();
    let mut inode_lock = inode.lock();
    inode_lock.hard_links -= 1;
    if inode_lock.hard_links == 0 {
        let number = inode_lock.number;
        let mut binding = fs.lock();
        let ram_inode = binding.get_mut(&number).unwrap();
        ram_inode.hard_links -= 1;
        assert_eq!(ram_inode.hard_links, 0);
        binding.remove(&number);
    }
    wwarn!("ramfs_unlink end");
    Ok(())
}
