use crate::{Inode, VfsMount};
use alloc::string::{String, ToString};
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use bitflags::bitflags;
use core::fmt::{Debug, Formatter};
use spin::Mutex;
bitflags! {
    pub struct DirFlags:u32{
        const IN_HASH = 0x1;
    }
}

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
}

impl Debug for DirEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DirEntry")
            .field("d_flags", &self.d_flags)
            .field("d_inode", &self.d_inode)
            .field("parent", &self.parent)
            .field("d_ops", &self.d_ops)
            .field("d_name", &self.d_name)
            .field("children", &self.children)
            .field("mount_count", &self.mount_count)
            .finish()
    }
}

impl DirEntry {
    pub fn empty() -> Self {
        DirEntry {
            d_flags: DirFlags::empty(),
            d_inode: Arc::new(Mutex::new(Inode::empty())),
            parent: Weak::new(),
            d_ops: DirEntryOps::empty(),
            d_name: String::new(),
            children: Vec::new(),
            mount_count: 0,
        }
    }
    pub fn new(inode: Arc<Mutex<Inode>>, parent: Weak<Mutex<DirEntry>>, name: &str) -> Self {
        DirEntry {
            d_flags: DirFlags::empty(),
            d_inode: inode,
            parent,
            d_ops: DirEntryOps::empty(),
            d_name: name.to_string(),
            children: Vec::new(),
            mount_count: 0,
        }
    }
    pub fn insert_child(&mut self, child: Arc<Mutex<DirEntry>>) {
        self.children.push(child);
    }
    pub fn remove_child(&mut self, child_name: &str) {
        self.children.retain(|x| !x.lock().d_name.eq(child_name));
    }
    pub fn from_lookup_data(data: &LookUpData) -> Self {
        let parent = data.dentry.clone();
        DirEntry {
            d_flags: DirFlags::empty(),
            d_inode: Arc::new(Mutex::new(Inode::empty())),
            parent: Arc::downgrade(&parent),
            d_ops: DirEntryOps::empty(),
            d_name: data.last.clone(),
            children: Vec::new(),
            mount_count: 0,
        }
    }
}

unsafe impl Send for DirEntry {}
unsafe impl Sync for DirEntry {}

#[derive(Clone)]
pub struct DirEntryOps {
    pub d_hash: fn(dentry: Arc<Mutex<DirEntry>>, name: &str) -> usize,
    pub d_compare: fn(dentry: Arc<Mutex<DirEntry>>, name1: &str, name2: &str) -> bool,
    pub d_delete: fn(dentry: Arc<Mutex<DirEntry>>),
    /// 默认什么都不做
    pub d_release: fn(dentry: Arc<Mutex<DirEntry>>),
    /// 丢弃目录项对应的索引节点
    pub d_iput: fn(dentry: Arc<Mutex<DirEntry>>, inode: Arc<Mutex<Inode>>),
}

impl Debug for DirEntryOps {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DirEntryOps").finish()
    }
}

impl DirEntryOps {
    fn empty() -> Self {
        DirEntryOps {
            d_hash: |_, _| 0,
            d_compare: |_, _, _| false,
            d_delete: |_| {},
            d_release: |_| {},
            d_iput: |_, _| {},
        }
    }
}

#[derive(Debug)]
pub struct DirContext {
    pub pos: usize,
    pub count: usize,
    pub buf: Vec<u8>,
}

impl DirContext {
    pub fn new(buf: Vec<u8>) -> Self {
        DirContext {
            pos: 0,
            count: 0,
            buf,
        }
    }
}

impl Iterator for DirContext {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.buf.len() {
            return None;
        }
        let mut i = self.pos;
        while i < self.buf.len() {
            if self.buf[i] == 0 {
                break;
            }
            i += 1;
        }
        let name = String::from_utf8_lossy(&self.buf[self.pos..i]).to_string();
        self.pos = i + 1;
        Some(name)
    }
}

bitflags! {
    pub struct LookUpFlags:u32{
        const READ_LINK = 0x1;
        const DIRECTORY = 0x2;
        const NOLAST = 0x4;
    }
}

bitflags! {
    pub struct PathType:u32{
        const PATH_ROOT = 0x1;
        const PATH_NORMAL = 0x2;
        const PATH_DOT = 0x4;
        const PATH_DOTDOT = 0x8;
    }
}
#[derive(Clone, Debug)]
pub struct LookUpData {
    pub last: String,
    // 文件名称
    pub name: String,
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
            last: "".to_string(),
            name: "".to_string(),
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

bitflags! {
    pub struct RenameFlag:u32{
        const RENAME_EXCHANGE = 0x1;
        const RENAME_NOREPLACE = 0x2;
    }
}
