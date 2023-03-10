use rvfs::file::{vfs_mkdir, vfs_open_file, vfs_readdir, vfs_write_file, FileFlags, FileMode};
use rvfs::link::{vfs_link, vfs_readlink, vfs_symlink, vfs_unlink};
use rvfs::{init_process_info, mount_rootfs, FakeFSC};

fn main() {
    env_logger::init();
    println!("init vfs");
    let rootfs = mount_rootfs();
    init_process_info(rootfs);
    // let lookup_data = path_walk::<FakeFSC>("/", LookUpFlags::DIRECTORY).unwrap();
    // println!("lookup_data: {:#?}", lookup_data);
    println!("mkdir /tmp");
    vfs_mkdir::<FakeFSC>("/tmp", FileMode::FMODE_WRITE).unwrap();
    let file0 = vfs_open_file::<FakeFSC>("/tmp", FileFlags::O_RDWR, FileMode::FMODE_READ).unwrap();
    println!("file: {:#?}", file0);
    let file = vfs_open_file::<FakeFSC>(
        "/tmp/f1",
        FileFlags::O_RDWR | FileFlags::O_CREAT,
        FileMode::FMODE_READ | FileMode::FMODE_WRITE,
    )
    .unwrap();
    println!("file: {:#?}", file);
    vfs_link::<FakeFSC>("/tmp/f1", "/tmp/f2").unwrap();
    println!("link ok ......");
    let file_f2 = vfs_open_file::<FakeFSC>(
        "/tmp/f2",
        FileFlags::O_RDWR,
        FileMode::FMODE_READ | FileMode::FMODE_WRITE,
    )
    .unwrap();
    println!("file: {:#?}", file_f2);

    vfs_symlink::<FakeFSC>("/tmp", "/tmp/f3").unwrap();
    println!("symlink ok ......");
    let file =
        vfs_open_file::<FakeFSC>("/tmp/f3", FileFlags::O_RDWR, FileMode::FMODE_READ).unwrap();
    println!("file: {:#?}", file);

    println!("--------------------------------------");
    let items = vfs_readdir(file0).unwrap();
    println!("items: {:?}", items);
    for name in items.into_iter() {
        println!("name: {}", name);
    }

    let mut buf = [0u8; 10];
    let size = vfs_readlink::<FakeFSC>("/tmp/f3", buf.as_mut()).unwrap();
    let target = std::str::from_utf8(&buf[0..size]).unwrap();
    println!("target: {}", target);

    println!(
        "/tmp/f1 hard_links: {:#?}",
        file_f2
            .f_dentry
            .access_inner()
            .d_inode
            .access_inner()
            .hard_links
    );
    vfs_unlink::<FakeFSC>("/tmp/f1").unwrap();
    println!(
        "/tmp/f1 hard_links: {:#?}",
        file_f2
            .f_dentry
            .access_inner()
            .d_inode
            .access_inner()
            .hard_links
    );
    vfs_unlink::<FakeFSC>("/tmp/f2").unwrap();
    println!(
        "/tmp/f1 hard_links: {:#?}",
        file_f2
            .f_dentry
            .access_inner()
            .d_inode
            .access_inner()
            .hard_links
    );
    vfs_write_file::<FakeFSC>(file_f2, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10].as_ref(), 0)
        .is_err()
        .then(|| {
            println!("write file error");
        });
}
