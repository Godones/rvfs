# VFS



**vfs_name**

当在Linux系统中执行重命名操作时，会调用内核中的`vfs_rename`函数来处理。以下是`vfs_rename`函数的解析过程：

1. 检查路径名和文件名是否存在，如果不存在则返回错误信息。
2. 确定源文件和目标文件所在的目录，并验证用户是否对这些目录具有适当的权限。
3. 验证目标文件是否已经存在，并验证用户是否对其具有适当的权限。
4. 在文件系统中获取源文件的inode和目标文件的inode。
5. 验证源文件是否可以重命名（即，它不是一个目录并且它没有被锁定）。
6. 验证目标文件是否可以替换（即，它不是一个目录并且它没有被锁定）。
7. 如果目标文件已经存在并且它是一个目录，则确保源文件和目标文件不在同一个目录下。
8. 如果源文件和目标文件在同一个文件系统上，则调用`rename`系统调用将源文件的目录项更改为目标文件的目录项。
9. 如果源文件和目标文件在不同的文件系统上，则执行一个文件系统层次的拷贝，将源文件的数据复制到目标文件，并删除源文件。
10. 更新源文件和目标文件所在目录的目录项缓存，以便反映文件系统的更改。
11. 返回成功或失败的信息。





当调用`vfs_rename`函数时，它将根据给定的源路径和目标路径，将源路径所指的文件或目录重命名为目标路径。该函数的处理过程如下：

1. 首先，`vfs_rename`函数会获取源路径和目标路径的父目录，以确保源路径和目标路径处于同一文件系统上。
2. `vfs_rename`函数会通过VFS层的`lookup`函数查找源路径所指向的文件或目录，并获取其对应的inode和dentry对象。
3. 如果源路径和目标路径指向同一文件或目录，则直接返回成功。
4. `vfs_rename`函数会检查目标路径是否已经存在。如果目标路径所指的文件或目录已经存在，则必须在目标路径上执行删除操作，以确保不会存在冲突。
5. `vfs_rename`函数会通过VFS层的`rename`函数实现源路径到目标路径的重命名。在这个过程中，VFS层会操作源路径和目标路径的dentry对象和inode对象，以及它们所对应的目录项和文件数据块。
6. `vfs_rename`函数会更新inode对象和dentry对象的引用计数，并更新它们所对应的文件系统数据结构。如果有必要，还会更新与inode对象关联的设备信息。
7. 如果`vfs_rename`函数成功完成了所有操作，则返回0。如果出现错误，函数会返回相应的错误代码。


## 参考资料

[Linux 虚拟文件系统 - beihai blog (wingsxdu.com)](https://wingsxdu.com/posts/linux/vfs/)

[ramfs、tmpfs、rootfs、ramdisk介绍 - 瘋耔 - 博客园 (cnblogs.com)](https://www.cnblogs.com/qiynet/p/15118550.html)

[linux系统调用之sys_unlink（基于linux0.11） - 腾讯云开发者社区-腾讯云 (tencent.com)](https://cloud.tencent.com/developer/article/1425086)

[Linux内核2.4.18创建硬链接的系统调用sys_link - 嵌入式Linux中文站 (embeddedlinux.org.cn)](http://www.embeddedlinux.org.cn/emb-linux/system-development/201708/11-7108.html)

[Linux内核2.4.18创建符号链接的系统调用sys_symlink分析-aweii-ChinaUnix博客](http://m.blog.chinaunix.net/uid-9059-id-5762627.html)

[序 - Linux x86_64系统调用简介 (evian-zhang.github.io)](https://evian-zhang.github.io/introduction-to-linux-x86_64-syscall/index.html)

[Linux VFS机制简析（一） - 舰队 - 博客园 (cnblogs.com)](https://www.cnblogs.com/jimbo17/p/10107318.html)

[Overview of the Linux Virtual File System — The Linux Kernel documentation](https://www.kernel.org/doc/html/next/filesystems/vfs.html)
