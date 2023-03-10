use crate::dentry::{find_file_indir, path_walk, DirEntry, LookUpFlags, PathType};
use crate::info::ProcessFs;
use crate::inode::{Inode, InodeMode};
use crate::{ddebug, StrResult};
use alloc::borrow::ToOwned;
use alloc::sync::Arc;
use log::{debug};

/// create a symlink
/// * target: the target of the symlink
/// * link: the path of the symlink
pub fn vfs_symlink<T: ProcessFs>(target: &str, link: &str) -> StrResult<()> {
    ddebug!("vfs_symlink");
    let new_lookup_data = path_walk::<T>(link, LookUpFlags::NOLAST);
    if new_lookup_data.is_err() {
        return Err("vfs_link: new path not found");
    }
    let mut new_lookup_data = new_lookup_data.unwrap();
    debug!(
        "vfs_link: new_lookup_data.path_type = {:?}",
        new_lookup_data.path_type
    );
    if new_lookup_data.path_type != PathType::PATH_NORMAL {
        return Err("It is not a normal path");
    }

    let last = new_lookup_data.last.to_owned();
    // 搜索子目录
    let sub_dentry = find_file_indir(&mut new_lookup_data, &last);
    if sub_dentry.is_ok() {
        return Err("The file already exists");
    }

    let target_dentry = Arc::new(DirEntry::from_lookup_data(&new_lookup_data));
    let dir = new_lookup_data.dentry.access_inner().d_inode.clone();
    let dentry = new_lookup_data.dentry.clone();
    do_symlink(dir, target_dentry.clone(), target)?;
    dentry.insert_child(target_dentry);
    ddebug!("vfs_symlink: end");
    Ok(())
}

fn do_symlink(dir: Arc<Inode>, dentry: Arc<DirEntry>, target: &str) -> StrResult<()> {
    ddebug!("do_symlink");
    may_create(dir.clone(), dentry.clone())?;
    let fs_symlink = dir.inode_ops.symlink;
    fs_symlink(dir, dentry, target)?;
    ddebug!("do_symlink: end");
    Ok(())
}

/// Check whether we can create an object with dentry child in directory dir.
#[inline]
fn may_create(dir: Arc<Inode>, child: Arc<DirEntry>) -> StrResult<()> {
    ddebug!("may_create");
    if child.access_inner().d_inode.mode != InodeMode::empty() {
        return Err("The file already exists");
    }
    if dir.mode != InodeMode::S_DIR {
        return Err("It is not a directory");
    }
    // if dir.lock().uid != 0 && dir.lock().uid != child.lock().uid {
    //     return Err("Permission denied");
    // }
    ddebug!("may_create: end");
    Ok(())
}

pub fn vfs_readlink<T: ProcessFs>(path: &str, buf: &mut [u8]) -> StrResult<usize> {
    ddebug!("vfs_readlink");
    let lookup_data = path_walk::<T>(path, LookUpFlags::empty())?;
    let dentry = lookup_data.dentry.clone();
    let inode = lookup_data.dentry.access_inner().d_inode.clone();
    let mode = inode.mode;
    let len = match mode {
        InodeMode::S_SYMLINK => {
            let readlink = inode.inode_ops.readlink;
            readlink(dentry, buf)?
        }
        _ => {
            return Err("It is not a symlink");
        }
    };
    ddebug!("vfs_readlink: end");
    Ok(len)
}
