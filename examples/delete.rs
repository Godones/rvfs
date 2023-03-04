use rvfs::dentry::vfs_rmdir;
use rvfs::{FakeFSC, init_vfs};
use rvfs::file::{FileFlags, FileMode, vfs_mkdir, vfs_open_file};

fn main() {
    env_logger::init();
    println!("init vfs");
    init_vfs();
    vfs_rmdir::<FakeFSC>("/")
        .is_err()
        .then(|| println!("rmdir / failed"));
    vfs_mkdir::<FakeFSC>("/tmp", FileMode::FMODE_WRITE)
        .is_ok()
        .then(|| println!("mkdir /tmp success"));
    vfs_rmdir::<FakeFSC>("/tmp").unwrap();
    vfs_open_file::<FakeFSC>("/tmp", FileFlags::O_RDWR, FileMode::FMODE_WRITE)
        .is_err()
        .then(|| println!("open /tmp failed"));
    vfs_mkdir::<FakeFSC>("/tmp", FileMode::FMODE_WRITE)
        .is_ok()
        .then(|| println!("mkdir /tmp success"));
    vfs_open_file::<FakeFSC>(
        "/tmp/f1",
        FileFlags::O_RDWR | FileFlags::O_CREAT,
        FileMode::FMODE_WRITE,
    )
    .is_ok()
    .then(|| println!("create /tmp/f1 success"));
    vfs_rmdir::<FakeFSC>("/tmp")
        .is_err()
        .then(|| println!("rmdir /tmp failed,it is not empty"));
}
