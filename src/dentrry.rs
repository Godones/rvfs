use crate::inode::Inode;
use crate::{StrResult, VfsMount};
use alloc::string::{String, ToString};
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::num::FpCategory::Normal;
use core::ops::Neg;
use bitflags::bitflags;
use spin::Mutex;
bitflags! {
    pub struct DirFlags:u32{
        const IN_HASH = 0x1;
    }
}

const SHORT_FNAME_LEN: usize = 35;
pub struct DirEntry {
    pub d_flags: DirFlags,
    /// 指向一个inode对象
    pub d_inode: Arc<Mutex<Inode>>,
    /// 父节点
    pub parent: Weak<Mutex<DirEntry>>,
    pub d_ops: DirEntryOps,
    pub d_name: String,
    pub children: Vec<Arc<Mutex<DirEntry>>>,
    pub mount_count: u32,
    /// 短文件名
    pub short_name: [u8; SHORT_FNAME_LEN],
}
impl DirEntry {
    pub fn insert_child(&mut self, child: Arc<Mutex<DirEntry>>) {
        self.children.push(child);
    }
}

pub struct DirEntryOps {
    pub d_hash: fn(dentry: Arc<Mutex<DirEntry>>, name: &str) -> usize,
    pub d_compare: fn(dentry: Arc<Mutex<DirEntry>>, name: &str) -> bool,
    pub d_delete: fn(dentry: Arc<Mutex<DirEntry>>),
    /// 默认什么都不做
    pub d_release: fn(dentry: Arc<Mutex<DirEntry>>),
    /// 丢弃目录项对应的索引节点
    pub d_iput: fn(dentry: Arc<Mutex<DirEntry>>, inode: Arc<Mutex<Inode>>),
}

/// 进程需要提供的信息
///
/// 由于vfs模块与内核模块分离了，所以需要进程提供一些信息
pub struct ProcessFsInfo {
    pub root_mount: Arc<Mutex<VfsMount>>,
    pub root_dir: Arc<Mutex<DirEntry>>,
    pub current_dir: Arc<Mutex<DirEntry>>,
    pub current_mount: Arc<Mutex<VfsMount>>,
}
impl ProcessFsInfo {
    pub fn new(
        root_mount: Arc<Mutex<VfsMount>>,
        root_dir: Arc<Mutex<DirEntry>>,
        current_dir: Arc<Mutex<DirEntry>>,
        current_mount: Arc<Mutex<VfsMount>>,
    ) -> ProcessFsInfo {
        ProcessFsInfo {
            root_mount,
            root_dir,
            current_dir,
            current_mount,
        }
    }
}
/// 调用此函数时进程应该保证数据中间没有被修改
pub trait ProcessFs {
    fn get_fs_info() -> ProcessFsInfo;
}

bitflags! {
    pub struct LookUpFlags:u32{
        const READ_LINK = 1;
        const DIRECTORY = 2;
        const NOLAST = 3;
    }
}

bitflags! {
    pub struct PathType:u32{
        const PATH_ROOT = 0x1;
        const PATH_NORMAL = 0x2;
        const PATH_DOT = 0x3;
        const PATH_DOTDOT = 0x4;
    }
}
pub struct LookUpData {
    /// 查找标志
    pub flags: LookUpFlags,
    ///  查找到的目录对象
    pub dentry: Arc<Mutex<DirEntry>>,
    /// 已经安装的文件系统对象
    pub mnt: Arc<Mutex<VfsMount>>,
    /// 路径名最后一个分量的类型。如PATHTYPE_NORMAL
    pub path_type: PathType,
    /// 符号链接查找的嵌套深度
    pub nested_count: u32,
    /// 嵌套关联路径名数组。
    pub symlink_names: Vec<String>,
}

impl LookUpData {
    pub fn new(
        flags: LookUpFlags,
        dentry: Arc<Mutex<DirEntry>>,
        mnt: Arc<Mutex<VfsMount>>,
    ) -> Self {
        Self {
            flags,
            dentry,
            mnt,
            path_type: PathType::empty(),
            nested_count: 0,
            symlink_names: vec![],
        }
    }
    pub fn update_dentry(&mut self, dentry: Arc<Mutex<DirEntry>>) {
        self.dentry = dentry;
    }
    pub fn update_mnt(&mut self, mnt: Arc<Mutex<VfsMount>>) {
        self.mnt = mnt;
    }
    pub fn inc_nested_count(&mut self) {
        self.nested_count += 1;
    }
    pub fn dec_nested_count(&mut self) {
        self.nested_count -= 1;
    }
}

pub fn path_release(lookup_data: &LookUpData) {
    unimplemented!()
}

/// 加载目录项
pub fn path_walk<T: ProcessFs>(dir_name: &str, flags: LookUpFlags) -> StrResult<LookUpData> {
    let fs_info = T::get_fs_info();
    let (mnt, dentry) = if dir_name.starts_with("/") {
        (fs_info.root_mount.clone(), fs_info.root_dir.clone())
    } else {
        (fs_info.current_mount.clone(), fs_info.current_dir.clone())
    };
    let mut lookup_data = LookUpData::new(flags, dentry, mnt);
    __generic_load_dentry(dir_name, &mut lookup_data)?;
    Ok(lookup_data)
}

/// 路径查找
fn __generic_load_dentry(dir_name: &str, lookup_data: &mut LookUpData) -> StrResult<()> {
    let mut lookup_flags = lookup_data.flags;
    // 符号链接查找。
    if lookup_data.nested_count > 0 {
        lookup_flags = LookUpFlags::READ_LINK;
    }
    // resolve consecutive slashes
    let mut dir_name = dir_name;
    while dir_name.starts_with("/") {
        dir_name = &dir_name[1..];
    }
    // 如果是空字符串/根目录，直接返回
    if dir_name.is_empty() {
        lookup_data.path_type = PathType::PATH_ROOT;
        return Ok(());
    }
    // 获取当前路径的inode
    let mut inode = lookup_data.dentry.lock().d_inode.clone();
    // 循环处理每一个路径分量
    let mut old_path: String;

    let normal_func = |dir:&str|->StrResult<()> {
        // 不解析最后一个文件名
        if lookup_flags.contains(LookUpFlags::NOLAST) {
            lookup_data.path_type = PathType::PATH_NORMAL;
            if !dir.starts_with("."){
                // 如果最后一个分量不是"."或者".."，那么最后一个分量默认就是LAST_NORM
                return Err("");
            }
            if dir == "." {
                lookup_data.path_type = PathType::PATH_DOT;
            } else if dir == ".." {
                lookup_data.path_type = PathType::PATH_DOTDOT;
            }
            return Err("");
        }
        if dir == "." {
            return Ok(());
        }else if dir==".." {
            // 尝试回到父目录
            recede_parent(lookup_data.mnt.clone(),lookup_data.dentry.clone());
            inode = lookup_data.dentry.lock().d_inode.clone();
            return Ok(());
        }
        let (next_mnt,next_dentry) = find_file_indir(lookup_data, dir)?;
        advance_mount(next_mnt, next_dentry)?;

        Ok(())
    };
    let end_with_slashes = ||->StrResult<()>{
        // 文件名最后一个字符是"/
        // 因此必须解析符号链接，并要求最终指向目录
        lookup_flags |= LookUpFlags::READ_LINK|LookUpFlags::DIRECTORY;
        normal_func()?;
        Ok(())
    };

    loop {
        old_path = dir_name.to_string();
        // 获取下一个路径分量
        let (next_path, component) = get_next_path_component(dir_name);
        //最后一个分量，并且没有以"/"结束，是一个文件。
        dir_name = next_path;
        if dir_name.is_empty() {
            normal_func(component)?;
        }
        // 如果分量以"/"结束，退出。
        while dir_name.starts_with("/") {
            dir_name = &dir_name[1..];
        }
        if dir_name.is_empty() {
            end_with_slashes();
        }
        // 当前路径以"."开头
        if component.starts_with(".") && component.len() <= 2 {
            if component == "." {
                continue;
            } else if component == ".." {
                // 转到上级目录并继续。
                recede_parent(lookup_data.mnt.clone(), lookup_data.dentry.clone());
                inode = lookup_data.dentry.lock().d_inode.clone();
                continue;
            }
        }
        // 在当前目录中搜索下一个分量。
        let (next_mnt,next_dentry) = find_file_indir(lookup_data, component)?;
        // 向前推进到当前目录最后一个安装点
        advance_mount(next_mnt.clone(),next_dentry.clone())?;
        inode = next_dentry.lock().d_inode.clone();
        // 目录下不存在请求的文件
        // if inode.is_none() {
        //     return_errno!(ENOENT, "file not found");
        // }

        //不是目录也不是符号链接，但是后面还跟着
        if inode.lock().inode_ops.is_none() {
            return Err("file is not link or dir");
        }
        // 链接文件/普通目录对象/其它对象
        if inode.lock().inode_ops.as_ref().unwrap().follow_link.is_some(){
            advance_link(lookup_data, next_dentry)?;
            inode = lookup_data.dentry.lock().d_inode.clone();
            if inode.lock().inode_ops.is_none() {
                return Err("file is not link or dir");
            }
        }else if inode.lock().inode_ops.as_ref().unwrap().lookup.is_some() {
            lookup_data.mnt = next_mnt.clone();
            lookup_data.dentry = next_dentry.clone();
        }else {
            return Err("file is not link or dir");
        }
    }
    Ok(())
}

fn recede_parent(mnt: Arc<Mutex<VfsMount>>, dentry: Arc<Mutex<DirEntry>>) {
   unimplemented!()
}

fn find_file_indir(
    lookup_data: &mut LookUpData,
    name: &str,
) -> StrResult<(Arc<Mutex<VfsMount>>, Arc<Mutex<DirEntry>>)> {
    unimplemented!()
}

fn advance_mount(mnt:Arc<Mutex<VfsMount>>,next_dentry:Arc<Mutex<DirEntry>>)->StrResult<()>{
    unimplemented!()
}

/// 读取链接符号内容
fn advance_link(lookup_data: &mut LookUpData, next_dentry: Arc<Mutex<DirEntry>>) -> StrResult<()> {
    unimplemented!()
}


#[inline]
fn get_next_path_component(dir_name: &str) -> (&str, &str) {
    let mut next_path = "";
    let mut component = "";
    if let Some(index) = dir_name.find("/") {
        next_path = &dir_name[index..];
        component = &dir_name[..index];
    } else {
        component = dir_name;
    }
    (next_path, component)
}

#[cfg(test)]
mod tests {
    use crate::dentrry::get_next_path_component;

    #[test]
    fn test_get_next_path_component() {
        let (next_path, component) = get_next_path_component("a/b/c");
        assert_eq!(next_path, "/b/c");
        assert_eq!(component, "a");
        let (next_path, component) = get_next_path_component("a");
        assert_eq!(next_path, "");
        assert_eq!(component, "a");
        let (next_path, component) = get_next_path_component("a/");
        assert_eq!(next_path, "/");
        assert_eq!(component, "a");
        let (next_path, component) = get_next_path_component("a//");
        assert_eq!(next_path, "//");
        assert_eq!(component, "a");
        let (next_path, component) = get_next_path_component("./a/b/c/");
        assert_eq!(next_path, "/a/b/c/");
        assert_eq!(component, ".");
    }
}
