use crate::dentry::{DirEntry, LookUpFlags};
use crate::inode::InodeMode;
use crate::mount::VfsMount;
use crate::StrResult;
use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::sync::Arc;

/// find the full path of the dentry
pub fn vfs_lookup_path(
    dentry: Arc<DirEntry>,
    mnt: Arc<VfsMount>,
    path: ParsePathType,
    _flag: LookUpFlags,
) -> StrResult<String> {
    // now we don't support the lookup flags
    let mut res = VecDeque::new();
    let mut current = dentry;
    let mut mnt = mnt;
    let is_dir = current.access_inner().d_inode.mode == InodeMode::S_DIR;
    // /f1/f2
    loop {
        let inner = current.access_inner();
        let parent = {
            if inner.d_name == "/" {
                // if we meet the mount point,we should recede it
                let mount_root = &mnt.root;
                assert!(Arc::ptr_eq(mount_root, &current));
                let mnt_point = mnt.access_inner().mount_point.clone();
                if !Arc::ptr_eq(&mnt_point, &current) {
                    let p_mnt = mnt.access_inner().parent.upgrade().unwrap();
                    mnt = p_mnt;
                    Some(mnt_point)
                } else {
                    if res.is_empty() {
                        res.push_back("/".to_string());
                    }
                    break;
                }
            } else {
                inner.parent.upgrade()
            }
        };
        let name = inner.d_name.clone();
        if name != "/" {
            res.push_back(name);
            res.push_back("/".to_string());
        }
        if parent.is_none() {
            break;
        }
        drop(inner);
        current = parent.unwrap();
    }
    // we ignore the first name
    if res.len() > 1 && !is_dir {
        res.pop_front();
    }

    if res.front().unwrap() != "/" {
        res.push_front("/".to_string());
    }

    let f_path = res.iter().rev().fold(String::new(), |mut acc, x| {
        acc.push_str(x);
        acc
    });
    // the path is relative
    assert!(path.is_relative());

    let res = stitching_path(f_path, path.path());
    if res.is_none() {
        return Err("path error");
    }
    Ok(res.unwrap())
}

/// we try to stitching path
/// # Example
/// * /bin/mytool/ + ../t1 == /bin/mytool/../t1 == /bin/t1
/// * /bin/mytool/ + ./t1 == /bin/mytool/t1
/// * /bin/mytool/ + t1 == /bin/mytool/t1
/// * /bin/mytool/ + ../../t1 == /bin/mytool/../../t1 == /t1
fn stitching_path(f_path: String, s_path: String) -> Option<String> {
    if let Some(stripped) = s_path.strip_prefix("./") {
        stitching_path(f_path, stripped.to_string())
    } else if let Some(stripped) = s_path.strip_prefix("../") {
        // find the index of the last / in f_path
        let index = f_path.rfind('/');
        index?;
        // index ==0 means the root,so we think it is error
        let index = index.unwrap();
        // find the second last /
        let index = f_path[..index].rfind('/');
        index?;
        let index = index.unwrap();
        // we get the new path
        let new_path = f_path[..=index].to_string();
        stitching_path(new_path, stripped.to_string())
    } else {
        return if s_path.eq(".") {
            // we think it is error
            Some(f_path)
        } else {
            // it is a relative path
            Some(f_path + s_path.as_str())
        };
    }
}

pub enum ParsePathType {
    // begin with ./ or ../ or other
    Relative(String),
    // begin with /
    Absolute(String),
}

impl ParsePathType {
    pub fn from<T: ToString>(value: T) -> Self {
        let path = value.to_string();
        if path.starts_with('/') {
            ParsePathType::Absolute(path)
        } else {
            ParsePathType::Relative(path)
        }
    }
    pub fn is_relative(&self) -> bool {
        matches!(self, ParsePathType::Relative(_))
    }
    pub fn is_absolute(&self) -> bool {
        matches!(self, ParsePathType::Absolute(_))
    }
    pub fn path(&self) -> String {
        match self {
            ParsePathType::Relative(p) => p.clone(),
            ParsePathType::Absolute(p) => p.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use alloc::string::ToString;

    #[test]
    fn test_stitching_path() {
        let f_path = "/bin/mytool/".to_string();
        let s_path = "../t1".to_string();
        let res = super::stitching_path(f_path, s_path);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), "/bin/t1".to_string());
    }
    #[test]
    fn test_stitching_path2() {
        let f_path = "/bin/mytool/".to_string();
        let s_path = "../../t1".to_string();
        let res = super::stitching_path(f_path, s_path);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), "/t1".to_string());
    }
    #[test]
    fn test_stitching_path3() {
        let f_path = "/bin/mytool/".to_string();
        let s_path = "./t1".to_string();
        let res = super::stitching_path(f_path, s_path);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), "/bin/mytool/t1".to_string());
    }
    #[test]
    fn test_stitching_path4() {
        let f_path = "/bin/mytool/".to_string();
        let s_path = "t1".to_string();
        let res = super::stitching_path(f_path, s_path);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), "/bin/mytool/t1".to_string());
    }

    #[test]
    fn test_stitching_path5() {
        let f_path = "/bin/mytool/".to_string();
        let s_path = ".".to_string();
        let res = super::stitching_path(f_path, s_path);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), "/bin/mytool/".to_string());
    }
}
