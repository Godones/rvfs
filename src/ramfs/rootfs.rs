use crate::dentry::DirEntry;
use crate::inode::{Inode, InodeMode, InodeOps};
use crate::ramfs::{
    ramfs_create, ramfs_create_root_dentry, ramfs_create_root_inode, ramfs_follow_link,
    ramfs_kill_super_blk, ramfs_mkdir, ramfs_read_file, ramfs_read_link, ramfs_simple_super_blk,
    ramfs_symlink, ramfs_write_file,
};
use crate::ramfs::{ramfs_link, ramfs_unlink, RamFsInode};
use crate::superblock::SuperBlock;
use crate::{
    wwarn, DataOps, DirContext, File, FileMode, FileOps, FileSystemAttr, FileSystemType,
    LookUpData, MountFlags, StrResult,
};
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicUsize, Ordering};
use hashbrown::HashMap;
use kmpsearch::Haystack;
use lazy_static::lazy_static;
use log::error;

use spin::Mutex;

static INODE_COUNT: AtomicUsize = AtomicUsize::new(0);

lazy_static! {
    static ref ROOT_FS: Arc<Mutex<HashMap<usize, RamFsInode>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub const fn root_fs_type() -> FileSystemType {
    FileSystemType {
        name: "rootfs",
        fs_flags: FileSystemAttr::empty(),
        super_blk_s: Vec::new(),
        get_super_blk: rootfs_get_super_blk,
        kill_super_blk: ramfs_kill_super_blk,
    }
}

const ROOTFS_DIR_INODE_OPS: InodeOps = {
    let mut ops = InodeOps::empty();
    ops.mkdir = rootfs_mkdir;
    ops.create = rootfs_create;
    ops.link = rootfs_link;
    ops.unlink = rootfs_unlink;
    ops.symlink = rootfs_symlink;
    ops.rmdir = rootfs_rmdir;
    ops
};

const ROOTFS_FILE_INODE_OPS: InodeOps = {
    let ops = InodeOps::empty();
    ops
};

const ROOTFS_SYMLINK_INODE_OPS: InodeOps = {
    let mut ops = InodeOps::empty();
    ops.readlink = rootfs_readlink;
    ops.follow_link = rootfs_follow_link;
    ops
};

const ROOTFS_FILE_FILE_OPS: FileOps = {
    let mut ops = FileOps::empty();
    ops.read = rootfs_read_file;
    ops.write = rootfs_write_file;
    ops.open = |_| Ok(());
    ops
};

const ROOTFS_SYMLINK_FILE_OPS: FileOps = {
    let mut ops = FileOps::empty();
    ops.open = |_| Ok(());
    ops
};

const ROOTFS_DIR_FILE_OPS: FileOps = {
    let mut ops = FileOps::empty();
    ops.readdir = rootfs_readdir;
    ops.open = |_| Ok(());
    ops
};

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
        ROOTFS_DIR_INODE_OPS,
        ROOTFS_DIR_FILE_OPS,
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
        ROOTFS_DIR_INODE_OPS,
        ROOTFS_DIR_FILE_OPS,
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
    error!("***** {}", dentry.lock().d_name);
    let number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    ramfs_create(
        ROOT_FS.clone(),
        dir,
        dentry,
        mode,
        number,
        ROOTFS_FILE_INODE_OPS,
        ROOTFS_FILE_FILE_OPS,
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

/// create a hard link to the inode
fn rootfs_link(
    old_dentry: Arc<Mutex<DirEntry>>,
    dir: Arc<Mutex<Inode>>,
    new_dentry: Arc<Mutex<DirEntry>>,
) -> StrResult<()> {
    wwarn!("rootfs_link");
    let _number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    ramfs_link(ROOT_FS.clone(), old_dentry, dir, new_dentry)?;
    wwarn!("rootfs_link end");
    Ok(())
}

/// decrease the hard link count of the inode
fn rootfs_unlink(dir: Arc<Mutex<Inode>>, dentry: Arc<Mutex<DirEntry>>) -> StrResult<()> {
    wwarn!("rootfs_unlink");
    ramfs_unlink(ROOT_FS.clone(), dir, dentry)?;
    wwarn!("rootfs_unlink end");
    Ok(())
}

/// create a symbolic link
fn rootfs_symlink(
    dir: Arc<Mutex<Inode>>,
    dentry: Arc<Mutex<DirEntry>>,
    target: &str,
) -> StrResult<()> {
    wwarn!("rootfs_symlink");
    let number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    ramfs_symlink(
        ROOT_FS.clone(),
        FileMode::FMODE_READ,
        number,
        dir,
        dentry,
        target,
        ROOTFS_SYMLINK_INODE_OPS,
        ROOTFS_SYMLINK_FILE_OPS,
    )?;
    wwarn!("rootfs_symlink end");
    Ok(())
}

/// read the target of a symbolic link
fn rootfs_readlink(dentry: Arc<Mutex<DirEntry>>, buf: &mut [u8]) -> StrResult<usize> {
    let inode = dentry.lock().d_inode.clone();
    let inode = inode.lock();
    let number = inode.number;
    let bind = ROOT_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    ramfs_read_link(ram_inode, buf)
}

/// follow a symbolic link
fn rootfs_follow_link(dentry: Arc<Mutex<DirEntry>>, lookup_data: &mut LookUpData) -> StrResult<()> {
    let inode = dentry.lock().d_inode.clone();
    let inode = inode.lock();
    let number = inode.number;
    let bind = ROOT_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    ramfs_follow_link(&ram_inode, lookup_data)
}

/// read the contents of a directory
fn rootfs_readdir(file: Arc<Mutex<File>>) -> StrResult<DirContext> {
    wwarn!("rootfs_readdir");
    let inode = file.lock().f_dentry.lock().d_inode.clone();
    let inode = inode.lock();
    let number = inode.number;
    let bind = ROOT_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    let data = ram_inode.data.clone();
    let dir_context = DirContext::new(data);
    wwarn!("rootfs_readdir end");
    Ok(dir_context)
}

fn rootfs_rmdir(dir: Arc<Mutex<Inode>>, dentry: Arc<Mutex<DirEntry>>) -> StrResult<()> {
    wwarn!("rootfs_rmdir");
    let inode = dir.lock();
    let number = inode.number;
    let mut bind = ROOT_FS.lock();
    let ram_inode = bind.get_mut(&number).unwrap();
    // check if the dir is empty
    assert!(!ram_inode.data.is_empty());
    // delete the sub dir
    // find name from data
    let name = dentry.lock().d_name.clone();
    let index = ram_inode
        .data
        .as_slice()
        .last_indexof_needle(name.as_bytes())
        .unwrap();
    ram_inode
        .data
        .splice(index..index + name.len() + 1, "".as_bytes().iter().cloned());
    let sub_dir = dentry.lock().d_inode.clone();
    let sub_dir = sub_dir.lock();
    let sub_number = sub_dir.number;
    // delete the sub dir
    bind.remove(&sub_number);
    wwarn!("rootfs_rmdir end");
    Ok(())
}
