use logger::{info, init_logger};
use rvfs::{
    init_vfs, open_file, path_walk, read_file, vfs_mkdir, write_file, FakeFSC, FileFlags,
    FileMode, LookUpFlags,
};

fn main() {
    // color_backtrace::install();
    init_logger();
    init_vfs();
    let lookup_data = path_walk::<FakeFSC>("/", LookUpFlags::DIRECTORY).unwrap();
    println!("lookup_data: {:#?}", lookup_data);
    info!("create /tmp dir");
    vfs_mkdir::<FakeFSC>("/tmp", FileMode::FMODE_WRITE).unwrap();
    let _temp_find = path_walk::<FakeFSC>("/tmp", LookUpFlags::DIRECTORY).unwrap();
    // println!("temp_find: {:#?}",temp_find);
    println!("--------------------------------------");
    // open exist file
    // let file = open_file::<FakeFSC>("/tmp", FileFlags::O_RDWR,FileMode::FMODE_READ).unwrap();
    // println!("file: {:#?}",file);

    // open or create file
    let file = open_file::<FakeFSC>(
        "/f1",
        FileFlags::O_RDWR | FileFlags::O_CREAT,
        FileMode::FMODE_WRITE,
    )
    .unwrap();
    println!("file: {:#?}", file);

    // test mount
    // println!("--------------------------------------");
    // do_mount::<FakeFSC>("","/tmp", "ramfs", MountFlags::MNT_NO_DEV, None).unwrap();

    // test read / write
    let mut buf = [0u8; 10];
    write_file::<FakeFSC>(file.clone(), [1, 2, 3, 4, 5, 6, 7, 8, 9, 10].as_ref(), 0).unwrap();
    let _read = read_file::<FakeFSC>(file, buf.as_mut(), 0).unwrap();

    println!("read: {:?}", buf);
}
