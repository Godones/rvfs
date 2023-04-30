use rvfs::{FakeFSC, init_process_info, mount_rootfs};
use rvfs::dentry::Dirent64;
use rvfs::file::{FileMode, OpenFlags, vfs_mkdir, vfs_open_file, vfs_readdir1};
use rvfs::stat::{vfs_getattr_by_file};


fn main(){
    env_logger::init();
    let mnt = mount_rootfs();
    init_process_info(mnt);
    vfs_mkdir::<FakeFSC>("/fs", FileMode::FMODE_WRITE).unwrap();
    vfs_open_file::<FakeFSC>("/f1", OpenFlags::O_RDWR|OpenFlags::O_CREAT, FileMode::FMODE_WRITE).unwrap();
    vfs_open_file::<FakeFSC>("/fddd", OpenFlags::O_RDWR|OpenFlags::O_CREAT, FileMode::FMODE_WRITE).unwrap();
    vfs_open_file::<FakeFSC>("/123123", OpenFlags::O_RDWR|OpenFlags::O_CREAT, FileMode::FMODE_WRITE).unwrap();

    let root = vfs_open_file::<FakeFSC>("/", OpenFlags::O_RDWR, FileMode::FMODE_WRITE).unwrap();

    let stat = vfs_getattr_by_file(root.clone()).unwrap();
    println!("stat: {:#?}", stat);
    let mut buf = vec![0u8; stat.st_size as usize];

    let mut len = vfs_readdir1(root.clone(), buf.as_mut_slice()).unwrap();
    println!("len: {}",len);
    let mut ptr = buf.as_ptr();
    loop {
        if len == 0 {
            break;
        }
        let dirent = unsafe {
            let dirent = &*(ptr as *const Dirent64);
            ptr = ptr.add(dirent.reclen as usize);
            dirent
         };
        println!("dirent: {:#?}",dirent);
        len -= dirent.reclen as usize;
    }
}