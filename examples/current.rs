use std::sync::Arc;
use rvfs::{FakeFSC, init_process_info, mount_rootfs, PROCESS_FS_CONTEXT};
use rvfs::file::{FileMode, OpenFlags, vfs_mkdir, vfs_open_file, vfs_readdir};

fn main(){
    env_logger::init();
    println!("init vfs");
    let rootfs = mount_rootfs();
    init_process_info(rootfs);
    vfs_mkdir::<FakeFSC>("/tmp", FileMode::FMODE_WRITE).unwrap();
    let tmp = vfs_open_file::<FakeFSC>("/tmp", OpenFlags::O_RDWR, FileMode::FMODE_READ).unwrap();
    vfs_open_file::<FakeFSC>("/tmp/f1", OpenFlags::O_RDWR | OpenFlags::O_CREAT, FileMode::FMODE_WRITE).unwrap();
    vfs_open_file::<FakeFSC>("./tmp/f1", OpenFlags::O_RDWR | OpenFlags::O_CREAT, FileMode::FMODE_WRITE).unwrap();

    let a_txt = vfs_open_file::<FakeFSC>("./a.txt", OpenFlags::O_RDWR | OpenFlags::O_CREAT, FileMode::FMODE_WRITE).unwrap();

    let root = vfs_open_file::<FakeFSC>("/", OpenFlags::O_RDWR, FileMode::FMODE_WRITE).unwrap();
    vfs_readdir(root).unwrap().for_each(|x| {
        println!("name: {}", x);
    });
    // we set the cwd to /tmp
    PROCESS_FS_CONTEXT.lock().cwd = tmp.f_dentry.clone();
    let file = vfs_open_file::<FakeFSC>("f2", OpenFlags::O_RDWR|OpenFlags::O_CREAT, FileMode::FMODE_WRITE).unwrap();
    println!("file:{:#?}",file);

    // f1 and f2
    vfs_readdir(tmp).unwrap().for_each(|x| {
        println!("name: {}", x);
    });

    let file_ = vfs_open_file::<FakeFSC>("./f2", OpenFlags::O_RDWR, FileMode::FMODE_WRITE).unwrap();
    assert!(Arc::ptr_eq(&file,&file_));
    println!("------------------");
    let a_txt_ = vfs_open_file::<FakeFSC>("../a.txt", OpenFlags::O_RDWR, FileMode::FMODE_WRITE).unwrap();
    assert!(Arc::ptr_eq(&a_txt,&a_txt_));

    vfs_mkdir::<FakeFSC>("./dir", FileMode::FMODE_WRITE).unwrap();
    let dir = vfs_open_file::<FakeFSC>("./dir", OpenFlags::O_RDWR, FileMode::FMODE_WRITE).unwrap();

    PROCESS_FS_CONTEXT.lock().cwd = dir.f_dentry.clone();

    let a_txt__ = vfs_open_file::<FakeFSC>("../../a.txt", OpenFlags::O_RDWR, FileMode::FMODE_WRITE).unwrap();
    assert!(Arc::ptr_eq(&a_txt,&a_txt__));
}