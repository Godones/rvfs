use crate::dentry::{DirContext, DirEntry};
use crate::inode::Inode;
use crate::mount::VfsMount;
use crate::StrResult;
use alloc::sync::Arc;
use bitflags::bitflags;
use core::fmt::{Debug, Formatter};
use spin::{Mutex, MutexGuard};

pub struct File {
    pub f_dentry: Arc<DirEntry>,
    pub f_mnt: Arc<VfsMount>,
    // 文件操作
    pub f_ops: FileOps,
    pub flags: OpenFlags,
    // 打开模式
    pub f_mode: FileMode,
    inner: Mutex<FileInner>,
}
#[derive(Debug)]
pub struct FileInner {
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
            .field("f_dentry", &self.f_dentry)
            .field("inner", &self.inner)
            .finish()
    }
}

impl File {
    pub fn new(
        dentry: Arc<DirEntry>,
        mnt: Arc<VfsMount>,
        flags: OpenFlags,
        mode: FileMode,
        f_ops: FileOps,
    ) -> File {
        File {
            f_dentry: dentry,
            f_mnt: mnt,
            f_ops,
            flags,
            f_mode: mode,
            inner: Mutex::new(FileInner {
                f_pos: 0,
                f_uid: 0,
                f_gid: 0,
            }),
        }
    }
    pub fn access_inner(&self) -> MutexGuard<FileInner> {
        self.inner.lock()
    }
}

bitflags! {
    pub struct OpenFlags:u32{
        const O_RDONLY = 0;
        const O_WRONLY = 1;
        const O_RDWR = 2;
        const O_CREAT = 0100;
        const O_EXCL = 0200;
        const O_NOCTTY = 0400;
        const O_TRUNC = 01000;
        const O_APPEND = 02000;
        const O_NONBLOCK = 04000;
        const O_NOFOLLOW = 0400000;
        const O_DIRECTORY = 0200000;
    }
}
bitflags! {
    pub struct FileMode:u32{
        const FMODE_READ = 0x1;
        const FMODE_WRITE = 0x3;
        const FMODE_EXEC = 0x5; //read and execute
    }
}
impl From<&[u8]> for FileMode {
    fn from(value: &[u8]) -> Self {
        let mut mode = FileMode::empty();
        if value.contains(&b'r') {
            mode |= FileMode::FMODE_READ;
        }
        if value.contains(&b'w') {
            mode |= FileMode::FMODE_WRITE;
        }
        if value.contains(&b'x') {
            mode |= FileMode::FMODE_EXEC;
        }
        mode
    }
}

#[derive(Copy, Clone)]
pub enum SeekFrom {
    Start(u64),
    End(u64),
    Current(i64),
    Unknown
}

impl From<(usize,usize)> for SeekFrom{
    fn from(value: (usize, usize)) -> Self {
        match value {
            (0,offset) => SeekFrom::Start(offset as u64),
            (1,offset) => SeekFrom::Current(offset as i64),
            (2,offset) => SeekFrom::End(offset as u64),
            _ => SeekFrom::Unknown
        }
    }
}

#[derive(Debug)]
pub struct VmArea {
    pub vm_start: usize,
    pub vm_end: usize,
}

#[derive(Clone)]
pub struct FileOps {
    pub llseek: fn(file: Arc<File>, whence: SeekFrom) -> StrResult<u64>,
    pub read: fn(file: Arc<File>, buf: &mut [u8], offset: u64) -> StrResult<usize>,
    pub write: fn(file: Arc<File>, buf: &[u8], offset: u64) -> StrResult<usize>,
    // 对于设备文件来说，这个字段应该为NULL。它仅用于读取目录，只对文件系统有用。
    // filldir_t用于提取目录项的各个字段。
    pub readdir: fn(file: Arc<File>) -> StrResult<DirContext>,
    /// 系统调用ioctl提供了一种执行设备特殊命令的方法(如格式化软盘的某个磁道，这既不是读也不是写操作)。
    //另外，内核还能识别一部分ioctl命令，而不必调用fops表中的ioctl。如果设备不提供ioctl入口点，
    // 则对于任何内核未预先定义的请求，ioctl系统调用将返回错误(-ENOTYY)
    pub ioctl: fn(dentry: Arc<Inode>, file: Arc<File>, cmd: u32, arg: u64) -> StrResult<isize>,
    pub mmap: fn(file: Arc<File>, vma: VmArea) -> StrResult<()>,
    pub open: fn(file: Arc<File>) -> StrResult<()>,
    pub flush: fn(file: Arc<File>) -> StrResult<()>,
    /// 该方法是fsync系统调用的后端实现
    // 用户调用它来刷新待处理的数据。
    // 如果驱动程序没有实现这一方法，fsync系统调用将返回-EINVAL。
    pub fsync: fn(file: Arc<File>, datasync: bool) -> StrResult<()>,
}

impl Debug for FileOps {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FileOps").finish()
    }
}

impl FileOps {
    pub const fn empty() -> FileOps {
        FileOps {
            llseek: |_, _| Err("Not support"),
            read: |_, _, _| Err("Not support"),
            write: |_, _, _| Err("Not support"),
            readdir: |_| Err("Not support"),
            ioctl: |_, _, _, _| Err("Not support"),
            mmap: |_, _| Err("Not support"),
            open: |_| Err("Not support"),
            flush: |_| Err("Not support"),
            fsync: |_, _| Err("Not support"),
        }
    }
}
