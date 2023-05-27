use rvfs::file::{vfs_close_file, vfs_open_file, FileMode, OpenFlags};
use rvfs::{init_process_info, mount_rootfs, FakeFSC};

fn main() {
    env_logger::init();
    let mnt = mount_rootfs();
    init_process_info(mnt);
    let file = vfs_open_file::<FakeFSC>(
        "/f1",
        OpenFlags::O_CREAT | OpenFlags::O_RDWR,
        FileMode::FMODE_RDWR,
    )
    .unwrap();
    println!("file: {:#?}", file);
    vfs_close_file::<FakeFSC>(file).unwrap();
}
