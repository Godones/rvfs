use crate::dentry::{DirEntry, Dirent64, DirentType, LookUpData};
use crate::inode::{Inode, InodeFlags, InodeMode, InodeOps};
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cmp::min;
use core::sync::atomic::{AtomicUsize, Ordering};
use hashbrown::HashMap;

use lazy_static::lazy_static;
use log::debug;

use super::{
    ramfs_create, ramfs_create_root_dentry, ramfs_create_root_inode, ramfs_follow_link,
    ramfs_kill_super_blk, ramfs_link, ramfs_mkdir, ramfs_read_file, ramfs_read_link,
    ramfs_simple_super_blk, ramfs_symlink, ramfs_unlink, ramfs_write_file, RamFsInode,
};
use crate::file::{File, FileMode, FileOps};
use crate::mount::MountFlags;
use crate::superblock::{DataOps, FileSystemAttr, FileSystemType, FileSystemTypeInner, SuperBlock};
use crate::{ddebug, StrResult};
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
        get_super_blk: tmpfs_get_super_blk,
        kill_super_blk: ramfs_kill_super_blk,
        inner: Mutex::new(FileSystemTypeInner {
            super_blk_s: Vec::new(),
        }),
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
    ops.get_attr = tmpfs_get_attr;
    ops.set_attr = tmpfs_set_attr;
    ops.remove_attr = tmpfs_remove_attr;
    ops.list_attr = tmpfs_list_attr;
    ops.rename = tmpfs_rename;
    ops
};

const TMPFS_FILE_INODE_OPS: InodeOps = {
    let mut ops = InodeOps::empty();
    ops.get_attr = tmpfs_get_attr;
    ops.set_attr = tmpfs_set_attr;
    ops.remove_attr = tmpfs_remove_attr;
    ops.list_attr = tmpfs_list_attr;
    ops.truncate = tmpfs_truncate;
    ops
};

const TMPFS_SYMLINK_INODE_OPS: InodeOps = {
    let mut ops = InodeOps::empty();
    ops.readlink = tmpfs_readlink;
    ops.follow_link = tmpfs_follow_link;
    ops.get_attr = tmpfs_get_attr;
    ops.set_attr = tmpfs_set_attr;
    ops.remove_attr = tmpfs_remove_attr;
    ops.list_attr = tmpfs_list_attr;
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
    let mut ops = FileOps::empty();
    ops.open = |_| Ok(());
    ops
};

const TMPFS_DIR_FILE_OPS: FileOps = {
    let mut ops = FileOps::empty();
    ops.open = |_| Ok(());
    ops.readdir = tmpfs_readdir;
    ops
};

fn tmpfs_get_super_blk(
    fs_type: Arc<FileSystemType>,
    flags: MountFlags,
    dev_name: &str,
    data: Option<Box<dyn DataOps>>,
) -> StrResult<Arc<SuperBlock>> {
    ddebug!("tmpfs_get_super_blk");
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
    // 创建目录项
    let dentry = ramfs_create_root_dentry(None, inode)?;
    sb_blk.update_root(dentry);
    // 将sb_blk插入到fs_type的链表中
    fs_type.insert_super_blk(sb_blk.clone());
    ddebug!("tmpfs_get_super_blk end");
    Ok(sb_blk)
}

fn tmpfs_mkdir(dir: Arc<Inode>, dentry: Arc<DirEntry>, attr: FileMode) -> StrResult<()> {
    ddebug!("tmpfs_mkdir");
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
    ddebug!("tmpfs_mkdir end");
    Ok(())
}

fn tmpfs_create(dir: Arc<Inode>, dentry: Arc<DirEntry>, mode: FileMode) -> StrResult<()> {
    ddebug!("tmpfs_create");
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
    ddebug!("tmpfs_create end");
    Ok(())
}

fn tmpfs_read_file(file: Arc<File>, buf: &mut [u8], offset: u64) -> StrResult<usize> {
    ddebug!("tmpfs_read_file");
    let len = ramfs_read_file(TMP_FS.clone(), file, buf, offset);
    ddebug!("tmpfs_read_file end");
    len
}
fn tmpfs_write_file(file: Arc<File>, buf: &[u8], offset: u64) -> StrResult<usize> {
    ddebug!("tmpfs_write_file");
    let len = ramfs_write_file(TMP_FS.clone(), file, buf, offset);
    ddebug!("tmpfs_write_file end");
    len
}

/// 创建硬链接
fn tmpfs_link(
    old_dentry: Arc<DirEntry>,
    dir: Arc<Inode>,
    new_dentry: Arc<DirEntry>,
) -> StrResult<()> {
    ddebug!("tmpfs_link");
    let _number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    ramfs_link(TMP_FS.clone(), old_dentry, dir, new_dentry)?;
    ddebug!("tmpfs_link end");
    Ok(())
}

/// 删除硬链接
fn tmpfs_unlink(dir: Arc<Inode>, dentry: Arc<DirEntry>) -> StrResult<()> {
    ddebug!("tmpfs_link");
    ramfs_unlink(TMP_FS.clone(), dir, dentry)?;
    ddebug!("tmpfs_link end");
    Ok(())
}

/// create a symbolic link
fn tmpfs_symlink(dir: Arc<Inode>, dentry: Arc<DirEntry>, target: &str) -> StrResult<()> {
    ddebug!("tmpfs_symlink");
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
    ddebug!("tmpfs_symlink end");
    Ok(())
}

/// read the target of a symbolic link
fn tmpfs_readlink(dentry: Arc<DirEntry>, buf: &mut [u8]) -> StrResult<usize> {
    let inode = dentry.access_inner().d_inode.clone();
    let inode = inode;
    let number = inode.number;
    let bind = TMP_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    ramfs_read_link(ram_inode, buf)
}

/// follow a symbolic link
fn tmpfs_follow_link(dentry: Arc<DirEntry>, lookup_data: &mut LookUpData) -> StrResult<()> {
    let inode = dentry.access_inner().d_inode.clone();
    let inode = inode;
    let number = inode.number;
    let bind = TMP_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    ramfs_follow_link(ram_inode, lookup_data)
}

/// read the contents of a directory
fn tmpfs_readdir(file: Arc<File>, dirents: &mut [u8]) -> StrResult<usize> {
    ddebug!("rootfs_readdir");
    let inode = file.f_dentry.access_inner().d_inode.clone();
    let number = inode.number;
    let bind = TMP_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    let mut count = 0;
    let mut count_empty = 0;
    let buf_len = dirents.len();
    let mut ptr = dirents.as_mut_ptr();
    ram_inode
        .dentries
        .iter()
        .enumerate()
        .for_each(|(index, (name, &number))| {
            let sub_inode = bind.get(&number).unwrap();
            let type_ = DirentType::from(sub_inode.mode);
            let dirent = Dirent64::new(name, number as u64, index as i64, type_);
            count_empty += dirent.len();
            if count + dirent.len() <= buf_len {
                let dirent_ptr = unsafe { &mut *(ptr as *mut Dirent64) };
                *dirent_ptr = dirent;
                let name_ptr = dirent_ptr.name.as_mut_ptr();
                unsafe {
                    let mut name = name.clone();
                    name.push('\0');
                    let len = name.len();
                    name_ptr.copy_from(name.as_ptr(), len);
                    ptr = ptr.add(dirent_ptr.len());
                }
                count += dirent_ptr.len();
            }
        });
    // if the buf len is zero,we return the size of all dirents
    if buf_len == 0 {
        return Ok(count_empty);
    }
    ddebug!("rootfs_readdir end");
    Ok(count)
}

fn tmpfs_rmdir(dir: Arc<Inode>, dentry: Arc<DirEntry>) -> StrResult<()> {
    ddebug!("tmpfs_rmdir");
    let inode = dir;
    let number = inode.number;
    let mut bind = TMP_FS.lock();
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
    ddebug!("tmpfs_rmdir end");
    Ok(())
}

fn tmpfs_get_attr(dentry: Arc<DirEntry>, key: &str, val: &mut [u8]) -> StrResult<usize> {
    let inode = dentry.access_inner().d_inode.clone();
    let number = inode.number;
    let bind = TMP_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    let ex_attr = ram_inode.ex_attr.get(key).unwrap();
    let len = ex_attr.as_slice().len();
    let min_len = min(len, val.len());
    val[..min_len].copy_from_slice(&ex_attr.as_slice()[..min_len]);
    Ok(ex_attr.as_slice().len())
}
fn tmpfs_set_attr(dentry: Arc<DirEntry>, key: &str, val: &[u8]) -> StrResult<()> {
    let inode = dentry.access_inner().d_inode.clone();
    let number = inode.number;
    let mut bind = TMP_FS.lock();
    let ram_inode = bind.get_mut(&number).unwrap();
    ram_inode.ex_attr.insert(key.to_string(), val.to_vec());
    Ok(())
}
fn tmpfs_remove_attr(dentry: Arc<DirEntry>, key: &str) -> StrResult<()> {
    let inode = dentry.access_inner().d_inode.clone();
    let number = inode.number;
    let mut bind = TMP_FS.lock();
    let ram_inode = bind.get_mut(&number).unwrap();
    ram_inode.ex_attr.remove(key);
    Ok(())
}
fn tmpfs_list_attr(dentry: Arc<DirEntry>, buf: &mut [u8]) -> StrResult<usize> {
    let inode = dentry.access_inner().d_inode.clone();
    let number = inode.number;
    let bind = TMP_FS.lock();
    let ram_inode = bind.get(&number).unwrap();
    let mut attr_list = String::new();
    for (key, _) in ram_inode.ex_attr.iter() {
        attr_list.push_str(key);
        attr_list.push(0 as char);
    }
    let len = attr_list.as_bytes().len();
    let min_len = min(len, buf.len());
    buf[..min_len].copy_from_slice(&attr_list.as_bytes()[..min_len]);
    Ok(len)
}
fn tmpfs_truncate(inode: Arc<Inode>) -> StrResult<()> {
    let number = inode.number;
    let mut bind = TMP_FS.lock();
    let ram_inode = bind.get_mut(&number).unwrap();
    let new_size = inode.access_inner().file_size;
    if new_size > ram_inode.data.len() {
        ram_inode.data.resize(new_size, 0);
    } else {
        ram_inode.data.truncate(new_size);
    }
    Ok(())
}
fn tmpfs_rename(
    old_dir: Arc<Inode>,
    old_dentry: Arc<DirEntry>,
    new_dir: Arc<Inode>,
    new_dentry: Arc<DirEntry>,
) -> StrResult<()> {
    ddebug!("tmpfs_rename");
    let old_dir_number = old_dir.number;
    let mut bind = TMP_FS.lock();
    let old_dir_inode = bind.get_mut(&old_dir_number).unwrap();
    let old_name = old_dentry.access_inner().d_name.clone();
    old_dir_inode.dentries.remove(&old_name);
    old_dir.access_inner().file_size -= 1;

    debug!("{:?}", old_dir_inode.dentries);
    drop(old_dir);
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
    ddebug!("tmpfs_rename end");
    Ok(())
}
