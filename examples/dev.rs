use rvfs::dentry::{vfs_rmdir, Dirent64Iterator};
use rvfs::devfs::DEVFS_TYPE;
use rvfs::file::{
    vfs_llseek, vfs_mkdir, vfs_mknod, vfs_open_file, vfs_readdir, File, FileMode, OpenFlags,
    SeekFrom,
};
use rvfs::inode::InodeMode;
use rvfs::link::{vfs_readlink, vfs_symlink};
use rvfs::mount::{do_mount, MountFlags};
use rvfs::stat::{vfs_getattr, StatFlags};
use rvfs::superblock::register_filesystem;
use rvfs::{init_process_info, mount_rootfs, FakeFSC};
use std::sync::Arc;

fn main() {
    env_logger::init();
    let mnt = mount_rootfs();
    init_process_info(mnt);
    register_filesystem(DEVFS_TYPE).unwrap();
    vfs_mkdir::<FakeFSC>("/dev", FileMode::FMODE_RDWR).unwrap();
    let _dev_mnt =
        do_mount::<FakeFSC>("dev", "/dev", "devfs", MountFlags::MNT_NO_DEV, None).unwrap();
    // println!("dev_mnt: {dev_mnt:#?}");

    vfs_mkdir::<FakeFSC>("/dev/d0", FileMode::FMODE_RDWR).unwrap();
    vfs_mkdir::<FakeFSC>("/dev/d1", FileMode::FMODE_RDWR).unwrap();
    println!("test vfs_open_file");
    let dev = vfs_open_file::<FakeFSC>("/dev", OpenFlags::O_RDWR, FileMode::FMODE_RDWR).unwrap();
    // println!("dev: {dev:#?}");
    readdir(dev.clone());
    println!("test vfs_symlink");
    vfs_symlink::<FakeFSC>("/dev/d0", "/dev/d0s").unwrap();
    dev.access_inner().f_pos = 0;
    readdir(dev.clone());
    let len = vfs_readlink::<FakeFSC>("/dev/d0s", &mut [0; 0]).unwrap();
    assert!(len > 0);
    let mut buf = vec![0u8; len];
    let r = vfs_readlink::<FakeFSC>("/dev/d0s", &mut buf[..]).unwrap();
    assert_eq!(r, len);
    println!("readlink: {:?}", String::from_utf8(buf).unwrap());

    vfs_mknod::<FakeFSC>("./dev/tty", InodeMode::S_CHARDEV, FileMode::FMODE_RDWR, 9).unwrap();
    vfs_llseek(dev.clone(), SeekFrom::Start(0)).unwrap();
    readdir(dev.clone());

    let stat = vfs_getattr::<FakeFSC>("/dev/tty", StatFlags::empty()).unwrap();
    println!("stat: {stat:#?}");

    let stat = vfs_getattr::<FakeFSC>("/dev", StatFlags::empty()).unwrap();
    println!("stat: {stat:#?}");

    println!("test rmdir");
    vfs_rmdir::<FakeFSC>("/dev/d1").unwrap();
    vfs_llseek(dev.clone(), SeekFrom::Start(0)).unwrap();
    readdir(dev.clone());

    let stat = vfs_getattr::<FakeFSC>("/dev", StatFlags::empty()).unwrap();
    println!("stat: {stat:#?}");
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
