use crate::inode::{Inode, InodeMode};
use crate::mount::VfsMount;
use alloc::string::{String, ToString};
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use bitflags::bitflags;
use core::ffi::c_char;
use core::fmt::{Debug, Formatter};
use spin::{Mutex, MutexGuard};
bitflags! {
    pub struct DirFlags:u32{
        const IN_HASH = 0x1;
    }
}
#[derive(Debug)]
pub struct DirEntry {
    pub d_flags: DirFlags,
    pub d_ops: DirEntryOps,
    inner: Mutex<DirEntryInner>,
}

#[derive(Debug)]
pub struct DirEntryInner {
    pub d_name: String,
    pub parent: Weak<DirEntry>,
    pub children: Vec<Arc<DirEntry>>,
    pub mount_count: u32,
    pub d_inode: Arc<Inode>,
}

impl DirEntry {
    pub fn empty() -> Self {
        DirEntry {
            d_flags: DirFlags::empty(),
            d_ops: DirEntryOps::empty(),
            inner: Mutex::new(DirEntryInner {
                d_name: String::new(),
                parent: Weak::new(),
                children: Vec::new(),
                mount_count: 0,
                d_inode: Arc::new(Inode::empty()),
            }),
        }
    }
    pub fn access_inner(&self) -> MutexGuard<DirEntryInner> {
        self.inner.lock()
    }
    pub fn with_inode_mode(mode: InodeMode) -> Self {
        let mut inode = Inode::empty();
        inode.mode = mode;
        DirEntry {
            d_flags: DirFlags::empty(),
            d_ops: DirEntryOps::empty(),
            inner: Mutex::new(DirEntryInner {
                d_name: String::new(),
                parent: Weak::new(),
                children: Vec::new(),
                mount_count: 0,
                d_inode: Arc::new(inode),
            }),
        }
    }
    pub fn new(
        d_flags: DirFlags,
        inode: Arc<Inode>,
        dir_ops: DirEntryOps,
        parent: Weak<DirEntry>,
        name: &str,
    ) -> Self {
        DirEntry {
            d_flags,
            d_ops: dir_ops,
            inner: Mutex::new(DirEntryInner {
                d_name: name.to_string(),
                parent,
                children: vec![],
                mount_count: 0,
                d_inode: inode,
            }),
        }
    }
    pub fn insert_child(&self, child: Arc<DirEntry>) {
        self.access_inner().children.push(child);
    }
    pub fn remove_child(&self, child_name: &str) {
        self.access_inner()
            .children
            .retain(|x| !x.access_inner().d_name.eq(child_name));
    }
    pub fn from_lookup_data(data: &LookUpData) -> Self {
        let parent = data.dentry.clone();
        DirEntry {
            d_flags: DirFlags::empty(),
            d_ops: DirEntryOps::empty(),
            inner: Mutex::new(DirEntryInner {
                parent: Arc::downgrade(&parent),
                d_name: data.last.clone(),
                children: vec![],
                mount_count: 0,
                d_inode: Arc::new(Inode::empty()),
            }),
        }
    }
}

unsafe impl Send for DirEntry {}
unsafe impl Sync for DirEntry {}

#[derive(Clone)]
pub struct DirEntryOps {
    pub d_hash: fn(dentry: Arc<DirEntry>, name: &str) -> usize,
    pub d_compare: fn(dentry: Arc<DirEntry>, name1: &str, name2: &str) -> bool,
    pub d_delete: fn(dentry: Arc<DirEntry>),
    /// 默认什么都不做
    pub d_release: fn(dentry: Arc<DirEntry>),
    /// 丢弃目录项对应的索引节点
    pub d_iput: fn(dentry: Arc<DirEntry>, inode: Arc<Inode>),
}

impl Debug for DirEntryOps {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DirEntryOps").finish()
    }
}

impl DirEntryOps {
    pub const fn empty() -> Self {
        DirEntryOps {
            d_hash: |_, _| 0,
            d_compare: |_, _, _| false,
            d_delete: |_| {},
            d_release: |_| {},
            d_iput: |_, _| {},
        }
    }
}

#[repr(C)]
pub struct Dirent64 {
    /// ino is an inode number
    pub ino: u64,
    /// off is an offset to next linux_dirent
    pub off: i64,
    /// reclen is the length of this linux_dirent
    pub reclen: u16,
    /// type is the file type
    pub type_: DirentType,
    /// name is the filename (null-terminated)
    pub name: [u8; 0],
}

pub struct Dirent64Iterator<'a> {
    pub buf: &'a [u8],
    pub pos: usize,
}

impl<'a> Dirent64Iterator<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Dirent64Iterator { buf, pos: 0 }
    }
}

impl<'a> Iterator for Dirent64Iterator<'a> {
    type Item = &'a Dirent64;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.buf.len() {
            return None;
        }
        let dirent = unsafe { &*(self.buf.as_ptr().add(self.pos) as *const Dirent64) };
        self.pos += dirent.len();
        Some(dirent)
    }
}

impl Debug for Dirent64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let name = self.name.as_ptr();
        unsafe {
            let name = core::ffi::CStr::from_ptr(name as *const c_char)
                .to_str()
                .unwrap();
            f.debug_struct("Dirent64")
                .field("ino", &self.ino)
                .field("off", &self.off)
                .field("reclen", &self.reclen)
                .field("type", &self.type_)
                .field("name", &name)
                .finish()
        }
    }
}

impl Dirent64 {
    pub fn new(name: &str, ino: u64, off: i64, type_: DirentType) -> Self {
        let size = core::mem::size_of::<Self>() + name.len() + 1;
        // align to 8 bytes
        let size = (size + 7) & !7;
        Self {
            ino,
            off,
            reclen: size as u16,
            type_,
            name: [0; 0],
        }
    }
    pub fn get_name(&self) -> &str {
        unsafe {
            let name = self.name.as_ptr();
            let name = core::ffi::CStr::from_ptr(name as *const i8);
            name.to_str().unwrap()
        }
    }
    pub fn len(&self) -> usize {
        self.reclen as usize
    }
}

bitflags! {
    pub struct DirentType:u8{
        const DT_UNKNOWN = 0;
        const DT_FIFO = 1;
        const DT_CHR = 2;
        const DT_DIR = 4;
        const DT_BLK = 6;
        const DT_REG = 8;
        const DT_LNK = 10;
        const DT_SOCK = 12;
        const DT_WHT = 14;
    }
}

impl From<InodeMode> for DirentType {
    fn from(value: InodeMode) -> Self {
        match value {
            InodeMode::S_SYMLINK => DirentType::DT_LNK,
            InodeMode::S_DIR => DirentType::DT_DIR,
            InodeMode::S_FILE => DirentType::DT_REG,
            InodeMode::S_CHARDEV => DirentType::DT_CHR,
            InodeMode::S_BLKDEV => DirentType::DT_BLK,
            InodeMode::S_FIFO => DirentType::DT_FIFO,
            InodeMode::S_SOCK => DirentType::DT_SOCK,
            // InodeMode:: => DirentType::DT_WHT,
            _ => DirentType::DT_UNKNOWN,
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
    pub fn len(&self) -> usize {
        self.buf.len()
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
        const EMPTY = 0x4000;
    }
}

//

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
    pub dentry: Arc<DirEntry>,
    /// 已经安装的文件系统对象
    pub mnt: Arc<VfsMount>,
    /// 路径名最后一个分量的类型。如PATHTYPE_NORMAL
    pub path_type: PathType,
    /// 符号链接查找的嵌套深度
    pub nested_count: u32,
    /// 嵌套关联路径名数组。
    pub symlink_names: Vec<String>,
}

impl LookUpData {
    pub fn new(flags: LookUpFlags, dentry: Arc<DirEntry>, mnt: Arc<VfsMount>) -> Self {
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
    pub fn update_dentry(&mut self, dentry: Arc<DirEntry>) {
        self.dentry = dentry;
    }
    pub fn update_mnt(&mut self, mnt: Arc<VfsMount>) {
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
