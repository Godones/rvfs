use crate::info::ProcessFs;
use crate::{
    __advance_link, advance_mount, find_file_indir, path_walk, wwarn, DirContext, DirEntry, Inode,
    InodeMode, LookUpData, LookUpFlags, PathType, StrResult,
};
use alloc::sync::Arc;
use log::info;
use spin::Mutex;

mod define;
pub use define::*;
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

// pub fn vfs_open

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
        return Err("Can'dentry find father dir");
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

/// llseek
pub fn vfs_llseek(file: Arc<Mutex<File>>, offset: u64, whence: SeekFrom) -> StrResult<u64> {
    let llseek = file.lock().f_ops.llseek;
    llseek(file.clone(), offset, whence)
}

pub fn vfs_readdir(file: Arc<Mutex<File>>) -> StrResult<DirContext> {
    let readdir = file.lock().f_ops.readdir;
    readdir(file)
}

pub fn vfs_fsync(file: Arc<Mutex<File>>) -> StrResult<()> {
    // check file mode
    let mode = file.lock().f_mode;
    if !mode.contains(FileMode::FMODE_WRITE) {
        return Err("file not open for writing");
    }
    let fsync = file.lock().f_ops.fsync;
    fsync(file, true)
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
    wwarn!("construct_file end");
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
            return Err("Don'dentry solve mount file");
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
        .contains(InodeMode::S_SYMLINK)
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
