use rvfs::dentry::vfs_rmdir;
use rvfs::file::{vfs_mkdir, vfs_open_file, FileMode, OpenFlags};
use rvfs::{init_process_info, mount_rootfs, FakeFSC};

fn main() {
    env_logger::init();
    println!("init vfs");
    let rootfs = mount_rootfs();
    init_process_info(rootfs);
    vfs_rmdir::<FakeFSC>("/")
        .is_err()
        .then(|| println!("rmdir / failed"));
    vfs_mkdir::<FakeFSC>("/tmp", FileMode::FMODE_WRITE)
        .is_ok()
        .then(|| println!("mkdir /tmp success"));
    vfs_rmdir::<FakeFSC>("/tmp").unwrap();
    vfs_open_file::<FakeFSC>("/tmp", OpenFlags::O_RDWR, FileMode::FMODE_WRITE)
        .is_err()
        .then(|| println!("open /tmp failed"));
    vfs_mkdir::<FakeFSC>("/tmp", FileMode::FMODE_WRITE)
        .is_ok()
        .then(|| println!("mkdir /tmp success"));
    vfs_open_file::<FakeFSC>(
        "/tmp/f1",
        OpenFlags::O_RDWR | OpenFlags::O_CREAT,
        FileMode::FMODE_WRITE,
    )
    .is_ok()
    .then(|| println!("create /tmp/f1 success"));
    vfs_rmdir::<FakeFSC>("/tmp")
        .is_err()
        .then(|| println!("rmdir /tmp failed,it is not empty"));
}
