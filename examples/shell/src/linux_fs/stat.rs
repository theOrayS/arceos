//! Linux stat/statx projection helpers.
//!
//! The syscall path owns user-memory copying; this module owns Linux-facing
//! statx flag and mask semantics.

use axerrno::LinuxError;
use linux_raw_sys::general;

pub const STATX_TYPE: u32 = 0x0001;
pub const STATX_MODE: u32 = 0x0002;
pub const STATX_NLINK: u32 = 0x0004;
pub const STATX_UID: u32 = 0x0008;
pub const STATX_GID: u32 = 0x0010;
pub const STATX_INO: u32 = 0x0100;
pub const STATX_SIZE: u32 = 0x0200;
pub const STATX_BLOCKS: u32 = 0x0400;

pub const STATX_SUPPORTED_MASK: u32 = STATX_TYPE
    | STATX_MODE
    | STATX_NLINK
    | STATX_UID
    | STATX_GID
    | STATX_INO
    | STATX_SIZE
    | STATX_BLOCKS;

const AT_SYMLINK_NOFOLLOW_FLAG: u32 = 0x0100;
const AT_NO_AUTOMOUNT_FLAG: u32 = 0x0800;
const AT_EMPTY_PATH_FLAG: u32 = 0x1000;
const AT_STATX_SYNC_TYPE_MASK: u32 = 0x6000;
const STATX_ALLOWED_FLAGS: u32 =
    AT_SYMLINK_NOFOLLOW_FLAG | AT_NO_AUTOMOUNT_FLAG | AT_EMPTY_PATH_FLAG | AT_STATX_SYNC_TYPE_MASK;

pub fn validate_statx_flags(flags: u32) -> Result<(), LinuxError> {
    if flags & !STATX_ALLOWED_FLAGS != 0 {
        Err(LinuxError::EINVAL)
    } else {
        Ok(())
    }
}

pub const fn statx_accepts_empty_path(flags: u32) -> bool {
    flags & AT_EMPTY_PATH_FLAG != 0
}

pub fn stat_to_statx(st: &general::stat, mask: u32) -> general::statx {
    let mut stx: general::statx = unsafe { core::mem::zeroed() };
    let requested = if mask == 0 {
        STATX_SUPPORTED_MASK
    } else {
        mask
    };
    stx.stx_mask = requested & STATX_SUPPORTED_MASK;
    stx.stx_blksize = st.st_blksize as _;
    stx.stx_nlink = st.st_nlink as _;
    stx.stx_uid = st.st_uid as _;
    stx.stx_gid = st.st_gid as _;
    stx.stx_mode = st.st_mode as _;
    stx.stx_ino = st.st_ino as _;
    stx.stx_size = st.st_size as _;
    stx.stx_blocks = st.st_blocks as _;
    stx.stx_dev_minor = st.st_dev as _;
    stx.stx_rdev_minor = st.st_rdev as _;
    stx
}

#[cfg(test)]
mod tests {
    use super::{
        STATX_MODE, STATX_SIZE, STATX_SUPPORTED_MASK, stat_to_statx, validate_statx_flags,
    };
    use axerrno::LinuxError;
    use linux_raw_sys::general;

    #[test]
    fn statx_mask_reports_only_supported_requested_fields() {
        let st: general::stat = unsafe { core::mem::zeroed() };
        let stx = stat_to_statx(&st, STATX_MODE | STATX_SIZE | 0x8000_0000);
        assert_eq!(stx.stx_mask, STATX_MODE | STATX_SIZE);
    }

    #[test]
    fn zero_mask_reports_supported_default_fields() {
        let st: general::stat = unsafe { core::mem::zeroed() };
        let stx = stat_to_statx(&st, 0);
        assert_eq!(stx.stx_mask, STATX_SUPPORTED_MASK);
    }

    #[test]
    fn unsupported_flags_are_invalid() {
        assert_eq!(validate_statx_flags(0x8000_0000), Err(LinuxError::EINVAL));
    }
}
