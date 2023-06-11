use crate::dentry::{path_walk, DirEntry, LookUpData, LookUpFlags};
use crate::info::{ProcessFs, VfsError, VfsResult};
use crate::inode::{InodeFlags, InodeMode};
use crate::superblock::{lookup_filesystem, DataOps, SuperBlock};
use crate::{ddebug, StrResult, GLOBAL_HASH_MOUNT};
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use bitflags::bitflags;
use core::fmt::{Debug, Formatter};
use log::{debug, warn};
use spin::{Mutex, MutexGuard};

bitflags! {
    pub struct MountFlags:u32{
        const MNT_READ_ONLY = 0x1;
        const MNT_NOSUID = 0x2;
        const MNT_NO_DEV = 0x4 ;
        const MNT_NO_EXEC = 0x8;
        const MNT_INTERNAL = 0x10;
    }
}
/// 挂载点描述符
pub struct VfsMount {
    /// 挂载点标志
    pub flag: MountFlags,
    /// 设备名
    pub dev_name: String,
    /// 被挂载点的根目录
    pub root: Arc<DirEntry>,
    /// 本文件系统的超级快对象
    pub super_block: Arc<SuperBlock>,
    pub inner: Mutex<VfsMountInner>,
}
#[derive(Debug)]
pub struct VfsMountInner {
    /// 子挂载点链表
    pub child: Vec<Arc<VfsMount>>,
    /// 父文件系统
    pub parent: Weak<VfsMount>,
    /// 挂载点
    pub mount_point: Arc<DirEntry>,
}

impl Debug for VfsMount {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VfsMount")
            .field("flag", &self.flag)
            .field("dev_name", &self.dev_name)
            .field("root", &self.root)
            .field("super_block", &self.super_block)
            .field("inner", &self.inner)
            .finish()
    }
}

impl VfsMount {
    /// user should not use this function
    #[doc(hidden)]
    pub fn empty() -> Self {
        Self {
            flag: MountFlags::empty(),
            dev_name: String::new(),
            root: Arc::new(DirEntry::empty()),
            super_block: Arc::new(SuperBlock::empty()),
            inner: Mutex::new(VfsMountInner {
                child: Vec::new(),
                parent: Weak::new(),
                mount_point: Arc::new(DirEntry::empty()),
            }),
        }
    }
    pub fn new(
        dev_name: &str,
        super_block: Arc<SuperBlock>,
        parent: Weak<VfsMount>,
        mnt_flags: MountFlags,
    ) -> Arc<VfsMount> {
        // 设置挂载点所在目录与挂载的文件系统根目录相同
        let dir = super_block.access_inner().root.clone();
        let vfs_mount = VfsMount {
            flag: mnt_flags,
            dev_name: dev_name.to_string(),
            root: dir.clone(),
            super_block,
            inner: Mutex::new(VfsMountInner {
                child: Vec::new(),
                parent,
                mount_point: dir,
            }),
        };
        let mnt = Arc::new(vfs_mount);
        if mnt.access_inner().parent.upgrade().is_none() {
            mnt.access_inner().parent = Arc::downgrade(&mnt);
        }
        mnt
    }
    pub fn access_inner(&self) -> MutexGuard<VfsMountInner> {
        self.inner.lock()
    }
    /// 插入子挂载点
    pub fn inert_child(&self, child: Arc<VfsMount>) {
        self.access_inner().child.push(child);
    }
    /// 设置父挂载点
    pub fn set_parent(&self, parent: Arc<VfsMount>) {
        self.access_inner().parent = Arc::downgrade(&parent);
    }
}

unsafe impl Send for VfsMount {}
unsafe impl Sync for VfsMount {}

/// 挂载文件系统
/// # Arguments
/// * `dev_name` - 设备名
/// * `dir_name` - 挂载点
/// * `fs_type` - 文件系统名
/// * `flags` - 挂载标志
/// * `data` - 额外的数据
pub fn do_mount<T: ProcessFs>(
    dev_name: &str,
    dir_name: &str,
    fs_type: &str,
    flags: MountFlags,
    data: Option<Box<dyn DataOps>>,
) -> StrResult<Arc<VfsMount>> {
    ddebug!("do_mount");
    //检查路径名是否为空
    if dir_name.is_empty() {
        return Err("Dirname is empty");
    }
    let mut mnt_flags = MountFlags::empty();
    let mut flags = flags;
    //检查挂载标志
    if flags.contains(MountFlags::MNT_NOSUID) {
        mnt_flags |= MountFlags::MNT_NOSUID;
    }
    if flags.contains(MountFlags::MNT_NO_DEV) {
        mnt_flags |= MountFlags::MNT_NO_DEV;
    }
    if flags.contains(MountFlags::MNT_NO_EXEC) {
        mnt_flags |= MountFlags::MNT_NO_EXEC;
    }
    flags -= MountFlags::MNT_NOSUID & MountFlags::MNT_NO_DEV & MountFlags::MNT_NO_EXEC;
    //  查找找安装点的 dentry 数据结构
    let ret = path_walk::<T>(dir_name, LookUpFlags::READ_LINK);
    if ret.is_err() {
        return Err("Can'dentry find mount dir");
    }
    debug!("**do_mount: path_walk ok");
    let lookup_data = ret.unwrap();
    let ret = do_add_mount(&lookup_data, fs_type, flags, mnt_flags, dev_name, data);
    ddebug!("do_mount end");
    ret.map_err(|_| "do_add_mount error")
}

fn do_add_mount(
    look: &LookUpData,
    fs_type: &str,
    flags: MountFlags,
    mnt_flags: MountFlags,
    dev_name: &str,
    data: Option<Box<dyn DataOps>>,
) -> VfsResult<Arc<VfsMount>> {
    ddebug!("do_add_mount");
    if fs_type.is_empty() {
        return Err(VfsError::FsTypeNotFound);
    }
    // 加载文件系统超级块
    let mount = do_kernel_mount(fs_type, flags, dev_name, mnt_flags, data)?;
    debug!("**do_add_mount: do_kernel_mount ok");
    // 检查是否对用户空间不可见
    if mount
        .super_block
        .mount_flag
        .contains(MountFlags::MNT_INTERNAL)
    {
        return Err(VfsError::MountInternal);
    }
    // 挂载系统目录
    debug!("**do_add_mount: mount.lock().flag = mnt_flags ok");
    check_and_graft_tree(mount, look).map_err(|err| VfsError::Other(err.to_string()))
}

/// 生成一个挂载点
pub fn do_kernel_mount(
    fs_type: &str,
    flags: MountFlags,
    dev_name: &str,
    mnt_flags: MountFlags,
    data: Option<Box<dyn DataOps>>,
) -> VfsResult<Arc<VfsMount>> {
    ddebug!("do_kernel_mount");
    let fs_type = lookup_filesystem(fs_type);
    // 错误的文件系统类型
    if fs_type.is_none() {
        return Err(VfsError::FsTypeNotFound);
    }
    let fs_type = fs_type.unwrap();
    // 从设备读取文件系统超级块数据
    // find the same super block according to dev_name
    let super_blk = fs_type.find_super_blk(dev_name);

    warn!("super_blk = {:#x?}", super_blk);

    let super_blk = if super_blk.is_none() {
        let get_sb_func = fs_type.get_super_blk;
        let super_blk = (get_sb_func)(fs_type.clone(), flags, dev_name, data)
            .map_err(|err| VfsError::DiskFsError(err.to_string()))?;
        // 将sb_blk插入到fs_type的链表中
        fs_type.insert_super_blk(super_blk.clone());
        super_blk
    } else {
        super_blk.unwrap()
    };

    // 分配挂载点描述符
    let mount = VfsMount::new(dev_name, super_blk, Weak::new(), mnt_flags);
    ddebug!("do_kernel_mount end");
    Ok(mount)
}
/// 挂载到系统目录中
fn check_and_graft_tree(new_mount: Arc<VfsMount>, look: &LookUpData) -> StrResult<Arc<VfsMount>> {
    ddebug!("check_and_graft_tree");
    // 如果文件系统已经被安装在指定的安装点上，
    // let mnt = look.mnt.lock();
    // let root_eq = Arc::ptr_eq(&mnt.root, &look.dentry);
    // let find_sb_ref = mnt.super_block.as_ref().unwrap();
    // let new_sb_ref = t_new_mnt.super_block.as_ref().unwrap();
    // let eq = Arc::ptr_eq(find_sb_ref, new_sb_ref);
    // if eq && root_eq {
    //     return Err("fs exist");
    // }
    // 或者该安装点是一个符号链接，则释放读写信号量并返回错误
    if new_mount
        .root
        .access_inner()
        .d_inode
        .mode
        .contains(InodeMode::S_SYMLINK)
    {
        return Err("mnt is symlink");
    }
    graft_tree(new_mount.clone(), look)?;
    let mut global_mount_lock = GLOBAL_HASH_MOUNT.write();
    global_mount_lock.push(new_mount.clone());
    ddebug!("check_and_graft_tree end");
    Ok(new_mount)
}

fn graft_tree(new_mount: Arc<VfsMount>, look: &LookUpData) -> StrResult<()> {
    ddebug!("graft_tree");
    // mount点应该是目录
    // 被mount的对象也应当(根)目录
    if !look
        .dentry
        .access_inner()
        .d_inode
        .mode
        .contains(InodeMode::S_DIR)
        || !new_mount
            .root
            .access_inner()
            .d_inode
            .mode
            .contains(InodeMode::S_DIR)
    {
        return Err("not dir");
    }
    debug!("**graft_tree: check dir ok");
    // 目录被删除了(但是内存中还存在)
    let dentry = look.dentry.clone();
    let inode = dentry.access_inner().d_inode.clone();
    if inode.access_inner().flags.contains(InodeFlags::S_DEL) {
        return Err("inode del");
    }

    /*
     * 1、根目录总是可以被重新mount的
     * 2、如果目录还在缓存哈希表中，说明它是有效的，可mount
     * 否则不能mount
     */

    // TODO 修改判断挂载点可用的条件
    // let c = look.dentry.clone();
    // let p = look.dentry.access_inner().parent.upgrade();
    // if Arc::ptr_eq(&c, &p) {
    //     return Err("not in cache");
    // }

    // 设置父节点以及挂载点目录对象
    new_mount.set_parent(look.mnt.clone());
    new_mount.access_inner().mount_point = look.dentry.clone();
    // 加入上级对象的子对象链表中
    look.mnt.inert_child(new_mount);
    look.dentry.access_inner().mount_count += 1;

    // debug!("parent: {:#?}", look.mnt);
    // debug!("child: {:#?}", new_mount);

    ddebug!("graft_tree end");
    Ok(())
}

/// 从系统目录中卸载文件系统
/// 如果文件系统中的文件当前正在使用，该文件系统是不能被卸载的
/// 根据文件系统所在设备的标识符，检查在索引节点高速缓存中是否有来自该文件系统的 VFS 索引节
/// 点，如果有且使用计数大于 0，则说明该文件系统正在被使用，因此，该文件系统不能被卸
/// 载。否则，查看对应的 VFS 超级块，如果该文件系统的 VFS 超级块标志为“脏”，则必须
/// 将超级块信息写回磁盘。
/// TODO do_unmount
pub fn do_unmount(mount: Arc<VfsMount>, _flags: MountFlags) -> StrResult<()> {
    let mut global_mount_lock = GLOBAL_HASH_MOUNT.write();
    // 检查是否有子挂载点
    if !mount.access_inner().child.is_empty() {
        return Err("Have child");
    } else {
        let parent = mount.access_inner().parent.upgrade().unwrap();
        // 从父挂载点的子挂载点链表中删除
        parent
            .access_inner()
            .child
            .retain(|x| !Arc::ptr_eq(x, &mount));
    }
    // 从全局挂载点链表中删除
    global_mount_lock.retain(|x| !Arc::ptr_eq(x, &mount));
    Ok(())
}

pub fn mnt_want_write(mnt: &Arc<VfsMount>) -> bool {
    if mnt.flag.contains(MountFlags::MNT_READ_ONLY) {
        return false;
    }
    true
}
