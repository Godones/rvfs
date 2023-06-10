use rvfs::file::{vfs_open_file, FileMode, OpenFlags, vfs_mkdir};
use rvfs::{init_process_info, mount_rootfs, FakeFSC};
use rvfs::mount::{do_mount, MountFlags};
use rvfs::ramfs::tmpfs::TMP_FS_TYPE;
use rvfs::superblock::register_filesystem;

fn main() {
    env_logger::init();
    println!("init vfs");
    let rootfs = mount_rootfs();
    init_process_info(rootfs);
    let file = vfs_open_file::<FakeFSC>("/", OpenFlags::O_RDWR, FileMode::FMODE_WRITE).unwrap();
    println!("file: {file:#?}");
    vfs_mkdir::<FakeFSC>("/mnt0", FileMode::FMODE_WRITE).unwrap();
    vfs_mkdir::<FakeFSC>("./mnt1",FileMode::FMODE_WRITE).unwrap();
    register_filesystem(TMP_FS_TYPE).unwrap();
    let _tmpfs =  do_mount::<FakeFSC>("/dev/sda1", "/mnt0", "tmpfs", MountFlags::MNT_NO_DEV, None).unwrap();
    // println!("tmpfs: {tmpfs:#?}");

    let _same_tmpfs = do_mount::<FakeFSC>("/dev/sda1","/mnt1","tmpfs",MountFlags::MNT_NO_DEV,None).unwrap();
    // println!("same_tmpfs: {same_tmpfs:#?}"); // you can see the same_tmpfs and tmpfs have same superblock

    vfs_mkdir::<FakeFSC>("/mnt0/d1",FileMode::FMODE_WRITE).unwrap();
    // println!("same_tmpfs: {same_tmpfs:#?}"); // we mkdir in /mnt0/d1, but we can see the same_tmpfs have the same d1 dir


    vfs_mkdir::<FakeFSC>("./mnt1/d0",FileMode::FMODE_WRITE).unwrap();
    // println!("tmpfs: {tmpfs:#?}"); // we mkdir in ./mnt1/d0, but we can see the tmpfs have the same d0 dir


    vfs_mkdir::<FakeFSC>("./mnt2",FileMode::FMODE_WRITE).unwrap();

    let same_rootfs = do_mount::<FakeFSC>("root","/mnt2","rootfs",MountFlags::MNT_NO_DEV,None).unwrap();
    println!("same_rootfs: {same_rootfs:#?}"); // we can see the same_rootfs and rootfs have same superblock
}
