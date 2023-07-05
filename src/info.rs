use crate::dentry::DirEntry;
use crate::mount::VfsMount;
use alloc::string::String;
use alloc::sync::Arc;
use core::error::Error;
use core::fmt::{Display, Formatter};

pub type VfsResult<T> = Result<T, VfsError>;

pub const MAGIC_BASE: usize = 0x761203;

/// The information of the process's file system
pub struct ProcessFsInfo {
    pub root_mount: Arc<VfsMount>,
    pub root_dir: Arc<DirEntry>,
    pub current_dir: Arc<DirEntry>,
    pub current_mount: Arc<VfsMount>,
}
impl ProcessFsInfo {
    pub fn new(
        root_mount: Arc<VfsMount>,
        root_dir: Arc<DirEntry>,
        current_dir: Arc<DirEntry>,
        current_mount: Arc<VfsMount>,
    ) -> ProcessFsInfo {
        ProcessFsInfo {
            root_mount,
            root_dir,
            current_dir,
            current_mount,
        }
    }
}
pub trait ProcessFs {
    // 调用此函数时进程应该保证数据中间没有被修改
    fn get_fs_info() -> ProcessFsInfo;
    // 检查进程的链接文件嵌套查询深度是否超过最大值
    fn check_nested_link() -> bool;
    // 更新进程链接文件的相关数据： 嵌套查询深度/ 调用链接查找的次数
    fn update_link_data();
    fn max_link_count() -> u32;
    fn current_time() -> VfsTime;
}

#[derive(Default, Debug, Clone,Copy)]
#[repr(C)]
pub struct VfsTime {
    pub year: u32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl VfsTime {
    pub fn new(year: u32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> VfsTime {
        VfsTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }
}

#[derive(Debug)]
pub enum VfsError {
    DirNotFound,
    FileNotFound,
    FileAlreadyExist,
    DirNotEmpty,
    NotDir,
    DirAlreadyExist,
    NotFile,
    NotLink,
    LinkNotFound,
    LinkLoop,
    LinkDepthTooDeep,
    LinkCountTooMany,
    InvalidPath,
    NotImpl,
    FsTypeNotFound,
    MountInternal,
    DiskFsError(String),
    Other(String),
}

impl Display for VfsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            VfsError::DirNotFound => write!(f, "Directory not found"),
            VfsError::FileNotFound => write!(f, "File not found"),
            VfsError::FileAlreadyExist => write!(f, "File already exist"),
            VfsError::DirNotEmpty => write!(f, "Directory not empty"),
            VfsError::NotDir => write!(f, "Not a directory"),
            VfsError::DirAlreadyExist => write!(f, "Directory already exist"),
            VfsError::NotFile => write!(f, "Not a file"),
            VfsError::NotLink => write!(f, "Not a link"),
            VfsError::LinkNotFound => write!(f, "Link not found"),
            VfsError::LinkLoop => write!(f, "Link loop"),
            VfsError::LinkDepthTooDeep => write!(f, "Link depth too deep"),
            VfsError::LinkCountTooMany => write!(f, "Link count too many"),
            VfsError::InvalidPath => write!(f, "Invalid path"),
            VfsError::NotImpl => write!(f, "Not implemented"),
            VfsError::DiskFsError(msg) => write!(f, "Disk fs error: {msg}",),
            VfsError::FsTypeNotFound => write!(f, "File system type not found"),
            VfsError::MountInternal => write!(f, "Mount internal error"),
            VfsError::Other(msg) => write!(f, "Other error: {msg}",),
        }
    }
}

#[derive(Default, Debug)]
pub struct VfsTimeSpec {
    pub tv_sec: u64,
    pub tv_nsec: u64,
}

impl Error for VfsError {}
