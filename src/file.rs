use crate::dentrry::{
    __advance_link, advance_mount, find_file_indir, path_walk, DirContext, DirEntry, LookUpData,
    LookUpFlags, PathType,
};
use crate::info::ProcessFs;
use crate::inode::{Inode, InodeMode};
use crate::{wwarn, StrResult, VfsMount};
use alloc::sync::Arc;
use bitflags::bitflags;
use core::fmt::{Debug, Formatter};
use log::info;
use spin::Mutex;

pub struct File {
    pub f_dentry: Arc<Mutex<DirEntry>>,
    // 含有该文件的已经安装的文件系统
    pub f_mnt: Arc<Mutex<VfsMount>>,
    // 文件操作
    pub f_ops: FileOps,
    pub flags: FileFlags,
    // 打开模式
    pub f_mode: FileMode,
    // 文件偏移量
    pub f_pos: usize,
    pub f_uid: u32,
    pub f_gid: u32,
}

impl Debug for File {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("File")
            .field("flags", &self.flags)
            .field("f_mode", &self.f_mode)
            .field("f_pos", &self.f_pos)
            .field("f_uid", &self.f_uid)
            .field("f_gid", &self.f_gid)
            .field("f_dentry", &self.f_dentry)
            .finish()
    }
}

impl File {
    pub fn new(
        dentry: Arc<Mutex<DirEntry>>,
        mnt: Arc<Mutex<VfsMount>>,
        flags: FileFlags,
        mode: FileMode,
        f_ops: FileOps,
    ) -> File {
        File {
            f_dentry: dentry,
            f_mnt: mnt,
            f_ops,
            flags,
            f_mode: mode,
            f_pos: 0,
            f_uid: 0,
            f_gid: 0,
        }
    }
}

bitflags! {
    pub struct FileFlags:u32{
        const O_RDONLY = 0x1;
        const O_WRONLY = 0x2;
        const O_RDWR = 0x3;
        const O_CREAT = 0x4;
        const O_EXCL = 0x8;
        const O_TRUNC = 0x10;
        const O_APPEND = 0x20;
        const O_DIRECTORY = 0x40;
        const O_NOFOLLOW = 0x80;
        const O_CLOEXEC = 0x100;
    }
}
bitflags! {
    pub struct FileMode:u32{
        const FMODE_READ = 0x1;
        const FMODE_WRITE = 0x2;
    }
}

#[derive(Debug)]
pub struct VmArea {
    pub vm_start: usize,
    pub vm_end: usize,
}

#[derive(Clone)]
pub struct FileOps {
    pub llseek: fn(file: Arc<Mutex<File>>, offset: u64, whence: usize) -> StrResult<u64>,
    pub read: fn(file: Arc<Mutex<File>>, buf: &mut [u8], offset: u64) -> StrResult<usize>,
    pub write: fn(file: Arc<Mutex<File>>, buf: &[u8], offset: u64) -> StrResult<usize>,
    // 对于设备文件来说，这个字段应该为NULL。它仅用于读取目录，只对文件系统有用。
    // filldir_t用于提取目录项的各个字段。
    // TODO readdir
    pub readdir: fn(file: Arc<Mutex<File>>) -> StrResult<DirContext>,
    /// 系统调用ioctl提供了一种执行设备特殊命令的方法(如格式化软盘的某个磁道，这既不是读也不是写操作)。
    //另外，内核还能识别一部分ioctl命令，而不必调用fops表中的ioctl。如果设备不提供ioctl入口点，
    // 则对于任何内核未预先定义的请求，ioctl系统调用将返回错误(-ENOTYY)
    pub ioctl: fn(
        dentry: Arc<Mutex<Inode>>,
        file: Arc<Mutex<File>>,
        cmd: u32,
        arg: u64,
    ) -> StrResult<isize>,
    pub mmap: fn(file: Arc<Mutex<File>>, vma: VmArea) -> StrResult<()>,
    pub open: fn(file: Arc<Mutex<File>>) -> StrResult<()>,
    pub flush: fn(file: Arc<Mutex<File>>) -> StrResult<()>,
    /// 该方法是fsync系统调用的后端实现
    // 用户调用它来刷新待处理的数据。
    // 如果驱动程序没有实现这一方法，fsync系统调用将返回-EINVAL。
    pub fsync: fn(file: Arc<Mutex<File>>, datasync: bool) -> StrResult<()>,
}

impl Debug for FileOps {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FileOps").finish()
    }
}

impl FileOps {
    pub const fn empty() -> FileOps {
        FileOps {
            llseek: |_, _, _| Err("NOT SUPPORT"),
            read: |_, _, _| Err("NOT SUPPORT"),
            write: |_, _, _| Err("NOT SUPPORT"),
            readdir: |_| Err("NOT SUPPORT"),
            ioctl: |_, _, _, _| Err("NOT SUPPORT"),
            mmap: |_, _| Err("NOT SUPPORT"),
            open: |_| Err("NOT SUPPORT"),
            flush: |_| Err("NOT SUPPORT"),
            fsync: |_, _| Err("NOT SUPPORT"),
        }
    }
}

/// 打开文件
/// * name:文件名
/// * flags: 访问模式
/// * mode: 创建文件读写权限
pub fn vfs_open_file<T: ProcessFs>(
    name: &str,
    flags: FileFlags,
    mode: FileMode,
) -> StrResult<Arc<Mutex<File>>> {
    wwarn!("open_file");
    let mut flags = flags;
    //  如果flag包含truncate标志，则将其转换为读写模式
    if flags.contains(FileFlags::O_TRUNC) {
        flags |= FileFlags::O_RDWR;
    }
    let mut lookup_data = open_dentry::<T>(name, flags, mode)?;
    construct_file(&mut lookup_data, flags, mode)
}

pub fn vfs_close_file<T: ProcessFs>(file: Arc<Mutex<File>>) -> StrResult<()> {
    // 调用文件的flush方法，只有少数驱动才会设置这个方法。
    let flush = file.lock().f_ops.flush;
    flush(file.clone())?;
    let t_file = file.lock();
    let sb = t_file.f_mnt.lock();
    let mut sb = sb.super_block.lock();
    sb.remove_file(file.clone());
    Ok(())
}

pub fn vfs_read_file<T: ProcessFs>(
    file: Arc<Mutex<File>>,
    buf: &mut [u8],
    offset: u64,
) -> StrResult<usize> {
    let mode = file.lock().f_mode;
    if !mode.contains(FileMode::FMODE_READ) {
        return Err("file not open for reading");
    }
    let read = file.lock().f_ops.read;

    read(file.clone(), buf, offset)
}

/// write file
pub fn vfs_write_file<T: ProcessFs>(
    file: Arc<Mutex<File>>,
    buf: &[u8],
    offset: u64,
) -> StrResult<usize> {
    let file_lock = file.lock();
    let write = file_lock.f_ops.write;
    let mode = file_lock.f_mode;
    if !mode.contains(FileMode::FMODE_WRITE) {
        return Err("file not open for writing");
    }
    drop(file_lock);

    write(file.clone(), buf, offset)
}

pub fn vfs_mkdir<T: ProcessFs>(name: &str, mode: FileMode) -> StrResult<()> {
    wwarn!("vfs_mkdir");
    let lookup_data = path_walk::<T>(name, LookUpFlags::NOLAST);
    if lookup_data.is_err() {
        return Err("Can't find father dir");
    }
    let mut lookup_data = lookup_data.unwrap();
    if lookup_data.path_type != PathType::PATH_NORMAL {
        return Err("It is not dir");
    }
    info!("find child dir");
    // 搜索子目录
    let last = lookup_data.last.clone();
    info!("last:{}", last);
    let inode = lookup_data.dentry.lock().d_inode.clone();
    let dentry = lookup_data.dentry.clone();
    let sub_dentry = find_file_indir(&mut lookup_data, &last);
    if sub_dentry.is_ok() {
        return Err("Dir exists");
    }
    info!("create new dir");
    // 调用函数创建一个新的目录
    let target_dentry = Arc::new(Mutex::new(DirEntry::empty()));
    let mkdir = inode.lock().inode_ops.mkdir;
    mkdir(inode, target_dentry.clone(), mode)?;
    // 设置目录名
    target_dentry.lock().d_name = last;
    // 设置父子关系
    target_dentry.lock().parent = Arc::downgrade(&dentry);
    dentry.lock().insert_child(target_dentry);
    // TODO dentry 插入全局链表
    Ok(())
}

pub fn generic_file_read(
    _file: Arc<Mutex<File>>,
    _buf: &mut [u8],
    _offset: u64,
) -> StrResult<usize> {
    // let inode = file.lock().f_dentry.lock().d_inode.clone();
    Ok(0)
}

pub fn generic_file_write(_file: Arc<Mutex<File>>, _buf: &[u8], _offset: u64) -> StrResult<usize> {
    Ok(0)
}

pub fn generic_file_readdir(_file: Arc<Mutex<File>>) -> StrResult<()> {
    Ok(())
}

pub fn generic_file_ioctl(_file: Arc<Mutex<File>>, _cmd: u32, _arg: u32) -> StrResult<()> {
    Ok(())
}
pub fn generic_file_mmap(_file: Arc<Mutex<File>>, _vma: VmArea) -> StrResult<()> {
    Ok(())
}

fn construct_file(
    lookup_data: &LookUpData,
    flags: FileFlags,
    mode: FileMode,
) -> StrResult<Arc<Mutex<File>>> {
    wwarn!("construct_file");
    let dentry = lookup_data.dentry.clone();
    let inode = dentry.lock().d_inode.clone();
    let f_ops = inode.lock().file_ops.clone();
    let open = f_ops.open;
    let file = File::new(dentry, lookup_data.mnt.clone(), flags, mode, f_ops);
    let file = Arc::new(Mutex::new(file));
    // TODO impl open in inodeops
    let res = open(file.clone());
    if res.is_err() {
        return Err(res.err().unwrap());
    }
    // 将文件放入超级块的文件表中
    let binding = lookup_data.mnt.lock();
    let mut sb = binding.super_block.lock();
    sb.insert_file(file.clone());
    Ok(file)
}

impl From<FileFlags> for LookUpFlags {
    fn from(val: FileFlags) -> Self {
        let mut flags = LookUpFlags::READ_LINK;
        if val.contains(FileFlags::O_DIRECTORY) {
            flags |= LookUpFlags::DIRECTORY;
        }
        if val.contains(FileFlags::O_NOFOLLOW) {
            flags -= LookUpFlags::READ_LINK;
        }
        if val.contains(FileFlags::O_EXCL | FileFlags::O_CREAT) {
            flags -= LookUpFlags::READ_LINK;
        }
        flags
    }
}
/// 创建目录项节点
/// 1. 只打开文件而不创建
/// 2. 查找文件所在父目录
/// 3. 在父目录创建文件
/// 4. 在父目录打开文件
/// 5. 处理链接文件
pub fn open_dentry<T: ProcessFs>(
    name: &str,
    flags: FileFlags,
    mode: FileMode,
) -> StrResult<LookUpData> {
    wwarn!("open_dentry");
    info!("{:?} -> {:?}", flags, Into::<LookUpFlags>::into(flags));

    // TODO 根据路径从缓存中直接查找
    // 只打开文件而不创建
    if !flags.contains(FileFlags::O_CREAT) {
        let res = path_walk::<T>(name, flags.into())?;
        //TODO 检查文件属性是否与参数冲突
        check_file_flags();
        return Ok(res);
    }
    // 查找文件所在父目录
    let mut lookup_data = path_walk::<T>(name, LookUpFlags::NOLAST)?;
    // 最后一个分量是目录。失败
    if lookup_data.path_type != PathType::PATH_NORMAL {
        return Err("open_direntry: last path component is a directory");
    }
    let dentry = lookup_data.dentry.clone();
    let inode = dentry.lock().d_inode.clone();
    lookup_data.flags -= LookUpFlags::NOLAST;
    // 获得父目录项
    let last = lookup_data.last.clone();
    info!("find father over, find child [{}] in dir", last);
    let mut find = find_file_indir(&mut lookup_data, &last).map(|x| x.1);
    // 识别最后一个分量
    let res = __recognize_last::<T>(&mut find, inode, flags, mode, &mut lookup_data);
    if res.is_ok() {
        Ok(lookup_data.clone())
    } else {
        Err(res.err().unwrap())
    }
}
fn __recognize_last<T: ProcessFs>(
    find: &mut Result<Arc<Mutex<DirEntry>>, &str>,
    inode: Arc<Mutex<Inode>>,
    flags: FileFlags,
    mode: FileMode,
    lookup_data: &mut LookUpData,
) -> StrResult<()> {
    wwarn!("__recognize_last");
    let mut count = 0usize;
    if find.is_err() {
        // 在父目录中创建文件
        // 调用文件系统的回调来创建真实的文件
        info!("create file in dir {}", lookup_data.dentry.lock().d_name);
        let create_func = inode.lock().inode_ops.create;
        let target_dentry = Arc::new(Mutex::new(DirEntry::empty()));
        create_func(inode.clone(), target_dentry.clone(), mode)?;
        // 设置dentry信息
        target_dentry.lock().d_name = lookup_data.last.clone();
        target_dentry.lock().parent = Arc::downgrade(&lookup_data.dentry);
        lookup_data
            .dentry
            .lock()
            .insert_child(target_dentry.clone());

        lookup_data.dentry = target_dentry;
        let mut flags = flags;
        flags -= FileFlags::O_TRUNC;
        check_file_flags();
        return Ok(());
    }
    // 文件存在
    // 如果包含O_EXCL，不能打开文件
    if flags.contains(FileFlags::O_EXCL) {
        return Err("flags contains O_EXCL");
    }
    // 是否挂载了文件系统
    let mut find_dentry = find.as_ref().unwrap().clone();
    if find_dentry.lock().mount_count > 0 {
        if flags.contains(FileFlags::O_NOFOLLOW) {
            return Err("Don't solve mount file");
        }
        advance_mount(&mut lookup_data.mnt, &mut find_dentry)?;
        lookup_data.dentry = find_dentry.clone();
    }
    // 处理链接文件
    if find_dentry
        .lock()
        .d_inode
        .lock()
        .mode
        .contains(InodeMode::S_IFLNK)
    {
        __solve_link_file::<T>(flags, mode, inode, lookup_data, &mut count)?;
    }
    // 文件为目录
    if find_dentry
        .lock()
        .d_inode
        .lock()
        .mode
        .contains(InodeMode::S_DIR)
    {
        return Err("open_direntry: file is a directory");
    }
    //TODO
    check_file_flags();
    // 设置正确结果
    lookup_data.dentry = find_dentry;
    wwarn!("__recognize_last over");
    Ok(())
}

fn __solve_link_file<T: ProcessFs>(
    flags: FileFlags,
    mode: FileMode,
    inode: Arc<Mutex<Inode>>,
    lookup_data: &mut LookUpData,
    count: &mut usize,
) -> StrResult<()> {
    if flags.contains(FileFlags::O_NOFOLLOW) {
        return Err("open_direntry: file is a symbolic link");
    }
    lookup_data.flags |= LookUpFlags::NOLAST;
    __advance_link::<T>(lookup_data, lookup_data.dentry.clone())?;
    lookup_data.flags -= LookUpFlags::NOLAST;
    if lookup_data.path_type != PathType::PATH_NORMAL {
        return Err("open_direntry: file is a directory");
    }
    if *count > T::max_link_count() as usize {
        return Err("open_direntry: too many symbolic links");
    }
    // 前面查找到父目录一级
    // 这里在父目录中查找最后一个文件
    let last = lookup_data.last.clone();
    let mut find = find_file_indir(lookup_data, &last).map(|x| x.1);
    // 识别最后一个分量
    __recognize_last::<T>(&mut find, inode, flags, mode, lookup_data)
}

fn check_file_flags() {}
