use crate::inode::{Inode, InodeMode};
use crate::{GLOBAL_HASH_MOUNT, StrResult, VfsMount};
use alloc::string::{String, ToString};
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
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
unsafe impl Send for DirEntry {}
unsafe impl Sync for DirEntry {}


impl DirEntry {
    pub fn insert_child(&mut self, child: Arc<Mutex<DirEntry>>) {
        self.children.push(child);
    }
}

pub struct DirEntryOps {
    pub d_hash: fn(dentry: Arc<Mutex<DirEntry>>, name: &str) -> usize,
    pub d_compare: fn(dentry: Arc<Mutex<DirEntry>>, name1: &str,name2:&str) -> bool,
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
pub trait ProcessFs {
    // 调用此函数时进程应该保证数据中间没有被修改
    fn get_fs_info() -> ProcessFsInfo;
    // 检查进程的链接文件嵌套查询深度是否超过最大值
    fn check_nested_link() -> bool;
    // 更新进程链接文件的相关数据： 嵌套查询深度/ 调用链接查找的次数
    fn update_link_data();
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
            name:"".to_string(),
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

pub fn rename_dentry<T:ProcessFs>(old_dentry: Arc<Mutex<DirEntry>>, new_dentry: Arc<Mutex<DirEntry>>) -> StrResult<()> {
    unimplemented!()
}

 /// 当删除物理文件时，释放缓存描述符的引用并将其从哈希表中删除
pub fn remove_dentry_cache(dentry: Arc<Mutex<DirEntry>>) {
    unimplemented!()
}
/// 在卸载特殊文件系统时，删除所有的缓存节点
pub fn delete_all_dentry_cache(root:Arc<Mutex<DirEntry>>) {
    unimplemented!()
}

/// 加载目录项
pub fn path_walk<T: ProcessFs>(dir_name: &str, flags: LookUpFlags) -> StrResult<LookUpData> {
    // 获取进程的文件系统信息
    let fs_info = T::get_fs_info();
    // 如果是绝对路径，则从根目录开始查找
    let (mnt, dentry) = if dir_name.starts_with("/") {
        (fs_info.root_mount.clone(), fs_info.root_dir.clone())
    } else {
        // 否则从当前目录开始查找
        (fs_info.current_mount.clone(), fs_info.current_dir.clone())
    };
    // 初始化查找数据
    let mut lookup_data = LookUpData::new(flags, dentry, mnt);
    __generic_load_dentry::<T>(dir_name, &mut lookup_data)?;
    Ok(lookup_data)
}

/// 路径查找
fn __generic_load_dentry<T:ProcessFs>(dir_name: &str, lookup_data: &mut LookUpData) -> StrResult<()> {
    let mut lookup_flags = lookup_data.flags;
    // 是否正在进行符号链接查找
    if lookup_data.nested_count > 0 {
        lookup_flags = LookUpFlags::READ_LINK;
    }
    // resolve consecutive slashes
    let mut dir_name = dir_name;
    while dir_name.starts_with("/") {
        dir_name = &dir_name[1..];
    }
    // 如果是空字符串/根目录，直接返回
    // 此时找到的是根目录
    if dir_name.is_empty() {
        lookup_data.path_type = PathType::PATH_ROOT;
        return Ok(());
    }
    // 获取当前路径的inode
    // 开始进一步查找
    let mut inode = lookup_data.dentry.lock().d_inode.clone();
    // 循环处理每一个路径分量
    // 循环处理路径的每一个分量，但不处理最后一部分
    loop {
        // 获取路径分量以及接下来的路径
        let (next_path, component) = get_next_path_component(dir_name);
        dir_name = next_path;
        lookup_data.name = component.to_string();
        //TODO是否计算component的hash值

        // 如果没有下一个分量，那么当前分量就是最后一个分量
        if dir_name.is_empty() {
            // 进入正常处理路径
            return __normal_load_dentry::<T>(lookup_data,lookup_flags,component,inode);
        }

        // 如果分量以"/"结束,说明也是路径的最后一个分量，但是此分量代表目录。
        while dir_name.starts_with("/") {
            dir_name = &dir_name[1..];
        }
        if dir_name.is_empty() {
            // 进入以"/"结尾的路径处理
            return __end_with_slashes::<T>(lookup_data,lookup_flags,component,inode);
        }

        // 当前路径以"."开头
        if component.starts_with(".") && component.len() <= 2 {
            if component == "." {
                continue;
            } else if component == ".." {
                // 转到上级目录并继续。
                recede_parent::<T>(&mut lookup_data.mnt, &mut lookup_data.dentry)?;

                inode = lookup_data.dentry.lock().d_inode.clone();
                continue;
            }
        }
        // 在当前目录中搜索下一个分量。
        let (mut next_mnt, mut next_dentry) = find_file_indir(lookup_data, component)?;
        // TODO 向前推进到当前目录最后一个安装点
        // 查找得到的目录可能依次挂载了很多文件系统
        advance_mount(&mut next_mnt,&mut next_dentry)?;
        inode = next_dentry.lock().d_inode.clone();

        //不是目录也不是符号链接
        let inode_mode = inode.lock().mode;

        match inode_mode {
            InodeMode::S_IFLNK => {
                // 链接文件
                advance_link::<T>(lookup_data, next_dentry)?;
                inode = lookup_data.dentry.lock().d_inode.clone();
                // 如果链接文件没有指向目录，那么就不再继续循环
                if inode.lock().mode != InodeMode::S_DIR {
                    return Err("file is not link or dir");
                }
            },
            InodeMode::S_DIR => {
                // 普通目录对象
                lookup_data.mnt = next_mnt.clone();
                lookup_data.dentry = next_dentry.clone();
            },
            _ => {
                // 普通文件
                return Err("file is not link or dir");
            }
        }
    }
}


/// 正常处理路径
fn __normal_load_dentry<T:ProcessFs>(lookup_data:&mut LookUpData,lookup_flags:LookUpFlags,dir:&str,inode:Arc<Mutex<Inode>>)->StrResult<()>{
    // 不解析最后一个文件名
    if lookup_flags.contains(LookUpFlags::NOLAST) {
        lookup_data.path_type = PathType::PATH_NORMAL;
        //TODO 保存最后一个分量
        if !dir.starts_with("."){
            // 如果最后一个分量不是"."或者".."，那么最后一个分量默认就是LAST_NORM
            // 可以直接返回成功
            return Ok(());
        }
        if dir == "." {
            lookup_data.path_type = PathType::PATH_DOT;
        } else if dir == ".." {
            lookup_data.path_type = PathType::PATH_DOTDOT;
        }
    }
    // 处理. / ..两种特殊目录
    let mut inode = inode;
    if dir == "." {
        return Ok(());
    }else if dir==".." {
        // 尝试回到父目录
        recede_parent::<T>(&mut lookup_data.mnt,&mut lookup_data.dentry)?;
        inode = lookup_data.dentry.lock().d_inode.clone();
        return Ok(());
    }
    // 在当前目录中搜索下一个分量。
    let (mut next_mnt, mut next_dentry) = find_file_indir(lookup_data, dir)?;
    // TODO 向前推进到当前目录最后一个安装点
    advance_mount(&mut next_mnt, &mut next_dentry)?;

    // 如果是一个符号链接并且需要读取链接文件
    if lookup_flags.contains(LookUpFlags::READ_LINK) && inode.lock().mode == InodeMode::S_IFLNK {
        // 处理链接文件
        advance_link::<T>(lookup_data, next_dentry.clone())?;
        inode = lookup_data.dentry.lock().d_inode.clone();
    } else {
        // 普通目录对象
        lookup_data.mnt = next_mnt;
        lookup_data.dentry = next_dentry;
    }
    // 要求最后一个文件必须是目录
    // 例如cd进入目录的情况，或者最后一个字符是/
    if lookup_flags.contains(LookUpFlags::DIRECTORY) && inode.lock().mode != InodeMode::S_DIR {
        return Err("file is not dir");
    }
    Ok(())
}


/// 结尾含有"/"
fn __end_with_slashes<T:ProcessFs>(lookup_data:&mut LookUpData, lookup_flags:LookUpFlags,dir:&str,inode:Arc<Mutex<Inode>>) ->StrResult<()>{
    // 文件名最后一个字符是"/
    // 因此必须解析符号链接，并要求最终指向目录
    let lookup_flags = lookup_flags | LookUpFlags::READ_LINK|LookUpFlags::DIRECTORY;
    __normal_load_dentry::<T>(lookup_data,lookup_flags,dir,inode)
}



/// 回退到父目录
///
/// 需要注意的是，如果当前目录是一个安装点，那么需要回退到父目录的安装点
fn recede_parent<T:ProcessFs>(mnt: &mut Arc<Mutex<VfsMount>>, dentry: &mut Arc<Mutex<DirEntry>>)
    ->StrResult<()>
{
    let mut t_mnt = mnt.clone();
    let mut t_dentry = dentry.clone();
    loop {
        // TODO 获取当前进程文件系统上下文的锁，防止线程修改根目录
        let process_fs = T::get_fs_info();
        // 如果当前目录是根目录，那么不需要回退
        if Arc::ptr_eq(&process_fs.root_dir,&t_dentry)&&
            Arc::ptr_eq(&t_mnt, &process_fs.root_mount){
            break;
        }
        // 如果当前目录不是所在文件系统的根目录，那么需要回退
        if !Arc::ptr_eq(&t_dentry, &t_mnt.lock().root){
            let parent =  t_dentry.lock().parent.clone().upgrade().unwrap();
            t_dentry = parent;
            break;
        }
        let _global_mnt  = GLOBAL_HASH_MOUNT.read();
        // 如果当前目录是文件系统的根目录，那么需要回退到父文件系统的根目录
        let parent_mnt = t_mnt.lock().parent.clone().upgrade();
        if parent_mnt.is_none(){
            // 说明到达顶级文件系统
            break;
        }
        // 获取挂载点
        t_dentry = t_mnt.lock().mount_point.clone();
        t_mnt = parent_mnt.unwrap();
    }
    // 处理父目录也是安装点的情况
    advance_mount(&mut t_mnt, &mut t_dentry)
}

/// 在当前目录中搜索指定文件
fn find_file_indir(
    lookup_data: &mut LookUpData,
    name: &str,
) -> StrResult<(Arc<Mutex<VfsMount>>, Arc<Mutex<DirEntry>>)> {
    // 先在缓存中搜索，看看文件是否存在
    let mut dentry = __find_in_cache(lookup_data.dentry.clone(), name);
    // 在缓存中没有找到
    // 必须在块设备上找一找了
    if dentry.is_err() {
        let inode = lookup_data.dentry.lock().d_inode.clone();
        // 获取文件节点锁
        let _inode_lock = inode.lock();
        // 调用文件系统的回调，从设备上装载文件节点
        dentry = __find_file_from_device(lookup_data,name);
        if dentry.is_err() {
            return Err("file not found");
        }
    }
    Ok((lookup_data.mnt.clone(), dentry.unwrap()))
}

/// 在缓存中搜索文件
/// TODO 使用map保存而不是vec
fn __find_in_cache(dentry: Arc<Mutex<DirEntry>>, name: &str) -> StrResult<Arc<Mutex<DirEntry>>> {
    let  dentry = dentry;
    let  dentry_lock = dentry.lock();
    let _comp_func = dentry_lock.d_ops.d_compare;
    for child in dentry_lock.children.iter() {
        let sub_name = child.lock().d_name.clone();
        // if comp_func
        // TODO deadlock in comp_func
        if sub_name.as_str() == name {
            return Ok(child.clone());
        }
    }
    Err("file not found")
}
/*
 * 在目录中查找指定的文件
 * 如果文件不存在，在缓存中创建一个缓存项
 * 调用者必须持有目录锁
 */
fn __find_file_from_device(lookup_data:&mut LookUpData,name:&str)->StrResult<Arc<Mutex<DirEntry>>>{
    // 先在节点缓存中搜索
    let dentry = __find_in_cache(lookup_data.dentry.clone(),name);
    if dentry.is_ok(){
        return dentry;
    }
    // 缓存中不存在
    let inode=  lookup_data.dentry.lock().d_inode.clone();
    let lookup_func = inode.lock().inode_ops.lookup;
    lookup_func(lookup_data.dentry.clone(),lookup_data)
}

/// 找到当前目录的最后一个挂载点
/// 并切换到该挂载点
fn advance_mount(mnt:&mut Arc<Mutex<VfsMount>>,next_dentry:&mut Arc<Mutex<DirEntry>>)->StrResult<()>{
    let mut mount_count = next_dentry.lock().mount_count;
    let mut t_mnt = mnt.clone();
    let mut t_dentry =next_dentry.clone();
    while mount_count > 0 {
        // 挂载点的根目录的mount_count必须大于0
        let child_mnt = lookup_mount(t_mnt.clone(),t_dentry.clone());
        if child_mnt.is_err(){break}
        t_mnt = child_mnt.unwrap();
        t_dentry = t_mnt.lock().root.clone();
        mount_count = t_dentry.lock().mount_count;
    }
    *mnt = t_mnt;
    *next_dentry = t_dentry;
    Ok(())
}

/// 在当前挂载点中查找子挂载点
fn lookup_mount(mnt:Arc<Mutex<VfsMount>>,next_dentry:Arc<Mutex<DirEntry>>)->StrResult<Arc<Mutex<VfsMount>>>{
    let global_vfsmount_lock = GLOBAL_HASH_MOUNT.read();
    global_vfsmount_lock.iter().find(|x| {
        let x = x.lock();
        let parent = x.parent.upgrade();
        //  此挂载点的父挂载点是当前挂载点并且挂载点的根目录是参数指定
        if parent.is_some() && Arc::ptr_eq(&parent.unwrap(),&mnt) && Arc::ptr_eq(&x.root,&next_dentry){
            true
        }else {
            false
        }
    }).ok_or("mount not found").map(|x|x.clone())
}
/// 读取链接符号内容
/// * `dentry` - 源文件
/// * `lookup_data` - 查找数据
fn advance_link<T:ProcessFs>(lookup_data: &mut LookUpData, dentry: Arc<Mutex<DirEntry>>) -> StrResult<()> {
    // 进程需要检查嵌套层数
    if T::check_nested_link(){
        return Err("too many nested links");
    }
    lookup_data.nested_count +=1;
    __advance_link::<T>(lookup_data,dentry)?;
    lookup_data.nested_count -=1;
    Ok(())
}
/// 符号链接查找，不考虑嵌套计数
fn __advance_link<T:ProcessFs>(lookup_data: &mut LookUpData, dentry: Arc<Mutex<DirEntry>>)->StrResult<()>{
    let follow_link = dentry.lock().d_inode.lock().inode_ops.follow_link;
    follow_link(dentry,lookup_data)?;
    let target_name = lookup_data.symlink_names.last().unwrap().clone();
    // 检查符号链接是否以'/'开头
    if target_name.starts_with("/") {
        // 是以'/'开头，已经找到一个绝对路径了
        // 因此没有必要保留前一个路径的任何信息,一切从头开始。
        let process_info = T::get_fs_info();
        lookup_data.dentry = process_info.current_dir.clone();
        lookup_data.mnt = process_info.current_mount.clone();
    }
    __generic_load_dentry::<T>(&target_name,lookup_data)
}

#[inline]
fn get_next_path_component(dir_name: &str) -> (&str, &str) {
    let mut next_path = "";
    let component ;
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
