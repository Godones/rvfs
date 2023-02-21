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

use spin::Mutex;

static INODE_COUNT: AtomicUsize = AtomicUsize::new(0);

lazy_static! {
    static ref TMP_FS: Arc<Mutex<HashMap<usize, RamFsInode>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub const fn tmp_fs_type() -> FileSystemType {
    FileSystemType {
        name: "tmpfs",
        fs_flags: FileSystemAttr::empty(),
        super_blk_s: Vec::new(),
        get_super_blk: tmpfs_get_super_blk,
        kill_super_blk: ramfs_kill_super_blk,
    }
}

const TMPFS_DIR_INODE_OPS: InodeOps = {
    let mut ops = InodeOps::empty();
    ops.mkdir = tmpfs_mkdir;
    ops.create = tmpfs_create;
    ops.link = tmpfs_link;
    ops.unlink = tmpfs_unlink;
    ops.symlink = tmpfs_symlink;
    ops.rmdir = tmpfs_rmdir;
    ops
};

const TMPFS_FILE_INODE_OPS: InodeOps = {
    let ops = InodeOps::empty();
    ops
};

const TMPFS_SYMLINK_INODE_OPS: InodeOps = {
    let mut ops = InodeOps::empty();
    ops.readlink = tmpfs_readlink;
    ops.follow_link = tmpfs_follow_link;
    ops
};

const TMPFS_FILE_FILE_OPS: FileOps = {
    let mut ops = FileOps::empty();
    ops.read = tmpfs_read_file;
    ops.write = tmpfs_write_file;
    ops.open = |_| Ok(());
    ops
};

const TMPFS_SYMLINK_FILE_OPS: FileOps = {
    let ops = FileOps::empty();
    ops
};

const TMPFS_DIR_FILE_OPS: FileOps = {
    let mut ops = FileOps::empty();
    ops.readdir = tmpfs_readdir;
    ops
};

fn tmpfs_get_super_blk(
    fs_type: Arc<Mutex<FileSystemType>>,
    flags: MountFlags,
    dev_name: &str,
    data: Option<Box<dyn DataOps>>,
) -> StrResult<Arc<Mutex<SuperBlock>>> {
    wwarn!("tmpfs_get_super_blk");
    let sb_blk = ramfs_simple_super_blk(fs_type.clone(), flags, dev_name, data)?;
    assert_eq!(INODE_COUNT.load(Ordering::SeqCst), 0);
    let number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    let inode = ramfs_create_root_inode(
        TMP_FS.clone(),
        sb_blk.clone(),
        InodeMode::S_DIR,
        TMPFS_DIR_INODE_OPS,
        TMPFS_DIR_FILE_OPS,
        number,
    )?;
    // 根目录硬链接计数不用自增1
    inode.lock().hard_links -= 1;
    // 创建目录项
    let dentry = ramfs_create_root_dentry(None, inode)?;
    sb_blk.lock().root = dentry;
    // 将sb_blk插入到fs_type的链表中
    fs_type.lock().insert_super_blk(sb_blk.clone());
    wwarn!("tmpfs_get_super_blk end");
    Ok(sb_blk)
}

fn tmpfs_mkdir(
    dir: Arc<Mutex<Inode>>,
    dentry: Arc<Mutex<DirEntry>>,
    attr: FileMode,
) -> StrResult<()> {
    wwarn!("tmpfs_mkdir");
    let number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    ramfs_mkdir(
        TMP_FS.clone(),
        dir,
        dentry,
        attr,
        number,
        TMPFS_DIR_INODE_OPS,
        TMPFS_DIR_FILE_OPS,
    )?;
    wwarn!("tmpfs_mkdir end");
    Ok(())
}

fn tmpfs_create(
    dir: Arc<Mutex<Inode>>,
    dentry: Arc<Mutex<DirEntry>>,
    mode: FileMode,
) -> StrResult<()> {
    wwarn!("tmpfs_create");
    let number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    ramfs_create(
        TMP_FS.clone(),
        dir,
        dentry,
        mode,
        number,
        TMPFS_FILE_INODE_OPS,
        TMPFS_FILE_FILE_OPS,
    )?;
    wwarn!("tmpfs_create end");
    Ok(())
}

fn tmpfs_read_file(file: Arc<Mutex<File>>, buf: &mut [u8], offset: u64) -> StrResult<usize> {
    wwarn!("tmpfs_read_file");
    ramfs_read_file(TMP_FS.clone(), file, buf, offset)
}
fn tmpfs_write_file(file: Arc<Mutex<File>>, buf: &[u8], offset: u64) -> StrResult<usize> {
    wwarn!("tmpfs_write_file");
    ramfs_write_file(TMP_FS.clone(), file, buf, offset)
}

/// 创建硬链接
fn tmpfs_link(
    old_dentry: Arc<Mutex<DirEntry>>,
    dir: Arc<Mutex<Inode>>,
    new_dentry: Arc<Mutex<DirEntry>>,
) -> StrResult<()> {
    wwarn!("tmpfs_link");
    let _number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    ramfs_link(TMP_FS.clone(), old_dentry, dir, new_dentry)?;
    wwarn!("tmpfs_link end");
    Ok(())
}

/// 删除硬链接
fn tmpfs_unlink(dir: Arc<Mutex<Inode>>, dentry: Arc<Mutex<DirEntry>>) -> StrResult<()> {
    wwarn!("tmpfs_link");
    ramfs_unlink(TMP_FS.clone(), dir, dentry)?;
    wwarn!("tmpfs_link end");
    Ok(())
}

/// create a symbolic link
fn tmpfs_symlink(
    dir: Arc<Mutex<Inode>>,
    dentry: Arc<Mutex<DirEntry>>,
    target: &str,
) -> StrResult<()> {
    wwarn!("tmpfs_symlink");
    let number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    ramfs_symlink(
        TMP_FS.clone(),
        FileMode::FMODE_READ,
        number,
        dir,
        dentry,
        target,
        TMPFS_SYMLINK_INODE_OPS,
        TMPFS_SYMLINK_FILE_OPS,
    )?;
    wwarn!("tmpfs_symlink end");
    Ok(())
}

/// read the target of a symbolic link
fn tmpfs_readlink(dentry: Arc<Mutex<DirEntry>>, buf: &mut [u8]) -> StrResult<usize> {
    let inode = dentry.lock().d_inode.clone();
    let inode = inode.lock();
    let number = inode.number;
    let bind = TMP_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    ramfs_read_link(ram_inode, buf)
}

/// follow a symbolic link
fn tmpfs_follow_link(dentry: Arc<Mutex<DirEntry>>, lookup_data: &mut LookUpData) -> StrResult<()> {
    let inode = dentry.lock().d_inode.clone();
    let inode = inode.lock();
    let number = inode.number;
    let bind = TMP_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    ramfs_follow_link(&ram_inode, lookup_data)
}

/// read the contents of a directory
fn tmpfs_readdir(file: Arc<Mutex<File>>) -> StrResult<DirContext> {
    wwarn!("rootfs_readdir");
    let inode = file.lock().f_dentry.lock().d_inode.clone();
    let inode = inode.lock();
    let number = inode.number;
    let bind = TMP_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    let data = ram_inode.data.clone();
    let dir_context = DirContext::new(data);
    wwarn!("rootfs_readdir end");
    Ok(dir_context)
}

fn tmpfs_rmdir(dir: Arc<Mutex<Inode>>, dentry: Arc<Mutex<DirEntry>>) -> StrResult<()> {
    wwarn!("rootfs_rmdir");
    let inode = dir.lock();
    let number = inode.number;
    let mut bind = TMP_FS.lock();
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
