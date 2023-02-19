use rvfs::{init_vfs, vfs_link, vfs_mkdir, vfs_open_file, vfs_readdir, vfs_symlink, FakeFSC, FileFlags, FileMode, vfs_readlink};

fn main() {
    env_logger::init();
    println!("init vfs");
    init_vfs();
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
    let file =
        vfs_open_file::<FakeFSC>("/tmp/f2", FileFlags::O_RDWR, FileMode::FMODE_READ).unwrap();
    println!("file: {:#?}", file);

    vfs_symlink::<FakeFSC>("/tmp", "/tmp/f3").unwrap();
    println!("symlink ok ......");
    let file =
        vfs_open_file::<FakeFSC>("/tmp/f3", FileFlags::O_RDWR, FileMode::FMODE_READ).unwrap();
    println!("file: {:#?}", file);

    println!("--------------------------------------");
    let items = vfs_readdir(file0).unwrap();
    println!("items: {:?}", items);
    for name in items.into_iter(){
        println!("name: {}", name);
    }

    let mut buf = [0u8; 10];
    let size = vfs_readlink::<FakeFSC>("/tmp/f3",buf.as_mut()).unwrap();
    let target = std::str::from_utf8(&buf[0..size]).unwrap();
    println!("target: {}", target);

}
