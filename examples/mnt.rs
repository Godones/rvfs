use rvfs::file::{vfs_open_file, FileMode, OpenFlags};
use rvfs::{init_process_info, mount_rootfs, FakeFSC};

fn main() {
    env_logger::init();
    println!("init vfs");
    let rootfs = mount_rootfs();
    init_process_info(rootfs);
    let file = vfs_open_file::<FakeFSC>("/", OpenFlags::O_RDWR, FileMode::FMODE_WRITE).unwrap();
    println!("file: {:#?}", file);
}
