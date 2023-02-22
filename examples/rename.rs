use rvfs::{init_vfs, vfs_open_file, vfs_rename, FakeFSC, FileFlags, FileMode};

fn main() {
    env_logger::init();
    println!("init vfs");
    init_vfs();
    let file1 = vfs_open_file::<FakeFSC>(
        "/file1",
        FileFlags::O_CREAT | FileFlags::O_RDWR,
        FileMode::FMODE_WRITE,
    )
    .unwrap();
    let file2 = vfs_open_file::<FakeFSC>(
        "/file2",
        FileFlags::O_CREAT | FileFlags::O_RDWR,
        FileMode::FMODE_WRITE,
    )
    .unwrap();
    println!("file1: {:#?}", file1);
    println!("file2: {:#?}", file2);
    println!("--------------------rename /file1 to /file3----------------------");
    vfs_rename::<FakeFSC>("/file1", "/file3").unwrap();
}
