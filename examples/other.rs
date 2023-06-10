use rvfs::dentry::{Dirent64Iterator, LookUpFlags};
use rvfs::file::{vfs_mkdir, vfs_open_file, vfs_readdir, File, FileMode, OpenFlags};
use rvfs::mount::{do_mount, MountFlags};
use rvfs::path::{vfs_lookup_path, ParsePathType};
use rvfs::ramfs::tmpfs::TMP_FS_TYPE;
use rvfs::stat::KStat;
use rvfs::superblock::register_filesystem;
use rvfs::{init_process_info, mount_rootfs, FakeFSC};
use std::mem::size_of;
use std::sync::Arc;

fn main() {
    env_logger::init();
    let mnt = mount_rootfs();
    init_process_info(mnt);
    vfs_mkdir::<FakeFSC>("/fs", FileMode::FMODE_WRITE).unwrap();
    vfs_mkdir::<FakeFSC>("/fs/tmpfs", FileMode::FMODE_WRITE).unwrap();
    let file = vfs_open_file::<FakeFSC>("/fs/", OpenFlags::O_RDWR, FileMode::FMODE_WRITE).unwrap();

    readdir(file);

    register_filesystem(TMP_FS_TYPE).unwrap();
    println!("register tmpfs ok ......");
    println!("test do_mount");
    let tmpfs =
        do_mount::<FakeFSC>("", "/fs/tmpfs", "tmpfs", MountFlags::MNT_NO_DEV, None).unwrap();
    println!("mnt: {tmpfs:#?}");
    println!("test do_mount ok ......");
    let file = vfs_open_file::<FakeFSC>(
        "/fs/tmpfs/f1",
        OpenFlags::O_RDWR | OpenFlags::O_CREAT,
        FileMode::FMODE_WRITE,
    )
    .unwrap();
    println!("file: {file:#?}");
    let dentry = file.f_dentry.clone();
    let path = vfs_lookup_path(
        dentry,
        file.f_mnt.clone(),
        ParsePathType::Relative("./f2".to_string()),
        LookUpFlags::empty(),
    );
    println!("path: {path:#?}");

    let root = vfs_open_file::<FakeFSC>("/", OpenFlags::O_RDWR, FileMode::FMODE_WRITE).unwrap();

    readdir(root.clone());

    let root_dentry = root.f_dentry.clone();
    let path = vfs_lookup_path(
        root_dentry,
        root.f_mnt.clone(),
        ParsePathType::Relative("./fs/tmpfs/f1".to_string()),
        LookUpFlags::empty(),
    );
    println!("path: {path:#?}");

    // let stat = vfs_getattr::<FakeFSC>("/fs/tmpfs/f1").unwrap();
    // println!("stat: {:#?}", stat);

    println!("size of kstat: {}", size_of::<KStat>())
}

fn readdir(dir: Arc<File>) {
    let len = vfs_readdir(dir.clone(), &mut [0; 0]).unwrap();
    assert!(len > 0);
    let mut dirents = vec![0u8; len];

    let r = vfs_readdir(dir, &mut dirents[..]).unwrap();
    assert_eq!(r, len);
    Dirent64Iterator::new(&dirents[..]).for_each(|x| {
        println!("{} {:?} {}",x.get_name(),x.type_,x.ino);
    });
}
