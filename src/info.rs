use alloc::string::{String, ToString};
use crate::dentry::DirEntry;
use crate::mount::VfsMount;
use alloc::sync::Arc;
use core::error::Error;
use core::fmt::{Display, Formatter};

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

#[derive(Default, Debug, Clone)]
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
pub enum VfsError{
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
    DiskFsError(String),
}
impl VfsError{
    pub fn to_string(&self) -> String {
        match self {
            VfsError::DirNotFound => "Directory not found".to_string(),
            VfsError::FileNotFound => "File not found".to_string(),
            VfsError::FileAlreadyExist => "File already exist".to_string(),
            VfsError::DirNotEmpty => "Directory not empty".to_string(),
            VfsError::NotDir => "Not a directory".to_string(),
            VfsError::DirAlreadyExist => "Directory already exist".to_string(),
            VfsError::NotFile => "Not a file".to_string(),
            VfsError::NotLink => "Not a link".to_string(),
            VfsError::LinkNotFound => "Link not found".to_string(),
            VfsError::LinkLoop => "Link loop".to_string(),
            VfsError::LinkDepthTooDeep => "Link depth too deep".to_string(),
            VfsError::LinkCountTooMany => "Link count too many".to_string(),
            VfsError::InvalidPath => "Invalid path".to_string(),
            VfsError::NotImpl => "Not implemented".to_string(),
            VfsError::DiskFsError(msg) => "Disk fs error: ".to_string() + msg,
        }
    }
}


impl Display for VfsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl Error for VfsError{}