use rvfs::dentry::{path_walk, LookUpFlags};
use rvfs::file::{vfs_mkdir, vfs_open_file, vfs_read_file, vfs_write_file, FileFlags, FileMode};
use rvfs::link::{vfs_link, vfs_symlink, vfs_unlink};
use rvfs::mount::{do_mount, MountFlags};
use rvfs::ramfs::tmpfs::tmp_fs_type;
use rvfs::stat::vfs_getattr;
use rvfs::superblock::register_filesystem;
use rvfs::{init_process_info, mount_rootfs, FakeFSC};

fn main() {
    env_logger::init();
    println!("init vfs");
    let rootfs = mount_rootfs();
    init_process_info(rootfs);
    println!("init vfs ok ......");
    // let lookup_data = path_walk::<FakeFSC>("/", LookUpFlags::DIRECTORY).unwrap();
    // println!("lookup_data: {:#?}", lookup_data);

    println!("--------------------------------------");
    println!("mkdir /tmp");
    vfs_mkdir::<FakeFSC>("/tmp", FileMode::FMODE_WRITE).unwrap();
    println!("mkdir /tmp ok ......");

    println!("--------------------------------------");
    println!("test path_walk /tmp");
    let _temp_find = path_walk::<FakeFSC>("/tmp", LookUpFlags::DIRECTORY).unwrap();
    println!("test path_walk /tmp ok ......");
    // println!("temp_find: {:#?}",temp_find);

    println!("--------------------------------------");
    // open exist file
    // let file = open_file::<FakeFSC>("/tmp", FileFlags::O_RDWR,FileMode::FMODE_READ).unwrap();
    // println!("file: {:#?}",file);
    println!("test create file /f1");
    // open or create file
    let file = vfs_open_file::<FakeFSC>(
        "/f1",
        FileFlags::O_RDWR | FileFlags::O_CREAT,
        FileMode::FMODE_WRITE | FileMode::FMODE_READ,
    )
    .unwrap();
    println!("test create file /f1 ok ......");

    println!("--------------------------------------");
    println!("test read/write file");
    // test read / write
    let mut buf = [0u8; 10];
    vfs_write_file::<FakeFSC>(file.clone(), [1, 2, 3, 4, 5, 6, 7, 8, 9, 10].as_ref(), 0).unwrap();
    let _read = vfs_read_file::<FakeFSC>(file, buf.as_mut(), 0).unwrap();
    println!("read: {:?}", buf);
    println!("test read/write file ok ......");

    // 注册tmpfs，实际上也是一个内存文件系统，但这里的实现将其与rootfs分开了
    println!("----------------------------------------");
    register_filesystem(tmp_fs_type()).unwrap();
    println!("register tmpfs ok ......");
    println!("test do_mount");
    let tmpfs = do_mount::<FakeFSC>("", "/tmp", "tmpfs", MountFlags::MNT_NO_DEV, None).unwrap();
    // println!("mnt: {:#?}", mnt);
    println!("test do_mount ok ......");

    println!("----------------------------------------");
    println!("{:#?}", tmpfs);

    println!("----------------------------------------");
    println!("mkdir /tmp/tt1, it should in tmpfs root dir");

    vfs_mkdir::<FakeFSC>("/tmp/tt1", FileMode::FMODE_WRITE).unwrap();
    println!("mkdir /tmp/tt1 ok ......");

    let temp_find = path_walk::<FakeFSC>("/tmp/tt1", LookUpFlags::DIRECTORY).unwrap();
    println!("temp_find: {:#?}", temp_find.dentry);
    // println!("{:#?}", tmpfs);
    println!("---------------------------------------");
    println!("test vfs_link");
    vfs_link::<FakeFSC>("/f1", "/f2").unwrap();
    println!("test vfs_link ok ......");
    println!("----------------------------------------");
    let f1_lookup = path_walk::<FakeFSC>("/f1", LookUpFlags::READ_LINK).unwrap();
    let f2_lookup = path_walk::<FakeFSC>("/f2", LookUpFlags::READ_LINK).unwrap();
    println!("f1_lookup: {:#?}", f1_lookup.dentry);
    println!("f2_lookup: {:#?}", f2_lookup.dentry);

    println!("-----------------------------------------");
    vfs_unlink::<FakeFSC>("/f2").unwrap();
    let f2_lookup = path_walk::<FakeFSC>("/f2", LookUpFlags::READ_LINK);
    assert_eq!(f2_lookup.is_err(), true);
    let f1_lookup = path_walk::<FakeFSC>("/f1", LookUpFlags::READ_LINK).unwrap();
    assert_eq!(
        f1_lookup
            .dentry
            .access_inner()
            .d_inode
            .access_inner()
            .hard_links,
        1
    );

    let file_attr = vfs_getattr::<FakeFSC>("/f1").unwrap();
    println!("file_attr: {:#?}", file_attr);

    let dir_attr = vfs_getattr::<FakeFSC>("/").unwrap();
    println!("dir_attr: {:#?}", dir_attr);

    println!("-----------------------------------------");
    vfs_symlink::<FakeFSC>("/", "/s1").unwrap();
    println!("vfs_symlink ok ......");

    let file_attr = vfs_getattr::<FakeFSC>("/s1").unwrap();
    println!("file_attr: {:#?}", file_attr);

    vfs_symlink::<FakeFSC>("/tmp/tt1", "/s2").unwrap();
    println!("vfs_symlink ok ......");
    let file_attr = vfs_getattr::<FakeFSC>("/s2").unwrap();
    println!("file_attr: {:#?}", file_attr);
}
