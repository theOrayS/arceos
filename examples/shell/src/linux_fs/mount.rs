//! Linux mount/umount compatibility semantics.
//!
//! Phase one moves the existing shell compatibility state here before broader
//! runtime mount support exists above axfs.

pub struct MountRequest<'a> {
    pub source: &'a str,
    pub target: &'a str,
    pub fstype: &'a str,
    pub flags: usize,
    pub data: usize,
}

pub struct UmountRequest<'a> {
    pub target: &'a str,
    pub flags: usize,
}

#[derive(Clone, Default)]
pub struct MountTable;
