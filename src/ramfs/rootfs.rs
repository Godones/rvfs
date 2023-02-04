use crate::dentrry::DirEntry;
use crate::inode::{Inode, InodeMode, InodeOps};
use crate::ramfs::RamFsInode;
use crate::ramfs::{
    ramfs_create, ramfs_create_root_dentry, ramfs_create_root_inode, ramfs_kill_super_blk,
    ramfs_mkdir, ramfs_read_file, ramfs_simple_super_blk, ramfs_write_file,
};
use crate::superblock::SuperBlock;
use crate::{
    wwarn, DataOps, File, FileMode, FileOps, FileSystemAttr, FileSystemType, MountFlags, StrResult,
};
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use logger::warn;
use spin::Mutex;

static INODE_COUNT: AtomicU32 = AtomicU32::new(0);

lazy_static! {
    static ref ROOT_FS: Arc<Mutex<HashMap<u32, RamFsInode>>> = Arc::new(Mutex::new(HashMap::new()));
}

pub const fn root_fs_type() -> FileSystemType {
    let fs_type = FileSystemType {
        name: "rootfs",
        fs_flags: FileSystemAttr::empty(),
        super_blk_s: Vec::new(),
        get_super_blk: rootfs_get_super_blk,
        kill_super_blk: ramfs_kill_super_blk,
    };
    fs_type
}

const fn root_fs_inode_ops() -> InodeOps {
    let mut ops = InodeOps::empty();
    ops.mkdir = rootfs_mkdir;
    ops.create = rootfs_create;
    ops
}

const fn root_fs_file_ops() -> FileOps {
    let mut ops = FileOps::empty();
    ops.read = rootfs_read_file;
    ops.write = rootfs_write_file;
    ops.open = |_| Ok(());
    ops
}

fn rootfs_get_super_blk(
    fs_type: Arc<Mutex<FileSystemType>>,
    flags: MountFlags,
    dev_name: &str,
    data: Option<Box<dyn DataOps>>,
) -> StrResult<Arc<Mutex<SuperBlock>>> {
    wwarn!("rootfs_get_super_blk");
    let sb_blk = ramfs_simple_super_blk(fs_type.clone(), flags, dev_name, data)?;
    assert_eq!(INODE_COUNT.load(Ordering::SeqCst), 0);
    let number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    let inode = ramfs_create_root_inode(
        ROOT_FS.clone(),
        sb_blk.clone(),
        InodeMode::S_DIR,
        root_fs_inode_ops(),
        root_fs_file_ops(),
        number,
    )?;
    // 根目录硬链接计数不用自增1
    inode.lock().hard_links -= 1;
    // 创建目录项
    let dentry = ramfs_create_root_dentry(None, inode)?;
    sb_blk.lock().root = dentry;
    // 将sb_blk插入到fs_type的链表中
    fs_type.lock().insert_super_blk(sb_blk.clone());
    wwarn!("rootfs_get_super_blk end");
    Ok(sb_blk)
}

fn rootfs_mkdir(
    dir: Arc<Mutex<Inode>>,
    dentry: Arc<Mutex<DirEntry>>,
    attr: FileMode,
) -> StrResult<()> {
    wwarn!("rootfs_mkdir");
    let number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    ramfs_mkdir(
        ROOT_FS.clone(),
        dir,
        dentry,
        attr,
        number,
        root_fs_inode_ops(),
        root_fs_file_ops(),
    )?;
    wwarn!("rootfs_mkdir end");
    Ok(())
}

fn rootfs_create(
    dir: Arc<Mutex<Inode>>,
    dentry: Arc<Mutex<DirEntry>>,
    mode: FileMode,
) -> StrResult<()> {
    wwarn!("rootfs_create");
    let number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    ramfs_create(
        ROOT_FS.clone(),
        dir,
        dentry,
        mode,
        number,
        root_fs_inode_ops(),
        root_fs_file_ops(),
    )?;
    wwarn!("rootfs_create end");
    Ok(())
}

fn rootfs_read_file(file: Arc<Mutex<File>>, buf: &mut [u8], offset: u64) -> StrResult<usize> {
    wwarn!("rootfs_read_file");
    ramfs_read_file(ROOT_FS.clone(), file, buf, offset)
}
fn rootfs_write_file(file: Arc<Mutex<File>>, buf: &[u8], offset: u64) -> StrResult<usize> {
    wwarn!("rootfs_write_file");
    ramfs_write_file(ROOT_FS.clone(), file, buf, offset)
}
