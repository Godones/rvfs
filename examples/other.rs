use rvfs::file::{
    vfs_mkdir, vfs_open_file, vfs_readdir, OpenFlags, FileMode,
};
use rvfs::superblock::{register_filesystem};
use rvfs::{init_process_info, mount_rootfs, FakeFSC};
use rvfs::dentry::LookUpFlags;
use rvfs::mount::{do_mount, MountFlags};
use rvfs::path::{ParsePathType, vfs_lookup_path};
use rvfs::ramfs::tmpfs::tmp_fs_type;

fn main() {
    env_logger::init();
    let mnt = mount_rootfs();
    init_process_info(mnt);
    vfs_mkdir::<FakeFSC>("/fs", FileMode::FMODE_WRITE).unwrap();
    vfs_mkdir::<FakeFSC>("/fs/tmpfs", FileMode::FMODE_WRITE).unwrap();
    let file = vfs_open_file::<FakeFSC>("/fs/", OpenFlags::O_RDWR, FileMode::FMODE_WRITE).unwrap();
    vfs_readdir(file).unwrap().into_iter().for_each(|name| {
        println!("name: {}", name);
    });
    register_filesystem(tmp_fs_type()).unwrap();
    println!("register tmpfs ok ......");
    println!("test do_mount");
    let tmpfs = do_mount::<FakeFSC>("", "/fs/tmpfs", "tmpfs", MountFlags::MNT_NO_DEV, None).unwrap();
    println!("mnt: {:#?}", tmpfs);
    println!("test do_mount ok ......");
    let file = vfs_open_file::<FakeFSC>("/fs/tmpfs/f1", OpenFlags::O_RDWR|OpenFlags::O_CREAT, FileMode::FMODE_WRITE).unwrap();
    println!("file: {:#?}", file);
    let dentry = file.f_dentry.clone();
    let path = vfs_lookup_path(dentry,file.f_mnt.clone(),ParsePathType::Relative("./f2".to_string()),LookUpFlags::empty());
    println!("path: {:#?}", path);

    let root = vfs_open_file::<FakeFSC>("/", OpenFlags::O_RDWR, FileMode::FMODE_WRITE).unwrap();
    vfs_readdir(root.clone()).unwrap().into_iter().for_each(|name| {
        println!("name: {}", name);
    });
    let root_dentry = root.f_dentry.clone();
    let path = vfs_lookup_path(root_dentry,root.f_mnt.clone(),ParsePathType::Relative("./fs/tmpfs/f1".to_string()),LookUpFlags::empty());
    println!("path: {:#?}", path);

    // let stat = vfs_getattr::<FakeFSC>("/fs/tmpfs/f1").unwrap();
    // println!("stat: {:#?}", stat);

}
