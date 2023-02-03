use logger::{info, init_logger};
use rvfs::{init_vfs, open_file, path_walk, vfs_mkdir, FakeFSC, FileFlags, FileMode, LookUpFlags};

fn main() {
    color_backtrace::install();
    init_logger();
    init_vfs();
    let lookup_data = path_walk::<FakeFSC>("/", LookUpFlags::DIRECTORY).unwrap();
    println!("lookup_data: {:#?}", lookup_data);
    info!("create /tmp dir");
    vfs_mkdir::<FakeFSC>("/tmp", FileMode::FMODE_WRITE).unwrap();
    let _temp_find = path_walk::<FakeFSC>("/tmp", LookUpFlags::DIRECTORY).unwrap();
    // println!("temp_find: {:#?}",temp_find);
    println!("--------------------------------------");
    // let file = open_file::<FakeFSC>("/tmp", FileFlags::O_RDWR,FileMode::FMODE_READ).unwrap();
    // println!("file: {:#?}",file);
    let file = open_file::<FakeFSC>(
        "/f1",
        FileFlags::O_RDWR | FileFlags::O_CREAT,
        FileMode::FMODE_READ,
    )
    .unwrap();
    println!("file: {:#?}", file);
}
