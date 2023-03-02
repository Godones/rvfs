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
    FileSystemTypeInner, InodeFlags, LookUpData, MountFlags, StrResult,
};
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cmp::min;

use core::sync::atomic::{AtomicUsize, Ordering};
use hashbrown::HashMap;

use lazy_static::lazy_static;
use log::{error, info};

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
        get_super_blk: rootfs_get_super_blk,
        kill_super_blk: ramfs_kill_super_blk,
        inner: Mutex::new(FileSystemTypeInner {
            super_blk_s: Vec::new(),
        }),
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
    ops.get_attr = rootfs_get_attr;
    ops.set_attr = rootfs_set_attr;
    ops.remove_attr = rootfs_remove_attr;
    ops.list_attr = rootfs_list_attr;
    ops.rename = rootfs_rename;
    ops
};

const ROOTFS_FILE_INODE_OPS: InodeOps = {
    let mut ops = InodeOps::empty();
    ops.get_attr = rootfs_get_attr;
    ops.set_attr = rootfs_set_attr;
    ops.remove_attr = rootfs_remove_attr;
    ops.list_attr = rootfs_list_attr;
    ops.truncate = rootfs_truncate;
    ops
};

const ROOTFS_SYMLINK_INODE_OPS: InodeOps = {
    let mut ops = InodeOps::empty();
    ops.readlink = rootfs_readlink;
    ops.follow_link = rootfs_follow_link;
    ops.get_attr = rootfs_get_attr;
    ops.set_attr = rootfs_set_attr;
    ops.remove_attr = rootfs_remove_attr;
    ops.list_attr = rootfs_list_attr;
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
    fs_type: Arc<FileSystemType>,
    flags: MountFlags,
    dev_name: &str,
    data: Option<Box<dyn DataOps>>,
) -> StrResult<Arc<SuperBlock>> {
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
    sb_blk.update_root(dentry);
    // 将sb_blk插入到fs_type的链表中
    fs_type.insert_super_blk(sb_blk.clone());
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
    let len = ramfs_read_file(ROOT_FS.clone(), file, buf, offset)?;
    wwarn!("rootfs_read_file end");
    Ok(len)
}
fn rootfs_write_file(file: Arc<Mutex<File>>, buf: &[u8], offset: u64) -> StrResult<usize> {
    wwarn!("rootfs_write_file");
    let len = ramfs_write_file(ROOT_FS.clone(), file, buf, offset)?;
    wwarn!("rootfs_write_file end");
    Ok(len)
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
    ramfs_follow_link(ram_inode, lookup_data)
}

/// read the contents of a directory
fn rootfs_readdir(file: Arc<Mutex<File>>) -> StrResult<DirContext> {
    wwarn!("rootfs_readdir");
    let inode = file.lock().f_dentry.lock().d_inode.clone();
    let inode = inode.lock();
    let number = inode.number;
    let bind = ROOT_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    let mut data = Vec::new();
    ram_inode.dentries.keys().for_each(|x| {
        data.extend_from_slice(x.as_bytes());
        data.push(0);
    });
    let dir_context = DirContext::new(data);
    wwarn!("rootfs_readdir end");
    Ok(dir_context)
}

fn rootfs_rmdir(dir: Arc<Mutex<Inode>>, dentry: Arc<Mutex<DirEntry>>) -> StrResult<()> {
    wwarn!("rootfs_rmdir");
    let mut inode = dir.lock();
    let number = inode.number;
    let mut bind = ROOT_FS.lock();
    let ram_inode = bind.get_mut(&number).unwrap();
    // check if the dir is empty
    assert!(!ram_inode.dentries.is_empty());
    // delete the sub dir
    // find name from data
    let name = dentry.lock().d_name.clone();
    ram_inode.dentries.remove(&name);

    let sub_dir = dentry.lock().d_inode.clone();
    let sub_dir = sub_dir.lock();
    let sub_number = sub_dir.number;

    // update the dir size
    inode.file_size = ram_inode.dentries.len();
    // delete the sub dir
    bind.remove(&sub_number);
    wwarn!("rootfs_rmdir end");
    Ok(())
}

fn rootfs_get_attr(dentry: Arc<Mutex<DirEntry>>, key: &str, val: &mut [u8]) -> StrResult<usize> {
    let inode = dentry.lock().d_inode.clone();
    let number = inode.lock().number;
    let bind = ROOT_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    let ex_attr = ram_inode.ex_attr.get(key).unwrap();
    let len = ex_attr.as_slice().len();
    let min_len = min(len, val.len());
    val[..min_len].copy_from_slice(&ex_attr.as_slice()[..min_len]);
    Ok(min_len)
}
fn rootfs_set_attr(dentry: Arc<Mutex<DirEntry>>, key: &str, val: &[u8]) -> StrResult<()> {
    let inode = dentry.lock().d_inode.clone();
    let number = inode.lock().number;
    let mut bind = ROOT_FS.lock();
    let ram_inode = bind.get_mut(&number).unwrap();
    ram_inode.ex_attr.insert(key.to_string(), val.to_vec());
    Ok(())
}
fn rootfs_remove_attr(dentry: Arc<Mutex<DirEntry>>, key: &str) -> StrResult<()> {
    let inode = dentry.lock().d_inode.clone();
    let number = inode.lock().number;
    let mut bind = ROOT_FS.lock();
    let ram_inode = bind.get_mut(&number).unwrap();
    ram_inode.ex_attr.remove(key);
    Ok(())
}
fn rootfs_list_attr(dentry: Arc<Mutex<DirEntry>>, buf: &mut [u8]) -> StrResult<usize> {
    let inode = dentry.lock().d_inode.clone();
    let number = inode.lock().number;
    let bind = ROOT_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    let mut attr_list = String::new();
    for (key, _) in ram_inode.ex_attr.iter() {
        attr_list.push_str(key);
        attr_list.push(0 as char);
    }
    let len = attr_list.as_bytes().len();
    let min_len = min(len, buf.len());
    buf[..min_len].copy_from_slice(&attr_list.as_bytes()[..min_len]);
    Ok(min_len)
}

fn rootfs_truncate(inode: Arc<Mutex<Inode>>) -> StrResult<()> {
    let number = inode.lock().number;
    let mut bind = ROOT_FS.lock();
    let ram_inode = bind.get_mut(&number).unwrap();
    let new_size = inode.lock().file_size;
    if new_size > ram_inode.data.len() {
        ram_inode.data.resize(new_size, 0);
    } else {
        ram_inode.data.truncate(new_size);
    }
    Ok(())
}

fn rootfs_rename(
    old_dir: Arc<Mutex<Inode>>,
    old_dentry: Arc<Mutex<DirEntry>>,
    new_dir: Arc<Mutex<Inode>>,
    new_dentry: Arc<Mutex<DirEntry>>,
) -> StrResult<()> {
    wwarn!("rootfs_rename");
    let mut old_dir_lock = old_dir.lock();
    let old_dir_number = old_dir_lock.number;
    let mut bind = ROOT_FS.lock();
    let old_dir_inode = bind.get_mut(&old_dir_number).unwrap();
    let old_name = old_dentry.lock().d_name.clone();
    old_dir_inode.dentries.remove(&old_name);
    old_dir_lock.file_size -= 1;

    info!("{:?}", old_dir_inode.dentries);
    drop(old_dir_lock);
    info!("update old dir over ....");
    let mut new_dir_lock = new_dir.lock();
    let new_dir_number = new_dir_lock.number;
    let new_dir_inode = bind.get_mut(&new_dir_number).unwrap();
    let new_name = new_dentry.lock().d_name.clone();
    let is_new = new_dir_inode
        .dentries
        .insert(new_name, old_dentry.lock().d_inode.lock().number);
    if is_new.is_some() {
        info!("is new file");
        //mark the new file as invalid
        let new_file = new_dentry.lock().d_inode.clone();
        let mut new_file = new_file.lock();
        new_file.flags = InodeFlags::S_INVALID;
        let new_file_number = new_file.number;
        bind.remove(&new_file_number);
    } else {
        new_dir_lock.file_size += 1;
    }
    wwarn!("rootfs_rename end");
    Ok(())
}
