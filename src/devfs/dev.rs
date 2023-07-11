use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::sync::{Arc, Weak};

use core::cmp::min;

use crate::dentry::{
    DirEntry, DirEntryOps, DirFlags, Dirent64, DirentType, LookUpData, LookUpFlags,
};
use crate::devfs::{DevDir, DevNode, DevType};
use crate::file::{File, FileMode, FileOps};
use crate::info::MAGIC_BASE;
use crate::inode::{create_tmp_inode_from_sb_blk, Inode, InodeMode, InodeOps};
use crate::mount::{MountFlags, VfsMount};
use crate::superblock::{
    find_super_blk, DataOps, FileSystemType, StatFs, SuperBlock, SuperBlockInner, SuperBlockOps,
};
use crate::StrResult;
use core::sync::atomic::{AtomicUsize, Ordering};
use log::{debug, warn};
use spin::Mutex;

static INODE_COUNT: AtomicUsize = AtomicUsize::new(0);

const DEVFS_SB_OPS: SuperBlockOps = {
    let mut ops = SuperBlockOps::empty();
    ops.stat_fs = devfs_stat_fs;
    ops
};

const DEVFS_DIR_INODE_OPS: InodeOps = {
    let mut ops = InodeOps::empty();
    ops.lookup = devfs_dir_lookup;
    ops.unlink = devfs_dir_unlink;
    ops.mkdir = devfs_dir_mkdir;
    ops.symlink = devfs_dir_symlink;
    ops.rmdir = devfs_dir_rmdir;
    // ops.create = devfs_dir_create;
    ops.mknod = devfs_dir_mknod;
    ops
};

const DEVFS_SYMLINK_INODE_OPS: InodeOps = {
    let mut ops = InodeOps::empty();
    ops.readlink = devfs_symlink_readlink;
    ops.follow_link = devfs_symlink_follow_link;
    ops
};

const DEVFS_DIR_FILE_OPS: FileOps = {
    let mut ops = FileOps::empty();
    ops.readdir = devfs_dir_readdir;
    ops.open = |_| Ok(());
    ops
};

const DEVFS_OTHER_FILE_OPS: FileOps = {
    let mut ops = FileOps::empty();
    ops.open = |_| Ok(());
    ops.write = devfs_other_file_write;
    ops.read = devfs_other_file_read;
    ops
};

fn devfs_stat_fs(super_blk: Arc<SuperBlock>) -> StrResult<StatFs> {
    let mut name = [0u8; 32];
    let dev_name = "devfs";
    let name_len = min(dev_name.len(), name.len());
    name[..name_len].copy_from_slice(dev_name.as_bytes());
    let statfs = StatFs {
        fs_type: 323232,
        block_size: super_blk.block_size as u64,
        total_blocks: 0,
        free_blocks: 0,
        total_inodes: 0,
        name_len: name_len as u32,
        name,
    };
    Ok(statfs)
}

pub fn devfs_get_super_blk(
    fs_type: Arc<FileSystemType>,
    flags: MountFlags,
    dev_name: &str,
    data: Option<Box<dyn DataOps>>,
) -> StrResult<Arc<SuperBlock>> {
    let test_func = |_x: Arc<SuperBlock>| true;
    let find_sb_blk = find_super_blk(fs_type.clone(), Some(&test_func));
    let sb_blk = match find_sb_blk {
        // 找到了旧超级快
        Ok(sb_blk) => sb_blk,
        Err(_) => {
            // 没有找到旧超级快需要重新分配
            debug!("create new super block for devfs");
            create_dev_super_blk(fs_type, flags, dev_name, data)?
        }
    };
    let inode = devfs_root_inode(sb_blk.clone())?;
    // 根目录硬链接计数不用自增1
    assert_eq!(inode.access_inner().hard_links, 1);
    // create dentry
    let dentry = devfs_root_dentry(inode)?;
    sb_blk.update_root(dentry);
    Ok(sb_blk)
}

fn devfs_root_dentry(inode: Arc<Inode>) -> StrResult<Arc<DirEntry>> {
    let dentry = DirEntry::new(
        DirFlags::empty(),
        inode,
        DirEntryOps::empty(),
        Weak::new(),
        "/",
    );
    let dentry = Arc::new(dentry);
    Ok(dentry)
}

fn devfs_root_inode(sb_blk: Arc<SuperBlock>) -> StrResult<Arc<Inode>> {
    let number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    assert_eq!(number, 0);
    let inode = create_tmp_inode_from_sb_blk(
        sb_blk,
        number,
        InodeMode::S_DIR,
        0,
        DEVFS_DIR_INODE_OPS,
        DEVFS_DIR_FILE_OPS,
        None,
    )?;
    inode.access_inner().hard_links = 1;
    let devfs_inode = DevNode::new(
        InodeMode::S_DIR,
        0,
        "root".to_string(),
        DevType::Dir(DevDir::empty()),
        FileMode::FMODE_RDWR,
    );
    let devfs_inode = Arc::new(devfs_inode);
    devfs_inode.access_inner().parent = Arc::downgrade(&devfs_inode);
    inode.access_inner().data = Some(Box::new(devfs_inode));
    Ok(inode)
}

fn create_dev_super_blk(
    fs_type: Arc<FileSystemType>,
    flags: MountFlags,
    dev_name: &str,
    data: Option<Box<dyn DataOps>>,
) -> StrResult<Arc<SuperBlock>> {
    let sb_blk = SuperBlock {
        dev_desc: 0,
        device: None,
        block_size: 1024,
        dirty_flag: false,
        file_max_bytes: 0,
        mount_flag: flags,
        magic: (MAGIC_BASE + 7) as u32,
        file_system_type: Arc::downgrade(&fs_type),
        super_block_ops: DEVFS_SB_OPS,
        blk_dev_name: dev_name.to_string(),
        data,
        inner: Mutex::new(SuperBlockInner::empty()),
    };
    let sb_blk = Arc::new(sb_blk);
    Ok(sb_blk)
}

pub fn devfs_kill_super_blk(super_blk: Arc<SuperBlock>) {
    let mut sb_inner = super_blk.access_inner();
    sb_inner.root = Arc::new(DirEntry::empty());
    sb_inner.files.clear();
    sb_inner.dirty_inode.clear();
    sb_inner.sync_inode.clear();
}

fn devfs_dir_lookup(dir: Arc<Inode>, dentry: Arc<DirEntry>) -> StrResult<()> {
    let devnode = inode_to_devnode(dir.clone())?;
    let name = dentry.access_inner().d_name.to_string();
    let child = __dev_find_in_dir(devnode, &name)?;
    let inode = devfs_create_inode(dir, child)?;
    dentry.access_inner().d_inode = inode;
    Ok(())
}

fn devfs_dir_unlink(dir: Arc<Inode>, dentry: Arc<DirEntry>) -> StrResult<()> {
    let devnode = inode_to_devnode(dir.clone())?;
    let name = dentry.access_inner().d_name.to_string();
    if !devnode.access_inner().may_delete {
        return Err("The node can't be deleted");
    }
    match &devnode.access_inner().dev_type {
        DevType::Dir(_dir) => {
            panic!("It is dir")
        }
        _ => {}
    }
    let parent = devnode.access_inner().parent.upgrade().unwrap();
    if let DevType::Dir(dir) = &mut parent.access_inner().dev_type {
        dir.children.retain(|node| node.access_inner().name != name);
    }
    let sub_file = dentry.access_inner().d_inode.clone();
    // remove devnode from inode data
    sub_file.access_inner().data = None;
    Ok(())
}

fn devfs_dir_mkdir(dir: Arc<Inode>, dentry: Arc<DirEntry>, mode: FileMode) -> StrResult<()> {
    let devnode = inode_to_devnode(dir.clone())?;
    let name = dentry.access_inner().d_name.to_string();
    let number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    let new_node = DevNode::new(
        InodeMode::S_DIR,
        number,
        name,
        DevType::Dir(DevDir::empty()),
        mode,
    );
    let new_node = Arc::new(new_node);
    devfs_node_stick(devnode.clone(), new_node.clone())?;
    let inode = devfs_create_inode(dir, new_node)?;
    dentry.access_inner().d_inode = inode;
    Ok(())
}

fn devfs_dir_rmdir(dir: Arc<Inode>, dentry: Arc<DirEntry>) -> StrResult<()> {
    debug!("devfs_dir_rmdir");
    let sub_dir = dentry.access_inner().d_inode.clone();
    let sub_devnode = inode_to_devnode(sub_dir.clone())?;
    if !sub_devnode.mode.contains(InodeMode::S_DIR) {
        return Err("It is not dir");
    }
    if !sub_devnode.access_inner().may_delete {
        return Err("The node can't be deleted");
    }
    if let DevType::Dir(dir) = &mut sub_devnode.access_inner().dev_type {
        if !dir.children.is_empty() {
            return Err("The dir is not empty");
        }
        dir.inactive = true;
    }
    let parent = sub_devnode.access_inner().parent.upgrade().unwrap();
    if let DevType::Dir(dir) = &mut parent.access_inner().dev_type {
        dir.children.retain(|node| !Arc::ptr_eq(node, &sub_devnode));
    }
    // remove devnode from inode data
    sub_dir.access_inner().data = None;
    dir.access_inner().file_size -= 1;
    Ok(())
}

fn devfs_dir_symlink(dir: Arc<Inode>, dentry: Arc<DirEntry>, target: &str) -> StrResult<()> {
    let devnode = inode_to_devnode(dir.clone())?;
    let name = dentry.access_inner().d_name.to_string();
    let number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    let new_node = DevNode::new(
        InodeMode::S_SYMLINK,
        number,
        name,
        DevType::SymLink(target.to_string()),
        FileMode::empty(),
    );
    let new_node = Arc::new(new_node);
    devfs_node_stick(devnode.clone(), new_node.clone())?;
    let inode = devfs_create_inode(dir, new_node)?;
    dentry.access_inner().d_inode = inode;
    Ok(())
}

fn devfs_dir_mknod(
    dir: Arc<Inode>,
    dentry: Arc<DirEntry>,
    type_: InodeMode,
    mode: FileMode,
    dev: u32,
) -> StrResult<()> {
    let devnode = inode_to_devnode(dir.clone())?;
    let name = dentry.access_inner().d_name.to_string();
    let number = INODE_COUNT.fetch_add(1, Ordering::SeqCst);
    let new_node = DevNode::new(type_, number, name, DevType::Dev(dev), mode);
    let new_node = Arc::new(new_node);
    devfs_node_stick(devnode.clone(), new_node.clone())?;
    let inode = devfs_create_inode(dir, new_node)?;
    dentry.access_inner().d_inode = inode;
    Ok(())
}

fn devfs_symlink_readlink(dentry: Arc<DirEntry>, buf: &mut [u8]) -> StrResult<usize> {
    let follow_link = dentry.access_inner().d_inode.inode_ops.follow_link;
    let mut lookup_data = LookUpData::new(
        LookUpFlags::READ_LINK,
        dentry.clone(),
        Arc::new(VfsMount::empty()),
    );
    follow_link(dentry, &mut lookup_data)?;
    let target = lookup_data.symlink_names.last().unwrap();
    if buf.is_empty() {
        return Ok(target.len());
    }
    let len = buf.len().min(target.len());
    buf[..len].copy_from_slice(&target.as_bytes()[..len]);
    Ok(len)
}

fn devfs_symlink_follow_link(dentry: Arc<DirEntry>, lookup_data: &mut LookUpData) -> StrResult<()> {
    let devnode = inode_to_devnode(dentry.access_inner().d_inode.clone())?;
    if !devnode.mode.contains(InodeMode::S_SYMLINK) {
        return Err("It is not symlink");
    }
    if let DevType::SymLink(target) = &devnode.access_inner().dev_type {
        lookup_data.symlink_names.push(target.clone());
    }
    Ok(())
}

pub fn devfs_dir_readdir(file: Arc<File>, dirents: &mut [u8]) -> StrResult<usize> {
    debug!("devfs_dir_readdir");
    let devnode = inode_to_devnode(file.f_dentry.access_inner().d_inode.clone())?;
    let mut pos = file.access_inner().f_pos;
    if dirents.is_empty() {
        let base = if devnode.number != 0 {
            let dirent0 = Dirent64::new(".", 0 as u64, 0, DirentType::DT_DIR);
            let dirent1 = Dirent64::new("..", 0 as u64, 0, DirentType::DT_DIR);
            dirent0.len() + dirent1.len()
        } else {
            0
        };
        if let DevType::Dir(dir) = &devnode.access_inner().dev_type {
            let size = dir
                .children
                .iter()
                .map(|node| {
                    let name = &node.access_inner().name;
                    let dirent64 = Dirent64::new(name, 0, 0, DirentType::empty());
                    dirent64.len()
                })
                .sum::<usize>();
            return Ok(size + base);
        }
        return Ok(base);
    }
    let mut count = 0;
    let buf_len = dirents.len();
    let mut ptr = dirents.as_mut_ptr();
    loop {
        let ino = devnode.number;
        let (dirent, name) = if pos == 0 && ino != 0 {
            let dirent = Dirent64::new(".", ino as u64, 0, DirentType::DT_DIR);
            (dirent, ".".to_string())
        } else if pos == 1 && ino != 0 {
            let ino = devnode.access_inner().parent.upgrade().unwrap().number;
            let dirent = Dirent64::new("..", ino as u64, 0, DirentType::DT_DIR);
            (dirent, "..".to_string())
        } else {
            let index = if ino == 0 { pos } else { pos - 2 };
            if let DevType::Dir(dir) = &devnode.access_inner().dev_type {
                if index >= dir.children.len() {
                    break;
                }
                let node = &dir.children[index];
                let ino = node.number;
                let dirent = Dirent64::new(
                    &node.access_inner().name,
                    ino as u64,
                    index as i64,
                    node.mode.into(),
                );
                (dirent, node.access_inner().name.clone())
            } else {
                return Err("It is not dir");
            }
        };
        if count + dirent.len() <= buf_len {
            let dirent_ptr = unsafe { &mut *(ptr as *mut Dirent64) };
            *dirent_ptr = dirent;
            let name_ptr = dirent_ptr.name.as_mut_ptr();
            unsafe {
                let mut name = name;
                name.push('\0');
                let len = name.len();
                name_ptr.copy_from(name.as_ptr(), len);
                ptr = ptr.add(dirent_ptr.len());
            }
            count += dirent_ptr.len();
            file.access_inner().f_pos += 1;
            pos += 1;
        } else {
            break;
        }
    }
    Ok(count)
}

fn devfs_node_stick(dir: Arc<DevNode>, new_node: Arc<DevNode>) -> StrResult<()> {
    if !dir.mode.contains(InodeMode::S_DIR) {
        return Err("It is not dir");
    }
    let mut dir_inner = dir.access_inner();
    if let DevType::Dir(s_dir) = &mut dir_inner.dev_type {
        if s_dir.inactive {
            return Err("The dir is inactive");
        }
        if s_dir
            .children
            .iter()
            .any(|node| node.access_inner().name == new_node.access_inner().name)
        {
            return Err("The node is exist");
        }
        new_node.access_inner().parent = Arc::downgrade(&dir);
        s_dir.children.push(new_node);
    }
    Ok(())
}

fn devfs_create_inode(dir: Arc<Inode>, node: Arc<DevNode>) -> StrResult<Arc<Inode>> {
    let (inode_ops, file_ops, dev_desc) = match node.mode {
        InodeMode::S_DIR => (DEVFS_DIR_INODE_OPS, DEVFS_DIR_FILE_OPS, 0),
        InodeMode::S_SYMLINK => (DEVFS_SYMLINK_INODE_OPS, DEVFS_OTHER_FILE_OPS, 0),
        InodeMode::S_CHARDEV | InodeMode::S_BLKDEV => {
            let dev_desc = match &node.access_inner().dev_type {
                DevType::Dev(dev) => *dev,
                _ => return Err("devfs_create_inode error"),
            };
            (InodeOps::empty(), DEVFS_OTHER_FILE_OPS, dev_desc as u32)
        }
        InodeMode::S_FIFO | InodeMode::S_SOCK => (InodeOps::empty(), DEVFS_OTHER_FILE_OPS, 0),
        _ => panic!("devfs_create_inode error"),
    };

    let inode = create_tmp_inode_from_sb_blk(
        dir.super_blk.upgrade().unwrap(),
        node.number,
        node.mode,
        dev_desc,
        inode_ops,
        file_ops,
        None,
    )?;

    match node.mode {
        InodeMode::S_SYMLINK => {
            inode.access_inner().file_size = match &node.access_inner().dev_type {
                DevType::SymLink(s) => s.len(),
                _ => 0,
            };
        }
        InodeMode::S_CHARDEV | InodeMode::S_BLKDEV => {}
        InodeMode::S_FIFO | InodeMode::S_SOCK => {}
        _ => {}
    };
    // warn!("{} devfs_create_inode:{}",dir.number,inode.number);
    inode.access_inner().data = Some(Box::new(node));
    dir.access_inner().file_size += 1;
    Ok(inode)
}

fn inode_to_devnode(inode: Arc<Inode>) -> StrResult<Arc<DevNode>> {
    let inode_inner = inode.access_inner();
    let data = inode_inner.data.as_ref().unwrap();
    let devnode = data.downcast_ref::<Arc<DevNode>>().unwrap();
    Ok(devnode.clone())
}

fn __dev_find_in_dir(node: Arc<DevNode>, name: &str) -> StrResult<Arc<DevNode>> {
    if !node.mode.contains(InodeMode::S_DIR) {
        return Err("not a dir");
    }
    return match &node.access_inner().dev_type {
        DevType::Dir(dir) => {
            let f = dir.children.iter().find(|x| x.access_inner().name == name);
            match f {
                Some(x) => Ok(x.clone()),
                None => Err("not found"),
            }
        }
        _ => Err("not a dir"),
    };
}

fn devfs_other_file_write(file: Arc<File>, _buf: &[u8], _offset: u64) -> StrResult<usize> {
    let _devnode = inode_to_devnode(file.f_dentry.access_inner().d_inode.clone())?;
    // now we don't support write
    Ok(0)
}

fn devfs_other_file_read(file: Arc<File>, buf: &mut [u8], _offset: u64) -> StrResult<usize> {
    warn!("devfs_other_file_read");
    let devnode = inode_to_devnode(file.f_dentry.access_inner().d_inode.clone())?;
    match devnode.access_inner().dev_type {
        DevType::Dev(dev) => {
            if dev == 0 || dev == u32::MAX {
                buf.fill(0);
                return Ok(buf.len());
            }
        }
        DevType::Dir(_) => {}
        DevType::SymLink(_) => {}
        DevType::Regular => {}
    }
    Ok(0)
}
