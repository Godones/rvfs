use rvfs::file::{FileFlags, FileMode, vfs_mkdir, vfs_open_file};
use rvfs::{FakeFSC, init_vfs};
use rvfs::dentry::vfs_truncate;
use rvfs::stat::{vfs_getattr, vfs_getxattr, vfs_listxattr, vfs_removexattr, vfs_setxattr};

fn main() {
    env_logger::init();
    println!("init vfs");
    init_vfs();
    vfs_mkdir::<FakeFSC>("/tmp", FileMode::FMODE_WRITE).unwrap();
    vfs_setxattr::<FakeFSC>("/tmp", "type", "dir".as_bytes()).unwrap();
    vfs_setxattr::<FakeFSC>("/tmp", "target", "mount".as_bytes()).unwrap();
    let mut buf = [0u8; 20];
    let len = vfs_listxattr::<FakeFSC>("/tmp", &mut buf).unwrap();
    println!("len: {}", len);
    buf.split(|&x| x == 0)
        .collect::<Vec<&[u8]>>()
        .iter()
        .map(|&x| std::str::from_utf8(x).unwrap())
        .collect::<Vec<&str>>()
        .iter()
        .for_each(|x| {
            println!("attr: {}", x);
        });
    vfs_removexattr::<FakeFSC>("/tmp", "type").unwrap();
    let mut buf = [0u8; 20];
    let _len = vfs_listxattr::<FakeFSC>("/tmp", &mut buf).unwrap();

    buf.split(|&x| x == 0)
        .collect::<Vec<&[u8]>>()
        .iter()
        .map(|&x| std::str::from_utf8(x).unwrap())
        .collect::<Vec<&str>>()
        .iter()
        .for_each(|x| {
            println!("attr: {}", x);
        });
    let mut buf = [0u8; 20];
    let len = vfs_getxattr::<FakeFSC>("/tmp", "target", &mut buf).unwrap();
    let str = std::str::from_utf8(&buf[0..len]).unwrap();
    println!("target: {}", str);

    vfs_truncate::<FakeFSC>("/tmp", 10).is_err().then(|| {
        println!("truncate failed");
    });
    vfs_open_file::<FakeFSC>(
        "/tmp/f1",
        FileFlags::O_CREAT | FileFlags::O_RDWR,
        FileMode::FMODE_WRITE,
    )
    .unwrap();
    vfs_truncate::<FakeFSC>("/tmp/f1", 10).is_ok().then(|| {
        println!("truncate success");
    });
    let attr = vfs_getattr::<FakeFSC>("/tmp/f1").unwrap();
    println!("attr: {:#?}", attr);
}
