// use dbfs2::{init_dbfs, DBFS_TYPE};
// use jammdb::memfile::{FakeMap, FileOpenOptions};
// use jammdb::DB;
// use rvfs::{
//     do_mount, init_vfs, register_filesystem, vfs_mkdir, vfs_open_file, vfs_read_file,
//     vfs_write_file, FakeFSC, FileFlags, FileMode, MountFlags,
// };
// use std::sync::Arc;
//
// fn main() {
//     env_logger::init();
//     println!("Init vfs");
//     init_vfs();
//     println!("Create file /f1 in rootfs");
//     let _ = vfs_open_file::<FakeFSC>(
//         "/f1",
//         FileFlags::O_RDWR | FileFlags::O_CREAT,
//         FileMode::FMODE_WRITE | FileMode::FMODE_READ,
//     )
//     .unwrap();
//     vfs_mkdir::<FakeFSC>("/dbfs", FileMode::FMODE_WRITE).unwrap();
//     println!("Try to mount DBFS");
//     let db = DB::open::<FileOpenOptions, _>(Arc::new(FakeMap), "my-database.db").unwrap();
//     init_dbfs(db);
//     register_filesystem(DBFS_TYPE).unwrap();
//     let dbfs = do_mount::<FakeFSC>("", "/dbfs", "dbfs", MountFlags::MNT_NO_DEV, None).unwrap();
//     //try mount dbfs to /
//     println!("mnt: {:#?}", dbfs);
//     println!("test do_mount ok ......");
//
//     // println!("Create file /f1 in dbfs");
//     // let file1 = vfs_open_file::<FakeFSC>("/f1", FileFlags::O_RDWR | FileFlags::O_CREAT, FileMode::FMODE_WRITE | FileMode::FMODE_READ).unwrap();
//     let file = vfs_open_file::<FakeFSC>(
//         "/dbfs/f1",
//         FileFlags::O_RDWR | FileFlags::O_CREAT,
//         FileMode::FMODE_WRITE | FileMode::FMODE_READ,
//     )
//     .unwrap();
//     println!("file: {:#?}", file);
//     vfs_write_file::<FakeFSC>(file.clone(), [1, 2, 3, 4, 5, 6, 7, 8, 9, 10].as_ref(), 0).unwrap();
//     let mut buf = [0u8; 10];
//     vfs_read_file::<FakeFSC>(file, buf.as_mut(), 0).unwrap();
//     println!("read: {:?}", buf);
//
//     vfs_mkdir::<FakeFSC>("/dbfs/tmp", FileMode::FMODE_WRITE).unwrap();
//     let dir = vfs_open_file::<FakeFSC>("/dbfs/tmp", FileFlags::O_DIRECTORY, FileMode::FMODE_READ)
//         .unwrap();
//     println!("dir: {:#?}", dir);
// }

fn main() {}
