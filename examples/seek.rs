use rvfs::{init_process_info, mount_rootfs, FakeFSC};
use rvfs::file::{FileMode, OpenFlags, SeekFrom, vfs_llseek, vfs_open_file, vfs_read_file, vfs_write_file};

fn main() {
    env_logger::init();
    println!("init vfs");
    let rootfs = mount_rootfs();
    init_process_info(rootfs);

    let file1 = vfs_open_file::<FakeFSC>(
        "/file1",
        OpenFlags::O_CREAT | OpenFlags::O_RDWR,
        FileMode::FMODE_WRITE,
    ).unwrap();

    let offset = file1.access_inner().f_pos;
    let write_len = vfs_write_file::<FakeFSC>(file1.clone(), b"hello world", offset as u64).unwrap();
    println!("write_len: {}", write_len);
    // when user call vfs_write_file, the file's f_pos will not be updated, so we need to update it manually

    let offset = vfs_llseek(file1.clone(), SeekFrom::Start(6)).unwrap();
    println!("offset: {}", offset);

    let offset = file1.access_inner().f_pos; // == 6
    let mut buf = [0u8; 5];
    let read_len = vfs_read_file::<FakeFSC>(file1.clone(), &mut buf, offset as u64).unwrap();
    println!("read_len: {}", read_len);
    println!("buf: {:?}", core::str::from_utf8(&buf).unwrap()); //"world"

    let offset = vfs_llseek(file1.clone(), SeekFrom::End(10)).unwrap();
    println!("offset: {}", offset);

    let len = vfs_write_file::<FakeFSC>(file1.clone(), b"hello world", offset as u64).unwrap();
    println!("len: {}", len);

    let offset = file1.access_inner().f_pos; // == 32
    println!("offset: {}", offset);

    println!("try to read hole ");
    let offset = vfs_llseek(file1.clone(), SeekFrom::Start(11)).unwrap();
    println!("offset: {}", offset);
    let mut buf = [0u8; 10];
    let read_len = vfs_read_file::<FakeFSC>(file1.clone(), &mut buf, offset as u64).unwrap();
    println!("read_len: {}", read_len);
    println!("buf: {:?}", core::str::from_utf8(&buf).unwrap()); //"\0\0\0\0\0\0\0\0\0\0"
}