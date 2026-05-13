#[cfg(feature = "myfs")]
pub mod myfs;

#[cfg(feature = "fatfs")]
pub mod fatfs;

#[cfg(feature = "ext4fs")]
pub mod ext4fs;

#[cfg(feature = "devfs")]
pub use axfs_devfs as devfs;

#[cfg(feature = "ramfs")]
pub use axfs_ramfs as ramfs;
