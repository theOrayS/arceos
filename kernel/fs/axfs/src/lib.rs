//! [ArceOS](https://github.com/arceos-org/arceos) filesystem module.
//!
//! It provides unified filesystem operations for various filesystems.
//!
//! # Cargo Features
//!
//! - `fatfs`: Use [FAT] as the main filesystem and mount it on `/`. This feature
//!   is **enabled** by default.
//! - `ext4fs`: Use [ext4] as the main filesystem and mount it on `/`. This feature
//!   is **enabled** by default for competition images and is read-only.
//! - `devfs`: Mount [`axfs_devfs::DeviceFileSystem`] on `/dev`. This feature is
//!   **enabled** by default.
//! - `ramfs`: Mount [`axfs_ramfs::RamFileSystem`] on `/tmp`. This feature is
//!   **enabled** by default.
//! - `myfs`: Allow users to define their custom filesystems to override the
//!   default. In this case, [`MyFileSystemIf`] is required to be implemented
//!   to create and initialize other filesystems. This feature is **disabled** by
//!   by default, but it will override other filesystem selection features if
//!   both are enabled.
//!
//! [FAT]: https://en.wikipedia.org/wiki/File_Allocation_Table
//! [ext4]: https://en.wikipedia.org/wiki/Ext4
//! [`MyFileSystemIf`]: fops::MyFileSystemIf

#![cfg_attr(all(not(test), not(doc)), no_std)]
#![feature(doc_auto_cfg)]

#[macro_use]
extern crate log;
extern crate alloc;

mod dev;
mod fs;
mod mounts;
mod root;

pub mod api;
pub mod fops;

use axdriver::{AxDeviceContainer, prelude::*};

/// Initializes filesystems by block devices.
pub fn init_filesystems(mut blk_devs: AxDeviceContainer<AxBlockDevice>) {
    info!("Initialize filesystems...");

    let dev = blk_devs.take_one().expect("No block device found!");
    info!("  use block device 0: {:?}", dev.device_name());
    self::root::init_rootfs(self::dev::Disk::new(dev));
}
