# VFS
使用`rust`实现的virtual file system 框架



## Description

## Example

```rust
fn main() {
    println!("init vfs");
    init_vfs();
    let file1 = vfs_open_file::<FakeFSC>(
        "/file1",
        FileFlags::O_CREAT | FileFlags::O_RDWR,
        FileMode::FMODE_WRITE | FileMode::FMODE_READ,
    )
    .unwrap();
	vfs_write_file::<FakeFSC>(file1.clone(), b"hello", 0).unwrap();
	vfs_rename::<FakeFSC>("/file1", "/file3").unwrap();
	let root = vfs_open_file::<FakeFSC>("/", FileFlags::O_RDONLY, 			    							     	    		FileMode::FMODE_READ).unwrap();
    // println!("root: {:#?}", root);
    vfs_readdir(root.clone())
    .unwrap()
    .into_iter()
    .for_each(|name| {
        println!("name: {}", name);
    });
	let mut buf = [0u8; 5];
    vfs_read_file::<FakeFSC>(file2, &mut buf, 0).unwrap();
}

```

