use logger::{info, init_logger};
use rvfs::ramfs::tmpfs::tmp_fs_type;
use rvfs::{
    do_kernel_mount, init_vfs, path_walk, register_filesystem, vfs_mkdir, vfs_open_file,
    vfs_read_file, vfs_write_file, FakeFSC, FileFlags, FileMode, LookUpFlags, MountFlags,
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
    let file = vfs_open_file::<FakeFSC>(
        "/f1",
        FileFlags::O_RDWR | FileFlags::O_CREAT,
        FileMode::FMODE_WRITE,
    )
    .unwrap();
    println!("file: {:#?}", file);

    // test read / write
    let mut buf = [0u8; 10];
    vfs_write_file::<FakeFSC>(file.clone(), [1, 2, 3, 4, 5, 6, 7, 8, 9, 10].as_ref(), 0).unwrap();
    let _read = vfs_read_file::<FakeFSC>(file, buf.as_mut(), 0).unwrap();
    println!("read: {:?}", buf);

    // 注册tmpfs，实际上也是一个内存文件系统，但这里的实现将其与rootfs分开了
    println!("----------------------------------------");
    println!("register tmpfs ok....");
    register_filesystem(tmp_fs_type()).unwrap();
    let _mnt = do_kernel_mount("tmpfs", MountFlags::MNT_NO_DEV, "", None).unwrap();
}
