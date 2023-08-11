use crate::dentry::DirEntry;
use crate::inode::{Inode, SpecialData};
use crate::mount::VfsMount;
use crate::StrResult;
use alloc::sync::Arc;
use bitflags::bitflags;
use core::fmt;
use core::fmt::{Debug, Formatter};
use spin::{Mutex, MutexGuard};

pub struct File {
    pub f_dentry: Arc<DirEntry>,
    pub f_mnt: Arc<VfsMount>,
    // 文件操作
    pub f_ops: FileOps,
    // 打开模式
    pub f_mode: FileMode,
    inner: Mutex<FileInner>,
}
#[derive(Debug)]
pub struct FileInner {
    pub flags: OpenFlags,
    // 文件偏移量
    pub f_pos: usize,
    pub f_uid: u32,
    pub f_gid: u32,
    pub f_ops_ext: FileExtOps,
}

impl Debug for File {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("File")
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

            f_mode: mode,
            inner: Mutex::new(FileInner {
                flags,
                f_pos: 0,
                f_uid: 0,
                f_gid: 0,
                f_ops_ext: FileExtOps::empty(),
            }),
        }
    }
    pub fn access_inner(&self) -> MutexGuard<FileInner> {
        self.inner.lock()
    }

    pub fn is_block_device(&self) -> bool {
        if let Some(SpecialData::BlockData(_x)) = self
            .f_dentry
            .access_inner()
            .d_inode
            .access_inner()
            .special_data
        {
            return true;
        }
        false
    }

    pub fn is_character_device(&self) -> bool {
        if let Some(SpecialData::CharData(_x)) = self
            .f_dentry
            .access_inner()
            .d_inode
            .access_inner()
            .special_data
        {
            return true;
        }
        false
    }

    pub fn is_pipe(&self) -> bool {
        if let Some(SpecialData::PipeData(_x)) = self
            .f_dentry
            .access_inner()
            .d_inode
            .access_inner()
            .special_data
        {
            return true;
        }
        false
    }

    pub fn is_socket(&self) -> bool{
        if let Some(SpecialData::Socket) = self
            .f_dentry
            .access_inner()
            .d_inode
            .access_inner()
            .special_data
        {
            return true;
        }
        false
    }
}

bitflags! {
    pub struct OpenFlags:u32{
        const O_RDONLY = 0x0;
        const O_WRONLY = 0x1;
        const O_RDWR = 0x2;
        const O_CREAT = 0x40;
        const O_EXCLUSIVE = 0x80;
        const O_NOCTTY = 0x100;
        const O_EXCL = 0x200;
        const O_APPEND = 0x400;
        const O_NONBLOCK = 0x800;
        const O_TRUNC = 0x1000; //?
        const O_TEXT = 0x4000;
        const O_BINARY = 0x8000;
        const O_DSYNC = 0x10000;
        const O_NOFOLLOW = 0x20000;
        const O_CLOSEEXEC = 0x80000;
        const O_DIRECTORY = 0x200000;
    }
}

/*
S_IRWXU  00700 user (file owner) has read, write, and
execute permission

S_IRUSR  00400 user has read permission

S_IWUSR  00200 user has write permission

S_IXUSR  00100 user has execute permission

S_IRWXG  00070 group has read, write, and execute
permission

S_IRGRP  00040 group has read permission

S_IWGRP  00020 group has write permission

S_IXGRP  00010 group has execute permission

S_IRWXO  00007 others have read, write, and execute
permission

S_IROTH  00004 others have read permission

S_IWOTH  00002 others have write permission

S_IXOTH  00001 others have execute permission

According to POSIX, the effect when other bits are set in
mode is unspecified.  On Linux, the following bits are
also honored in mode:

S_ISUID  0004000 set-user-ID bit

S_ISGID  0002000 set-group-ID bit (see inode(7)).

S_ISVTX  0001000 sticky bit (see inode(7)).*/
bitflags! {
    pub struct FileMode:u32{
        const FMODE_READ = 0x0;
        const FMODE_WRITE = 0x1;
        const FMODE_RDWR = 0x2;
        const FMODE_EXEC = 0x5; //read and execute
    }

    pub struct FileMode2:u32{
        const S_IRUSR = 0x00400;
        const S_IWUSR = 0x00200;
        const S_IXUSR = 0x00100;
        const S_IRWXU = 0x0070;
        const S_IRGRP = 0x00040;
        const S_IWGRP = 0x00020;
        const S_IXGRP = 0x00010;
        const S_IRWXG = 0x0007;
        const S_IROTH = 0x00004;
        const S_IWOTH = 0x00002;
        const S_IXOTH = 0x00001;
        const S_ISUID = 0x0004000;
        const S_ISGID = 0x0002000;
        const S_ISVTX = 0x0001000;
    }
}

impl Default for FileMode2 {
    fn default() -> Self {
        FileMode2::from_bits_truncate(0x600)
    }
}

impl From<FileMode2> for FileMode {
    fn from(value: FileMode2) -> Self {
        let mut file_mode = FileMode::empty();
        if value.contains(FileMode2::S_IRUSR) {
            file_mode |= FileMode::FMODE_READ;
        }
        if value.contains(FileMode2::S_IWUSR) {
            file_mode |= FileMode::FMODE_WRITE;
        }
        if value.contains(FileMode2::S_IXUSR) {
            file_mode |= FileMode::FMODE_EXEC;
        }
        file_mode
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

#[derive(Debug, Copy, Clone)]
pub enum SeekFrom {
    Start(u64),
    End(u64),
    Current(i64),
    Unknown,
}

impl From<(usize, usize)> for SeekFrom {
    fn from(value: (usize, usize)) -> Self {
        match value {
            (0, offset) => SeekFrom::Start(offset as u64),
            (1, offset) => SeekFrom::Current(offset as i64),
            (2, offset) => SeekFrom::End(offset as u64),
            _ => SeekFrom::Unknown,
        }
    }
}

#[derive(Debug)]
pub struct VmArea {
    pub vm_start: usize,
    pub vm_end: usize,
}

/// For poll
#[derive(Clone)]
pub struct FileExtOps {
    pub is_ready_read: fn(file: Arc<File>) -> bool,
    pub is_ready_write: fn(file: Arc<File>) -> bool,
    pub is_ready_exception: fn(file: Arc<File>) -> bool,
    pub is_hang_up: fn(file: Arc<File>) -> bool,
    pub ioctl:fn(file:Arc<File>,cmd:u32,arg:usize)->isize,
}
impl Debug for FileExtOps {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileExtOps")
            .field("is_ready_read", &"fn(file: Arc<File>) -> bool")
            .field("is_ready_write", &"fn(file: Arc<File>) -> bool")
            .field("is_ready_exception", &"fn(file: Arc<File>) -> bool")
            .field("is_hang_up", &"fn(file: Arc<File>) -> bool")
            .finish()
    }
}

impl FileExtOps {
    pub const fn empty() -> Self {
        FileExtOps {
            is_ready_read: |_| true,
            is_ready_write: |_| true,
            is_ready_exception: |_| false,
            is_hang_up: |_| false,
            ioctl: |_,_,_|-1,
        }
    }
}

#[derive(Clone)]
pub struct FileOps {
    pub llseek: fn(file: Arc<File>, whence: SeekFrom) -> StrResult<u64>,
    pub read: fn(file: Arc<File>, buf: &mut [u8], offset: u64) -> StrResult<usize>,
    pub write: fn(file: Arc<File>, buf: &[u8], offset: u64) -> StrResult<usize>,
    pub readdir: fn(file: Arc<File>, dirents: &mut [u8]) -> StrResult<usize>,
    /// 系统调用ioctl提供了一种执行设备特殊命令的方法(如格式化软盘的某个磁道，这既不是读也不是写操作)。
    /// 另外，内核还能识别一部分ioctl命令，而不必调用fops表中的ioctl。如果设备不提供ioctl入口点，
    /// 则对于任何内核未预先定义的请求，ioctl系统调用将返回错误(-ENOTYY)
    pub ioctl: fn(dentry: Arc<Inode>, file: Arc<File>, cmd: u32, arg: u64) -> StrResult<isize>,
    pub mmap: fn(file: Arc<File>, vma: VmArea) -> StrResult<()>,
    pub open: fn(file: Arc<File>) -> StrResult<()>,
    pub flush: fn(file: Arc<File>) -> StrResult<()>,
    /// 该方法是fsync系统调用的后端实现
    /// 用户调用它来刷新待处理的数据。
    /// 如果驱动程序没有实现这一方法，fsync系统调用将返回-EINVAL。
    pub fsync: fn(file: Arc<File>, datasync: bool) -> StrResult<()>,
    pub release: fn(file: Arc<File>) -> StrResult<()>,
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
            readdir: |_, _| Err("Not support"),
            ioctl: |_, _, _, _| Err("Not support"),
            mmap: |_, _| Err("Not support"),
            open: |_| Err("Not support"),
            flush: |_| Ok(()),
            fsync: |_, _| Ok(()),
            release: |_| Ok(()),
        }
    }
}
