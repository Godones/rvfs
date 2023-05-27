use rvfs::dentry::vfs_rename;
use rvfs::file::{
    vfs_mkdir, vfs_open_file, vfs_read_file, vfs_readdir, vfs_write_file, FileMode, OpenFlags,
};
use rvfs::{init_process_info, mount_rootfs, FakeFSC};

fn main() {
    env_logger::init();
    println!("init vfs");
    let rootfs = mount_rootfs();
    init_process_info(rootfs);
    let file1 = vfs_open_file::<FakeFSC>(
        "/file1",
        OpenFlags::O_CREAT | OpenFlags::O_RDWR,
        FileMode::FMODE_WRITE | FileMode::FMODE_READ,
    )
    .unwrap();
    let file2 = vfs_open_file::<FakeFSC>(
        "/file2",
        OpenFlags::O_CREAT | OpenFlags::O_RDWR,
        FileMode::FMODE_WRITE | FileMode::FMODE_READ,
    )
    .unwrap();
    println!("file1: {:#?}", file1);
    println!("file2: {:#?}", file2);
    vfs_write_file::<FakeFSC>(file1.clone(), b"hello", 0).unwrap();
    vfs_write_file::<FakeFSC>(file2.clone(), b"world", 0).unwrap();

    println!("--------------------rename /file1 to /file3----------------------");
    vfs_rename::<FakeFSC>("/file1", "/file3").unwrap();
    let root = vfs_open_file::<FakeFSC>("/", OpenFlags::O_RDONLY, FileMode::FMODE_READ).unwrap();
    // println!("root: {:#?}", root);
    vfs_readdir(root.clone())
        .unwrap()
        .into_iter()
        .for_each(|name| {
            println!("name: {}", name);
        });
    println!("--------------------rename /file2 to /file3----------------------");
    vfs_rename::<FakeFSC>("/file2", "/file3").unwrap();
    vfs_readdir(root.clone())
        .unwrap()
        .into_iter()
        .for_each(|name| {
            println!("name: {}", name);
        });
    println!("file2: {:#?}", file2);

    let mut buf = [0u8; 5];
    vfs_read_file::<FakeFSC>(file2, &mut buf, 0).unwrap();
    println!("buf: {:?}", core::str::from_utf8(&buf)); //"world"

    vfs_mkdir::<FakeFSC>("/tmp", FileMode::FMODE_WRITE).unwrap();
    let _file3 = vfs_open_file::<FakeFSC>(
        "/tmp/file3",
        OpenFlags::O_CREAT | OpenFlags::O_RDWR,
        FileMode::FMODE_WRITE | FileMode::FMODE_READ,
    );
    // println!("file3: {:#?}", file3);
    println!("--------------------rename /tmp to /tmptmp----------------------");
    vfs_rename::<FakeFSC>("/tmp", "/tmptmp").unwrap();

    vfs_readdir(root).unwrap().into_iter().for_each(|name| {
        println!("name: {}", name);
    });
    // println!("file3: {:#?}", file3);
    let tmp =
        vfs_open_file::<FakeFSC>("/tmptmp", OpenFlags::O_RDONLY, FileMode::FMODE_READ).unwrap();
    vfs_readdir(tmp).unwrap().into_iter().for_each(|name| {
        println!("name: {}", name);
    });
}
