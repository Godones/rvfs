use alloc::sync::Arc;
use crate::dentry::DirEntry;
use crate::mount::VfsMount;


/// 进程需要提供的信息
///
/// 由于vfs模块与内核模块分离了，所以需要进程提供一些信息
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
