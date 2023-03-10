mod define;
use crate::info::ProcessFs;
use crate::inode::{Inode, InodeFlags, InodeMode};
use crate::mount::{mnt_want_write, VfsMount};
use crate::{ddebug, StrResult, GLOBAL_HASH_MOUNT};
use alloc::string::ToString;
use alloc::sync::Arc;
pub use define::*;
use log::{debug, error};

/// 当删除物理文件时，释放缓存描述符的引用并将其从哈希表中删除
pub fn remove_dentry_cache(_dentry: Arc<DirEntry>) {
    unimplemented!()
}
/// 在卸载特殊文件系统时，删除所有的缓存节点
pub fn delete_all_dentry_cache(_root: Arc<DirEntry>) {
    unimplemented!()
}

/// 加载目录项
pub fn path_walk<T: ProcessFs>(dir_name: &str, flags: LookUpFlags) -> StrResult<LookUpData> {
    // 获取进程的文件系统信息
    ddebug!("path_walk");
    let fs_info = T::get_fs_info();
    // 如果是绝对路径，则从根目录开始查找
    let (mnt, dentry) = if dir_name.starts_with('/') {
        (fs_info.root_mount.clone(), fs_info.root_dir)
    } else {
        // 否则从当前目录开始查找
        (fs_info.current_mount.clone(), fs_info.current_dir)
    };
    // 初始化查找数据
    let mut lookup_data = LookUpData::new(flags, dentry, mnt);
    __generic_load_dentry::<T>(dir_name, &mut lookup_data)?;
    ddebug!("path_walk end");
    Ok(lookup_data)
}

/// 路径查找
fn __generic_load_dentry<T: ProcessFs>(
    dir_name: &str,
    lookup_data: &mut LookUpData,
) -> StrResult<()> {
    ddebug!("__generic_load_dentry");
    let mut lookup_flags = lookup_data.flags;
    // 是否正在进行符号链接查找
    if lookup_data.nested_count > 0 {
        lookup_flags = LookUpFlags::READ_LINK;
    }
    // resolve consecutive slashes
    let mut dir_name = dir_name;
    while dir_name.starts_with('/') {
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
    let mut inode = lookup_data.dentry.access_inner().d_inode.clone();
    // 循环处理每一个路径分量
    // 循环处理路径的每一个分量，但不处理最后一部分
    debug!("dir_name: {}", dir_name);
    loop {
        // 获取路径分量以及接下来的路径
        let (next_path, component) = get_next_path_component(dir_name);
        debug!("next_path: {}, component: {}", next_path, component);
        dir_name = next_path;
        lookup_data.name = component.to_string();
        //TODO 是否计算component的hash值
        //如果没有下一个分量，那么当前分量就是最后一个分量
        if dir_name.is_empty() {
            // 进入正常处理路径
            return __normal_load_dentry::<T>(lookup_data, lookup_flags, component, inode);
        }

        // 如果分量以"/"结束,说明也是路径的最后一个分量，但是此分量代表目录。
        while dir_name.starts_with('/') {
            dir_name = &dir_name[1..];
        }
        if dir_name.is_empty() {
            // 进入以"/"结尾的路径处理
            return __end_with_slashes::<T>(lookup_data, lookup_flags, component, inode);
        }

        // 当前路径以"."开头
        if component.starts_with('.') && component.len() <= 2 {
            if component == "." {
                continue;
            } else if component == ".." {
                // 转到上级目录并继续。
                recede_parent::<T>(&mut lookup_data.mnt, &mut lookup_data.dentry)?;

                inode = lookup_data.dentry.access_inner().d_inode.clone();
                continue;
            }
        }
        // 在当前目录中搜索下一个分量。
        debug!("try find {} in current dir", component);
        let (mut next_mnt, mut next_dentry) = find_file_indir(lookup_data, component)?;
        // TODO 向前推进到当前目录最后一个安装点
        // 查找得到的目录可能依次挂载了很多文件系统
        advance_mount(&mut next_mnt, &mut next_dentry)?;
        inode = next_dentry.access_inner().d_inode.clone();

        //不是目录也不是符号链接
        let inode_mode = inode.mode;

        match inode_mode {
            InodeMode::S_SYMLINK => {
                // 链接文件
                advance_link::<T>(lookup_data, next_dentry)?;
                inode = lookup_data.dentry.access_inner().d_inode.clone();
                // 如果链接文件没有指向目录，那么就不再继续循环
                if inode.mode != InodeMode::S_DIR {
                    return Err("file is not link or dir");
                }
            }
            InodeMode::S_DIR => {
                // 普通目录对象
                lookup_data.mnt = next_mnt.clone();
                lookup_data.dentry = next_dentry.clone();
            }
            _ => {
                // 普通文件
                return Err("file is not link or dir");
            }
        }
    }
}

/// 正常处理路径
fn __normal_load_dentry<T: ProcessFs>(
    lookup_data: &mut LookUpData,
    lookup_flags: LookUpFlags,
    dir: &str,
    inode: Arc<Inode>,
) -> StrResult<()> {
    ddebug!("__normal_load_dentry");
    // 不解析最后一个文件名
    if lookup_flags.contains(LookUpFlags::NOLAST) {
        lookup_data.path_type = PathType::PATH_NORMAL;
        lookup_data.last = dir.to_string();
        //TODO 保存最后一个分量
        if !dir.starts_with('.') {
            // 如果最后一个分量不是"."或者".."，那么最后一个分量默认就是LAST_NORM
            // 可以直接返回成功
            return Ok(());
        }
        if dir == "." {
            lookup_data.path_type = PathType::PATH_DOT;
        } else if dir == ".." {
            lookup_data.path_type = PathType::PATH_DOTDOT;
        }
        return Ok(());
    }
    // 处理. / ..两种特殊目录
    let mut inode = inode;
    if dir == "." {
        return Ok(());
    } else if dir == ".." {
        // 尝试回到父目录
        recede_parent::<T>(&mut lookup_data.mnt, &mut lookup_data.dentry)?;
        inode = lookup_data.dentry.access_inner().d_inode.clone();
        return Ok(());
    }
    // 在当前目录中搜索下一个分量。
    let (mut next_mnt, mut next_dentry) = find_file_indir(lookup_data, dir)?;

    debug!("find_file_indir ok");
    // TODO 向前推进到当前目录最后一个安装点
    advance_mount(&mut next_mnt, &mut next_dentry)?;

    // 如果是一个符号链接并且需要读取链接文件
    if lookup_flags.contains(LookUpFlags::READ_LINK) && inode.mode == InodeMode::S_SYMLINK {
        // 处理链接文件
        advance_link::<T>(lookup_data, next_dentry.clone())?;
        inode = lookup_data.dentry.access_inner().d_inode.clone();
    } else {
        // 普通目录对象
        debug!("普通目录对象");
        lookup_data.mnt = next_mnt;
        lookup_data.dentry = next_dentry;
    }
    // 要求最后一个文件必须是目录
    // 例如cd进入目录的情况，或者最后一个字符是/
    if lookup_flags.contains(LookUpFlags::DIRECTORY) && inode.mode != InodeMode::S_DIR {
        return Err("file is not dir");
    }
    Ok(())
}

/// 结尾含有"/"
fn __end_with_slashes<T: ProcessFs>(
    lookup_data: &mut LookUpData,
    lookup_flags: LookUpFlags,
    dir: &str,
    inode: Arc<Inode>,
) -> StrResult<()> {
    // 文件名最后一个字符是"/
    // 因此必须解析符号链接，并要求最终指向目录
    let lookup_flags = lookup_flags | LookUpFlags::READ_LINK | LookUpFlags::DIRECTORY;
    __normal_load_dentry::<T>(lookup_data, lookup_flags, dir, inode)
}

/// 回退到父目录
///
/// 需要注意的是，如果当前目录是一个安装点，那么需要回退到父目录的安装点
fn recede_parent<T: ProcessFs>(
    mnt: &mut Arc<VfsMount>,
    dentry: &mut Arc<DirEntry>,
) -> StrResult<()> {
    let mut t_mnt = mnt.clone();
    let mut t_dentry = dentry.clone();
    loop {
        // TODO 获取当前进程文件系统上下文的锁，防止线程修改根目录
        let process_fs = T::get_fs_info();
        // 如果当前目录是根目录，那么不需要回退
        if Arc::ptr_eq(&process_fs.root_dir, &t_dentry)
            && Arc::ptr_eq(&t_mnt, &process_fs.root_mount)
        {
            break;
        }
        // 如果当前目录不是所在文件系统的根目录，那么需要回退
        if !Arc::ptr_eq(&t_dentry, &t_mnt.root) {
            let parent = t_dentry.access_inner().parent.clone().upgrade().unwrap();
            t_dentry = parent;
            break;
        }
        let _global_mnt = GLOBAL_HASH_MOUNT.read();
        // 如果当前目录是文件系统的根目录，那么需要回退到父文件系统的根目录
        let parent_mnt = t_mnt.access_inner().parent.clone().upgrade();
        if parent_mnt.is_none() {
            // 说明到达顶级文件系统
            break;
        }
        // 获取挂载点
        t_dentry = t_mnt.access_inner().mount_point.clone();
        t_mnt = parent_mnt.unwrap();
    }
    // 处理父目录也是安装点的情况
    advance_mount(&mut t_mnt, &mut t_dentry)
}

/// 在当前目录中搜索指定文件
pub fn find_file_indir(
    lookup_data: &mut LookUpData,
    name: &str,
) -> StrResult<(Arc<VfsMount>, Arc<DirEntry>)> {
    ddebug!("find_file_indir");
    // 检查是否是在目录下查找
    if !lookup_data.dentry.access_inner().d_inode.mode == InodeMode::S_DIR {
        return Err("not a dir");
    }
    // 先在缓存中搜索，看看文件是否存在
    let mut dentry = __find_in_cache(lookup_data.dentry.clone(), name);
    // 在缓存中没有找到
    // 必须在块设备上找一找了
    if dentry.is_err() {
        // let inode = lookup_data.dentry.access_inner().d_inode.clone();
        // 获取文件节点锁
        // 调用文件系统的回调，从设备上装载文件节点
        dentry = __find_file_from_device(lookup_data, name);
        if dentry.is_err() {
            return Err("file not found");
        }
    }
    if dentry.is_err() {
        return Err("file not found");
    }
    ddebug!("find_file_indir end");
    Ok((lookup_data.mnt.clone(), dentry.unwrap()))
}

/// 在缓存中搜索文件
fn __find_in_cache(dentry: Arc<DirEntry>, name: &str) -> StrResult<Arc<DirEntry>> {
    // TODO 使用map保存而不是vec
    ddebug!("__find_in_cache");
    let _comp_func = dentry.d_ops.d_compare;
    for child in dentry.access_inner().children.iter() {
        let sub_name = child.access_inner().d_name.clone();
        debug!("find file in cache: {}", sub_name.as_str());
        // if comp_func
        // TODO deadlock in comp_func
        if sub_name.as_str() == name {
            debug!("find file in cache ok");
            return Ok(child.clone());
        }
    }
    ddebug!("__find_in_cache end");
    Err("file not found")
}
/*
 * 在目录中查找指定的文件
 * 如果文件不存在，在缓存中创建一个缓存项
 * 调用者必须持有目录锁
 */
fn __find_file_from_device(lookup_data: &mut LookUpData, name: &str) -> StrResult<Arc<DirEntry>> {
    ddebug!("__find_file_from_device");
    // 先在节点缓存中搜索
    let dentry = __find_in_cache(lookup_data.dentry.clone(), name);
    if dentry.is_ok() {
        return dentry;
    }
    // 缓存中不存在
    let inode = lookup_data.dentry.access_inner().d_inode.clone();
    let lookup_func = inode.inode_ops.lookup;

    let target_dentry = Arc::new(DirEntry::empty());
    // 设置dentry信息
    target_dentry.access_inner().d_name = name.to_string();
    target_dentry.access_inner().parent = Arc::downgrade(&lookup_data.dentry);
    let res = lookup_func(inode, target_dentry.clone());
    if res.is_err() {
        error!("lookup file from device error");
        return Err("file not found");
    }
    // 将新创建的dentry加入到父目录的子目录列表中
    lookup_data
        .dentry
        .access_inner()
        .children
        .push(target_dentry.clone());
    ddebug!("__find_file_from_device end");
    Ok(target_dentry)
}

/// 找到当前目录的最后一个挂载点
/// 并切换到该挂载点
pub fn advance_mount(mnt: &mut Arc<VfsMount>, next_dentry: &mut Arc<DirEntry>) -> StrResult<()> {
    ddebug!("advance_mount");
    let mut mount_count = next_dentry.access_inner().mount_count;
    let mut t_mnt = mnt.clone();
    let mut t_dentry = next_dentry.clone();
    // debug!("dentry:{:#?}", t_dentry);
    while mount_count > 0 {
        // 挂载点的根目录的mount_count必须大于0
        let child_mnt = lookup_mount(t_mnt.clone(), t_dentry.clone());
        if child_mnt.is_err() {
            break;
        }
        debug!("step into next mount point");
        t_mnt = child_mnt.unwrap();
        t_dentry = t_mnt.root.clone();
        mount_count = t_dentry.access_inner().mount_count;
    }
    *mnt = t_mnt;
    *next_dentry = t_dentry;
    ddebug!("advance_mount end");
    Ok(())
}

/// 在当前挂载点中查找子挂载点
fn lookup_mount(mnt: Arc<VfsMount>, next_dentry: Arc<DirEntry>) -> StrResult<Arc<VfsMount>> {
    let global_vfsmount_lock = GLOBAL_HASH_MOUNT.read();
    global_vfsmount_lock
        .iter()
        .find(|x| {
            let parent = x.access_inner().parent.upgrade();
            //  此挂载点的父挂载点是当前挂载点并且挂载点的根目录是参数指定
            parent.is_some()
                && Arc::ptr_eq(&parent.unwrap(), &mnt)
                && Arc::ptr_eq(&x.access_inner().mount_point, &next_dentry)
        })
        .ok_or("mount not found")
        .map(|x| x.clone())
}
/// 读取链接符号内容
/// * `dentry` - 源文件
/// * `lookup_data` - 查找数据
pub fn advance_link<T: ProcessFs>(
    lookup_data: &mut LookUpData,
    dentry: Arc<DirEntry>,
) -> StrResult<()> {
    // 进程需要检查嵌套层数
    if T::check_nested_link() {
        return Err("too many nested links");
    }
    lookup_data.nested_count += 1;
    __advance_link::<T>(lookup_data, dentry)?;
    lookup_data.nested_count -= 1;
    Ok(())
}
/// 符号链接查找，不考虑嵌套计数
fn __advance_link<T: ProcessFs>(
    lookup_data: &mut LookUpData,
    dentry: Arc<DirEntry>,
) -> StrResult<()> {
    let follow_link = dentry.access_inner().d_inode.inode_ops.follow_link;
    follow_link(dentry, lookup_data)?;
    let target_name = lookup_data.symlink_names.last().unwrap().clone();
    // 检查符号链接是否以'/'开头
    if target_name.starts_with('/') {
        // 是以'/'开头，已经找到一个绝对路径了
        // 因此没有必要保留前一个路径的任何信息,一切从头开始。
        let process_info = T::get_fs_info();
        lookup_data.dentry = process_info.current_dir.clone();
        lookup_data.mnt = process_info.current_mount;
    }
    __generic_load_dentry::<T>(&target_name, lookup_data)
}

#[inline]
fn get_next_path_component(dir_name: &str) -> (&str, &str) {
    let mut next_path = "";
    let component;
    if let Some(index) = dir_name.find('/') {
        next_path = &dir_name[index..];
        component = &dir_name[..index];
    } else {
        component = dir_name;
    }
    (next_path, component)
}

/// delete a directory
/// * `dir_name` - directory name
pub fn vfs_rmdir<T: ProcessFs>(dir_name: &str) -> StrResult<()> {
    ddebug!("vfs_rmdir");
    // find dir
    let lookup_data = path_walk::<T>(dir_name, LookUpFlags::DIRECTORY)?;
    match lookup_data.path_type {
        PathType::PATH_DOT => return Err("invalid path"),
        PathType::PATH_DOTDOT => return Err("not empty"),
        PathType::PATH_ROOT => return Err("it is root"),
        _ => {}
    }

    if !mnt_want_write(&lookup_data.mnt) {
        return Err("read only file system");
    }
    debug!("mnt is writable");
    let dentry = lookup_data.dentry;
    let parent = dentry.access_inner().parent.upgrade().unwrap();
    let parent_inode = parent.access_inner().d_inode.clone();
    may_delete(parent_inode.clone(), dentry.clone(), true)?;

    // mount point
    let mount = dentry.access_inner().mount_count;
    if mount > 0 {
        return Err("can't remove mount point");
    }
    // ensure dir is empty
    let inode = dentry.access_inner().d_inode.clone();
    let dir_size = inode.access_inner().file_size;
    if dir_size > 0 {
        return Err("directory not empty");
    }

    let rmdir = parent_inode.inode_ops.rmdir;
    // remove from parent dentry
    let name = dentry.access_inner().d_name.clone();
    parent.remove_child(name.as_str());
    // set inode with del flag
    rmdir(parent_inode, dentry.clone())?;
    inode.access_inner().flags = InodeFlags::S_DEL;
    ddebug!("vfs_rmdir end");
    Ok(())
}

/*
 *	Check whether we can remove a link victim from directory dir, check
 *  whether the type of victim is right.
 *  1. We can't do it if dir is read-only (done in permission())
 *  2. We should have write and exec permissions on dir
 *  3. We can't remove anything from append-only dir
 *  4. We can't do anything with immutable dir (done in permission())
 *  5. If the sticky bit on dir is set we should either
 *	a. be owner of dir, or
 *	b. be owner of victim, or
 *	c. have CAP_FOWNER capability
 *  6. If the victim is append-only or immutable we can't do antyhing with
 *     links pointing to it.
 *  7. If we were asked to remove a directory and victim isn't one - ENOTDIR.
 *  8. If we were asked to remove a non-directory and victim isn't one - EISDIR.
 *  9. We can't remove a root or mountpoint.
 * 10. We don't allow removal of NFS sillyrenamed files; it's handled by
 *     nfs_async_unlink().
 */
/// check whether we can delete a find in dir
pub fn may_delete(dir: Arc<Inode>, dentry: Arc<DirEntry>, isdir: bool) -> StrResult<()> {
    ddebug!("may_delete");
    let mode = dir.mode;
    if mode != InodeMode::S_DIR {
        return Err("not a directory");
    }
    if dentry.access_inner().d_inode.access_inner().flags == InodeFlags::S_INVALID {
        return Err("invalid dir");
    }
    if isdir {
        if dentry.access_inner().d_inode.mode != InodeMode::S_DIR {
            return Err("not a directory");
        }
        // root
        let parent = dentry.access_inner().parent.upgrade().unwrap();
        if Arc::ptr_eq(&parent, &dentry) {
            return Err("can't remove root directory");
        }
    } else if dentry.access_inner().d_inode.mode == InodeMode::S_DIR {
        return Err("is a directory");
    }
    ddebug!("may_delete end");
    Ok(())
}

pub fn may_create(dir: Arc<Inode>, dentry: Arc<DirEntry>) -> StrResult<()> {
    ddebug!("may_create");
    let mode = dir.mode;
    if mode != InodeMode::S_DIR {
        return Err("not a directory");
    }
    // root
    let parent = dentry.access_inner().parent.upgrade().unwrap();
    if Arc::ptr_eq(&parent, &dentry) {
        return Err("can't create root directory");
    }
    ddebug!("may_create end");
    Ok(())
}
/// truncate a file to a specified length
/// * `file_name` - file name
/// * `len` - length
pub fn vfs_truncate<T: ProcessFs>(file_name: &str, len: usize) -> StrResult<()> {
    ddebug!("vfs_truncate");
    let lookup_data = path_walk::<T>(file_name, LookUpFlags::empty())?;
    let inode = lookup_data.dentry.access_inner().d_inode.clone();
    if is_dir(inode.clone()) {
        return Err("is a directory");
    }
    if !mnt_want_write(&lookup_data.mnt) {
        return Err("read only file system");
    }
    // ignore permission

    // modify the inode file_size
    inode.access_inner().file_size = len;
    let truncate = inode.inode_ops.truncate;
    truncate(inode)?;
    ddebug!("vfs_truncate end");
    Ok(())
}

#[inline(always)]
pub fn is_dir(inode: Arc<Inode>) -> bool {
    inode.mode == InodeMode::S_DIR
}

/// rename a file
/// * `old_name` - old file name
/// * `new_name` - new file name
/// * `flag` - rename flag
/// # description
/// 1. old_name and new_name must be in the same file system
/// 2.
pub fn vfs_rename<T: ProcessFs>(old_name: &str, new_name: &str) -> StrResult<()> {
    ddebug!("vfs_rename");
    if old_name == "/" {
        return Err("can't rename root directory");
    }
    // parse name and get dentry
    let mut old_lookup_data = path_walk::<T>(old_name, LookUpFlags::NOLAST)?;
    let mut new_lookup_data = path_walk::<T>(new_name, LookUpFlags::NOLAST)?;

    let old_mnt = &old_lookup_data.mnt;
    let new_mnt = &new_lookup_data.mnt;
    // check if in the same file system
    if !Arc::ptr_eq(old_mnt, new_mnt) {
        return Err("not in the same file system");
    }
    let old_dentry = old_lookup_data.dentry.clone();
    let new_dentry = new_lookup_data.dentry.clone();
    if old_lookup_data.path_type != PathType::PATH_NORMAL
        || new_lookup_data.path_type != PathType::PATH_NORMAL
    {
        return Err("invalid path");
    }

    // find old file in parent dir
    let last = old_lookup_data.last.clone();
    let (_, old_sub_dentry) = find_file_indir(&mut old_lookup_data, &last)?;
    if Arc::ptr_eq(&old_sub_dentry, &old_dentry) {
        return Err("can't rename a file to itself");
    }

    if !is_dir(old_sub_dentry.access_inner().d_inode.clone()) {
        // if the file name ends with '/', it means we want to rename a directory
        // TODO(lookup_data should do)
    }
    let new_last = new_lookup_data.last.clone();
    error!("new last: {}", new_last);
    let res = find_file_indir(&mut new_lookup_data, &new_last);
    let new_sub_dentry = match res {
        Ok((_, sub_dentry)) => sub_dentry,
        Err(_) => {
            // a fake dentry
            debug!("make a fake dentry");
            let mut dentry = DirEntry::with_inode_mode(old_sub_dentry.access_inner().d_inode.mode);
            dentry.access_inner().d_name = new_lookup_data.last.clone();
            dentry.access_inner().parent = Arc::downgrade(&new_dentry);
            dentry.d_ops = old_sub_dentry.d_ops.clone();
            Arc::new(dentry)
        }
    };
    error!("path walk over");

    let old_inode = old_dentry.access_inner().d_inode.clone();
    // the old_dentry may be equal to new_dentry
    do_internal_rename(
        old_inode,
        old_sub_dentry.clone(),
        new_dentry.access_inner().d_inode.clone(),
        new_sub_dentry.clone(),
    )?;
    // after rename, the old dentry is invalid
    // so we need to update the old dentry
    old_sub_dentry.access_inner().d_name = new_sub_dentry.access_inner().d_name.clone();
    old_sub_dentry.access_inner().parent = new_sub_dentry.access_inner().parent.clone();
    ddebug!("vfs_rename end");
    Ok(())
}

fn do_internal_rename(
    old_dir: Arc<Inode>,
    old_dentry: Arc<DirEntry>,
    new_dir: Arc<Inode>,
    new_dentry: Arc<DirEntry>,
) -> StrResult<()> {
    ddebug!("do_internal_rename");
    let is_dir = is_dir(old_dentry.access_inner().d_inode.clone());
    let old_inode = old_dentry.access_inner().d_inode.clone();
    let new_inode = new_dentry.access_inner().d_inode.clone();
    if Arc::ptr_eq(&old_inode, &new_inode) {
        return Err("can't rename a file to itself");
    }

    debug!("old_dentry: {:?}", old_dentry.access_inner().d_name);
    may_delete(old_dir.clone(), old_dentry.clone(), is_dir)?;

    debug!("new_dentry: {:?}", new_dentry.access_inner().d_name);
    if new_dentry.access_inner().d_inode.access_inner().flags == InodeFlags::S_INVALID {
        // if the file doesn't exist, we need to create it
        may_create(new_dir.clone(), new_dentry.clone())?;
    } else {
        may_delete(new_dir.clone(), new_dentry.clone(), is_dir)?;
    }
    // rename
    if is_dir {
        vfs_rename_dir(old_dir, old_dentry, new_dir, new_dentry)?;
    } else {
        vfs_rename_other(old_dir, old_dentry, new_dir, new_dentry)?;
    }
    ddebug!("do_internal_rename end");
    Ok(())
}

fn vfs_rename_other(
    old_dir: Arc<Inode>,
    old_dentry: Arc<DirEntry>,
    new_dir: Arc<Inode>,
    new_dentry: Arc<DirEntry>,
) -> StrResult<()> {
    ddebug!("vfs_rename_other start");
    // do somthing that i dont know
    let rename = old_dir.inode_ops.rename;
    rename(old_dir, old_dentry, new_dir, new_dentry)?;
    ddebug!("vfs_rename_other end");
    Ok(())
}

fn vfs_rename_dir(
    old_dir: Arc<Inode>,
    old_dentry: Arc<DirEntry>,
    new_dir: Arc<Inode>,
    new_dentry: Arc<DirEntry>,
) -> StrResult<()> {
    ddebug!("vfs_rename_dir start");
    // do somthing that i dont know
    let rename = old_dir.inode_ops.rename;
    rename(old_dir, old_dentry, new_dir, new_dentry)?;
    ddebug!("vfs_rename_dir end");
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::dentry::get_next_path_component;

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
