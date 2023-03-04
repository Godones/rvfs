pub mod rootfs;
pub mod tmpfs;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use hashbrown::HashMap;
use log::info;
use spin::Mutex;
use crate::file::{File, FileMode, FileOps};
use crate::inode::{create_tmp_inode_from_sb_blk, Inode, InodeMode, InodeOps, simple_statfs};
use crate::mount::MountFlags;
use crate::{StrResult, wwarn};
use crate::dentry::{DirEntry, LookUpData};
use crate::superblock::{DataOps, FileSystemType, find_super_blk, SuperBlock, SuperBlockInner, SuperBlockOps};

#[derive(Clone)]
pub struct RamFsInode {
    // inode number
    number: usize,
    // may be for normal file
    data: Vec<u8>,
    // may be for dir to store sub_file
    dentries: HashMap<String, usize>,
    // type
    mode: InodeMode,
    hard_links: u32,
    // write/read mod
    attr: FileMode,
    // extra attribute
    ex_attr: HashMap<String, Vec<u8>>,
}

impl RamFsInode {
    pub fn new(mode: InodeMode, attr: FileMode, number: usize) -> Self {
        let h_link = if mode == InodeMode::S_DIR { 2 } else { 1 };
        Self {
            number,
            data: Vec::new(),
            dentries: HashMap::new(),
            mode,
            hard_links: h_link,
            attr,
            ex_attr: HashMap::new(),
        }
    }
}
const RAMFS_SB_OPS: SuperBlockOps = {
    let mut sb_ops = SuperBlockOps::empty();
    sb_ops.stat_fs = simple_statfs;
    sb_ops
};

const RAM_BLOCK_SIZE: u32 = 4096;
const RAM_FILE_MAX_SIZE: usize = 4096;
const RAM_MAGIC: u32 = 0x12345678;

/// 创建一个内存文件系统的超级块
fn create_simple_ram_super_blk(
    fs_type: Arc<FileSystemType>,
    flags: MountFlags,
    dev_name: &str,
    data: Option<Box<dyn DataOps>>,
) -> StrResult<Arc<SuperBlock>> {
    let sb_blk = SuperBlock {
        dev_desc: 0,
        device: None,
        block_size: RAM_BLOCK_SIZE,
        dirty_flag: false,
        file_max_bytes: RAM_FILE_MAX_SIZE,
        mount_flag: flags,
        magic: RAM_MAGIC,
        file_system_type: Arc::downgrade(&fs_type),
        super_block_ops: RAMFS_SB_OPS,
        inner: Mutex::new(SuperBlockInner::empty()),
        blk_dev_name: dev_name.to_string(),
        data,
    };
    let sb_blk = Arc::new(sb_blk);
    Ok(sb_blk)
}

fn ramfs_simple_super_blk(
    fs_type: Arc<FileSystemType>,
    flags: MountFlags,
    dev_name: &str,
    data: Option<Box<dyn DataOps>>,
) -> StrResult<Arc<SuperBlock>> {
    wwarn!("ramfs_simple_super_blk");
    let find_sb_blk = find_super_blk(fs_type.clone(), None);
    let sb_blk = match find_sb_blk {
        // 找到了旧超级快
        Ok(sb_blk) => sb_blk,
        Err(_) => {
            // 没有找到旧超级快需要重新分配
            info!("create new super block for ramfs");

            create_simple_ram_super_blk(fs_type, flags, dev_name, data)?
        }
    };
    wwarn!("ramfs_simple_super_blk end");
    Ok(sb_blk)
}

fn ramfs_kill_super_blk(_super_blk: Arc<SuperBlock>) {}

/// 创建内存文件系统的根inode
fn ramfs_create_root_inode(
    fs: Arc<Mutex<HashMap<usize, RamFsInode>>>,
    sb_blk: Arc<SuperBlock>,
    mode: InodeMode,
    inode_ops: InodeOps,
    file_ops: FileOps,
    number: usize,
) -> StrResult<Arc<Inode>> {
    let inode = create_tmp_inode_from_sb_blk(sb_blk, 0, mode, 0, inode_ops, file_ops, None)?;
    // 设置inode的编号
    assert_eq!(number, 0);
    inode.access_inner().hard_links = 0;
    // TODO 设置uid/gid
    // 插入根inode
    let mut ram_inode = RamFsInode::new(mode, FileMode::FMODE_WRITE, 0);
    ram_inode.hard_links = 0;
    fs.lock().insert(0, ram_inode);
    Ok(inode)
}

fn ramfs_create_root_dentry(
    parent: Option<Arc<DirEntry>>,
    inode: Arc<Inode>,
) -> StrResult<Arc<DirEntry>> {
    let dentry = DirEntry::empty();
    assert!(parent.is_none());
    dentry.access_inner().d_inode = inode;
    dentry.access_inner().d_name = "/".to_string();
    Ok(Arc::new(dentry))
}

fn ramfs_create_inode(
    fs: Arc<Mutex<HashMap<usize, RamFsInode>>>,
    dir: Arc<Inode>,
    mode: InodeMode,
    attr: FileMode,
    number: usize,
    inode_ops: InodeOps,
    file_ops: FileOps,
    name: String,
) -> StrResult<Arc<Inode>> {
    wwarn!("ramfs_create_inode");
    // 创建raminode
    let ram_inode = RamFsInode::new(mode, attr, number);
    fs.lock().insert(number, ram_inode.clone());

    // 根据ramfs的inode创建inode
    let sb_blk = dir.super_blk.upgrade().unwrap();
    // 创建inode根据raminode 设置inode的属性
    let inode = create_tmp_inode_from_sb_blk(
        sb_blk,
        ram_inode.number,
        ram_inode.mode,
        0,
        inode_ops,
        file_ops,
        None,
    )?;
    inode.access_inner().hard_links = ram_inode.hard_links;
    inode.access_inner().file_size = ram_inode.data.len();
    // 在父目录中写入目录项
    let dir_number = dir.number;
    let mut bind = fs.lock();
    let ram_inode = bind.get_mut(&dir_number).unwrap();
    let old = ram_inode.dentries.insert(name, number);
    assert!(old.is_none());
    dir.access_inner().file_size = ram_inode.dentries.len();
    drop(dir);
    wwarn!("ramfs_create_inode end");
    Ok(inode)
}

/// 创建内存文件系统的目录并返回目录项
/// * dir: 父目录的inode
/// * dentry: 需要填充的目录项
/// * attr: 目录的属性
fn ramfs_mkdir(
    fs: Arc<Mutex<HashMap<usize, RamFsInode>>>,
    dir: Arc<Inode>,
    dentry: Arc<DirEntry>,
    attr: FileMode,
    number: usize,
    inode_ops: InodeOps,
    file_ops: FileOps,
) -> StrResult<()> {
    wwarn!("ramfs_mkdir");
    let inode = ramfs_create_inode(
        fs,
        dir,
        InodeMode::S_DIR,
        attr,
        number,
        inode_ops,
        file_ops,
        dentry.access_inner().d_name.clone(),
    )?;
    dentry.access_inner().d_inode = inode;
    wwarn!("ramfs_mkdir end");
    Ok(())
}

/// 创建内存文件系统的文件并返回目录项
fn ramfs_create(
    fs: Arc<Mutex<HashMap<usize, RamFsInode>>>,
    dir: Arc<Inode>,
    dentry: Arc<DirEntry>,
    mode: FileMode,
    number: usize,
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
        dentry.access_inner().d_name.clone(),
    )?;
    dentry.access_inner().d_inode = inode;
    wwarn!("rootfs_create end");
    Ok(())
}

fn ramfs_read_file(
    fs: Arc<Mutex<HashMap<usize, RamFsInode>>>,
    file: Arc<File>,
    buf: &mut [u8],
    offset: u64,
) -> StrResult<usize> {
    let dentry = &file.f_dentry;
    let inode = &dentry.access_inner().d_inode;
    // 获取inode的编号
    let number = inode.number;
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
    fs: Arc<Mutex<HashMap<usize, RamFsInode>>>,
    file: Arc<File>,
    buf: &[u8],
    offset: u64,
) -> StrResult<usize> {
    wwarn!("ramfs_write_file");
    let dentry = &file.f_dentry;
    let inode = &dentry.access_inner().d_inode;
    // 获取inode的编号
    let number = inode.number;
    info!("number: {}", number);
    let mut binding = fs.lock();
    let ram_inode = binding.get_mut(&number);
    if ram_inode.is_none() {
        return Err("ramfs_write_file: ram_inode is none");
    }
    let ram_inode = ram_inode.unwrap();
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
    fs: Arc<Mutex<HashMap<usize, RamFsInode>>>,
    old_dentry: Arc<DirEntry>,
    dir: Arc<Inode>,
    new_dentry: Arc<DirEntry>,
) -> StrResult<()> {
    wwarn!("ramfs_link");
    let old_inode = old_dentry.access_inner().d_inode.clone();
    old_inode.access_inner().hard_links += 1;
    let inode_number = old_inode.number;
    let mut binding = fs.lock();
    let ram_inode = binding.get_mut(&inode_number).unwrap();
    ram_inode.hard_links += 1;

    new_dentry.access_inner().d_inode = old_inode;
    let dir_lock = dir;
    assert_eq!(dir_lock.mode, InodeMode::S_DIR);

    // create a new inode
    let number = dir_lock.number;
    let ram_inode = binding.get_mut(&number).unwrap();
    let name = new_dentry.access_inner().d_name.clone();
    ram_inode.dentries.insert(name, inode_number);
    dir_lock.access_inner().file_size = ram_inode.dentries.len();
    wwarn!("ramfs_link end");
    Ok(())
}

fn ramfs_unlink(
    fs: Arc<Mutex<HashMap<usize, RamFsInode>>>,
    dir: Arc<Inode>,
    dentry: Arc<DirEntry>,
) -> StrResult<()> {
    wwarn!("ramfs_unlink");
    assert_eq!(dir.mode, InodeMode::S_DIR);
    let name = dentry.access_inner().d_name.clone();

    let inode = dentry.access_inner().d_inode.clone();
    let inode_lock = inode;
    inode_lock.access_inner().hard_links -= 1;

    let number = inode_lock.number;
    let mut binding = fs.lock();
    let ram_inode = binding.get_mut(&number).unwrap();
    ram_inode.hard_links -= 1;

    if inode_lock.access_inner().hard_links == 0 {
        assert_eq!(ram_inode.hard_links, 0);
        binding.remove(&number);
    }

    // delete dentry and update dir size
    let dir_number = dir.number;
    let dir_ram_inode = binding.get_mut(&dir_number).unwrap();
    dir_ram_inode.dentries.remove(&name);
    dir.access_inner().file_size = dir_ram_inode.dentries.len();

    wwarn!("ramfs_unlink end");
    Ok(())
}

fn ramfs_symlink(
    fs: Arc<Mutex<HashMap<usize, RamFsInode>>>,
    mode: FileMode,
    number: usize,
    dir: Arc<Inode>,
    dentry: Arc<DirEntry>,
    target: &str,
    inode_ops: InodeOps,
    file_ops: FileOps,
) -> StrResult<()> {
    wwarn!("ramfs_symlink");
    let inode = ramfs_create_inode(
        fs.clone(),
        dir,
        InodeMode::S_SYMLINK,
        mode,
        number,
        inode_ops,
        file_ops,
        dentry.access_inner().d_name.clone(),
    )?;
    let mut fs_lk = fs.lock();
    let ram_inode = fs_lk.get_mut(&number).unwrap();
    ram_inode.data.extend_from_slice(target.as_bytes());
    inode.access_inner().file_size = target.len();
    dentry.access_inner().d_inode = inode;
    wwarn!("ramfs_symlink end");
    Ok(())
}

fn ramfs_read_link(ram_inode: &RamFsInode, buf: &mut [u8]) -> StrResult<usize> {
    wwarn!("ramfs_read_link");
    let read_len = core::cmp::min(buf.len(), ram_inode.data.len());
    unsafe {
        core::ptr::copy(ram_inode.data.as_ptr(), buf.as_mut_ptr(), read_len);
    }
    wwarn!("ramfs_read_link end");
    Ok(read_len)
}

/// TODO
fn ramfs_follow_link(ram_inode: &RamFsInode, lookup_data: &mut LookUpData) -> StrResult<()> {
    wwarn!("ramfs_follow_link");
    let target_name = ram_inode.data.clone();
    let name = String::from_utf8(target_name).unwrap();
    lookup_data.symlink_names.push(name);
    wwarn!("ramfs_follow_link end");
    Ok(())
}
