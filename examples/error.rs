use rvfs::dentry::Dirent64Iterator;
use rvfs::file::{vfs_mkdir, vfs_open_file, vfs_readdir, File, FileMode, OpenFlags};
use rvfs::stat::vfs_getattr_by_file;
use rvfs::{init_process_info, mount_rootfs, FakeFSC};
use std::sync::Arc;

fn main() {
    env_logger::init();
    let mnt = mount_rootfs();
    init_process_info(mnt);
    vfs_mkdir::<FakeFSC>("/fs", FileMode::FMODE_WRITE).unwrap();
    vfs_open_file::<FakeFSC>(
        "/f1",
        OpenFlags::O_RDWR | OpenFlags::O_CREAT,
        FileMode::FMODE_WRITE,
    )
    .unwrap();
    vfs_open_file::<FakeFSC>(
        "/fddd",
        OpenFlags::O_RDWR | OpenFlags::O_CREAT,
        FileMode::FMODE_WRITE,
    )
    .unwrap();
    vfs_open_file::<FakeFSC>(
        "/123123",
        OpenFlags::O_RDWR | OpenFlags::O_CREAT,
        FileMode::FMODE_WRITE,
    )
    .unwrap();

    let root = vfs_open_file::<FakeFSC>(".", OpenFlags::O_RDWR, FileMode::FMODE_WRITE).unwrap();

    let stat = vfs_getattr_by_file(root.clone()).unwrap();
    println!("stat: {stat:#?}");

    readdir(root);
}

fn readdir(dir: Arc<File>) {
    let len = vfs_readdir(dir.clone(), &mut [0; 0]).unwrap();
    assert!(len > 0);
    let mut dirents = vec![0u8; len];

    let r = vfs_readdir(dir, &mut dirents[..]).unwrap();
    assert_eq!(r, len);
    Dirent64Iterator::new(&dirents[..]).for_each(|x| {
        println!("{} {:?} {}", x.get_name(), x.type_, x.ino);
    });
}
