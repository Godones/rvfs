mod hardlink;
mod symlink;

use bitflags::bitflags;
pub use hardlink::{vfs_link, vfs_unlink};
pub use symlink::{vfs_readlink, vfs_symlink};

bitflags! {
    #[derive(Default)]
    pub struct LinkFlags:u32{
        /// Follow symbolic links.
        const AT_SYMLINK_FOLLOW = 0x400;
        /// Allow empty relative pathname.
        const AT_EMPTY_PATH = 0x1000;
    }
}
