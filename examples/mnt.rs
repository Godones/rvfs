use rvfs::file::{vfs_open_file, FileFlags, FileMode};
use rvfs::{init_process_info, mount_rootfs, FakeFSC};

fn main() {
    env_logger::init();
    println!("init vfs");
    let rootfs = mount_rootfs();
    init_process_info(rootfs);
    let file = vfs_open_file::<FakeFSC>("/", FileFlags::O_RDWR, FileMode::FMODE_WRITE).unwrap();
    println!("file: {:#?}", file);
}
