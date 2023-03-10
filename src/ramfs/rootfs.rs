use super::{
    ramfs_create, ramfs_create_root_dentry, ramfs_create_root_inode, ramfs_follow_link,
    ramfs_kill_super_blk, ramfs_link, ramfs_mkdir, ramfs_read_file, ramfs_read_link,
    ramfs_simple_super_blk, ramfs_symlink, ramfs_unlink, ramfs_write_file, RamFsInode,
};
use crate::dentry::{DirContext, DirEntry, LookUpData};
use crate::file::{File, FileMode, FileOps};
use crate::inode::{Inode, InodeFlags, InodeMode, InodeOps};
use crate::mount::MountFlags;
use crate::superblock::{DataOps, FileSystemAttr, FileSystemType, FileSystemTypeInner, SuperBlock};
use crate::{ddebug, StrResult};
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cmp::min;
use core::sync::atomic::{AtomicUsize, Ordering};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use log::{debug, error};
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
    ddebug!("rootfs_get_super_blk");
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
    assert_eq!(inode.access_inner().hard_links, 0);
    // 创建目录项
    let dentry = ramfs_create_root_dentry(None, inode)?;
    sb_blk.update_root(dentry);
    // 将sb_blk插入到fs_type的链表中
    fs_type.insert_super_blk(sb_blk.clone());
    ddebug!("rootfs_get_super_blk end");
    Ok(sb_blk)
}

fn rootfs_mkdir(dir: Arc<Inode>, dentry: Arc<DirEntry>, attr: FileMode) -> StrResult<()> {
    ddebug!("rootfs_mkdir");
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
    ddebug!("rootfs_mkdir end");
    Ok(())
}

fn rootfs_create(dir: Arc<Inode>, dentry: Arc<DirEntry>, mode: FileMode) -> StrResult<()> {
    ddebug!("rootfs_create");
    error!("***** {}", dentry.access_inner().d_name);
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
    ddebug!("rootfs_create end");
    Ok(())
}

fn rootfs_read_file(file: Arc<File>, buf: &mut [u8], offset: u64) -> StrResult<usize> {
    ddebug!("rootfs_read_file");
    let len = ramfs_read_file(ROOT_FS.clone(), file, buf, offset)?;
    ddebug!("rootfs_read_file end");
    Ok(len)
}
fn rootfs_write_file(file: Arc<File>, buf: &[u8], offset: u64) -> StrResult<usize> {
    ddebug!("rootfs_write_file");
    let len = ramfs_write_file(ROOT_FS.clone(), file, buf, offset)?;
    ddebug!("rootfs_write_file end");
    Ok(len)
}

/// create a hard link to the inode
fn rootfs_link(
    old_dentry: Arc<DirEntry>,
    dir: Arc<Inode>,
    new_dentry: Arc<DirEntry>,
) -> StrResult<()> {
    ddebug!("rootfs_link");
    let _number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    ramfs_link(ROOT_FS.clone(), old_dentry, dir, new_dentry)?;
    ddebug!("rootfs_link end");
    Ok(())
}

/// decrease the hard link count of the inode
fn rootfs_unlink(dir: Arc<Inode>, dentry: Arc<DirEntry>) -> StrResult<()> {
    ddebug!("rootfs_unlink");
    ramfs_unlink(ROOT_FS.clone(), dir, dentry)?;
    ddebug!("rootfs_unlink end");
    Ok(())
}

/// create a symbolic link
fn rootfs_symlink(dir: Arc<Inode>, dentry: Arc<DirEntry>, target: &str) -> StrResult<()> {
    ddebug!("rootfs_symlink");
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
    ddebug!("rootfs_symlink end");
    Ok(())
}

/// read the target of a symbolic link
fn rootfs_readlink(dentry: Arc<DirEntry>, buf: &mut [u8]) -> StrResult<usize> {
    let inode = dentry.access_inner().d_inode.clone();
    let inode = inode;
    let number = inode.number;
    let bind = ROOT_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    ramfs_read_link(ram_inode, buf)
}

/// follow a symbolic link
fn rootfs_follow_link(dentry: Arc<DirEntry>, lookup_data: &mut LookUpData) -> StrResult<()> {
    let inode = dentry.access_inner().d_inode.clone();
    let inode = inode;
    let number = inode.number;
    let bind = ROOT_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    ramfs_follow_link(ram_inode, lookup_data)
}

/// read the contents of a directory
fn rootfs_readdir(file: Arc<File>) -> StrResult<DirContext> {
    ddebug!("rootfs_readdir");
    let inode = file.f_dentry.access_inner().d_inode.clone();
    let number = inode.number;
    let bind = ROOT_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    let mut data = Vec::new();
    ram_inode.dentries.keys().for_each(|x| {
        data.extend_from_slice(x.as_bytes());
        data.push(0);
    });
    let dir_context = DirContext::new(data);
    ddebug!("rootfs_readdir end");
    Ok(dir_context)
}

fn rootfs_rmdir(dir: Arc<Inode>, dentry: Arc<DirEntry>) -> StrResult<()> {
    ddebug!("rootfs_rmdir");
    let inode = dir;
    let number = inode.number;
    let mut bind = ROOT_FS.lock();
    let ram_inode = bind.get_mut(&number).unwrap();
    // check if the dir is empty
    assert!(!ram_inode.dentries.is_empty());
    // delete the sub dir
    // find name from data
    let name = dentry.access_inner().d_name.clone();
    ram_inode.dentries.remove(&name);

    let sub_dir = dentry.access_inner().d_inode.clone();
    let sub_number = sub_dir.number;
    // delete the sub dir
    bind.remove(&sub_number);
    ddebug!("rootfs_rmdir end");
    Ok(())
}

fn rootfs_get_attr(dentry: Arc<DirEntry>, key: &str, val: &mut [u8]) -> StrResult<usize> {
    let inode = dentry.access_inner().d_inode.clone();
    let number = inode.number;
    let bind = ROOT_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    let ex_attr = ram_inode.ex_attr.get(key).unwrap();
    let len = ex_attr.as_slice().len();
    let min_len = min(len, val.len());
    val[..min_len].copy_from_slice(&ex_attr.as_slice()[..min_len]);
    Ok(min_len)
}
fn rootfs_set_attr(dentry: Arc<DirEntry>, key: &str, val: &[u8]) -> StrResult<()> {
    let inode = dentry.access_inner().d_inode.clone();
    let number = inode.number;
    let mut bind = ROOT_FS.lock();
    let ram_inode = bind.get_mut(&number).unwrap();
    ram_inode.ex_attr.insert(key.to_string(), val.to_vec());
    Ok(())
}
fn rootfs_remove_attr(dentry: Arc<DirEntry>, key: &str) -> StrResult<()> {
    let inode = dentry.access_inner().d_inode.clone();
    let number = inode.number;
    let mut bind = ROOT_FS.lock();
    let ram_inode = bind.get_mut(&number).unwrap();
    ram_inode.ex_attr.remove(key);
    Ok(())
}
fn rootfs_list_attr(dentry: Arc<DirEntry>, buf: &mut [u8]) -> StrResult<usize> {
    let inode = dentry.access_inner().d_inode.clone();
    let number = inode.number;
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

fn rootfs_truncate(inode: Arc<Inode>) -> StrResult<()> {
    let number = inode.number;
    let mut bind = ROOT_FS.lock();
    let ram_inode = bind.get_mut(&number).unwrap();
    let new_size = inode.access_inner().file_size;
    if new_size > ram_inode.data.len() {
        ram_inode.data.resize(new_size, 0);
    } else {
        ram_inode.data.truncate(new_size);
    }
    Ok(())
}

fn rootfs_rename(
    old_dir: Arc<Inode>,
    old_dentry: Arc<DirEntry>,
    new_dir: Arc<Inode>,
    new_dentry: Arc<DirEntry>,
) -> StrResult<()> {
    ddebug!("rootfs_rename");
    let old_dir_number = old_dir.number;
    let mut bind = ROOT_FS.lock();
    let old_dir_inode = bind.get_mut(&old_dir_number).unwrap();
    let old_name = old_dentry.access_inner().d_name.clone();
    old_dir_inode.dentries.remove(&old_name);
    old_dir.access_inner().file_size -= 1;

    debug!("{:?}", old_dir_inode.dentries);
    debug!("update old dir over ....");
    let new_dir_number = new_dir.number;
    let new_dir_inode = bind.get_mut(&new_dir_number).unwrap();
    let new_name = new_dentry.access_inner().d_name.clone();
    let is_new = new_dir_inode
        .dentries
        .insert(new_name, old_dentry.access_inner().d_inode.number);
    if is_new.is_some() {
        debug!("is new file");
        //mark the new file as invalid
        let new_file = new_dentry.access_inner().d_inode.clone();
        new_file.access_inner().flags = InodeFlags::S_INVALID;
        let new_file_number = new_file.number;
        bind.remove(&new_file_number);
    } else {
        new_dir.access_inner().file_size += 1;
    }
    ddebug!("rootfs_rename end");
    Ok(())
}
