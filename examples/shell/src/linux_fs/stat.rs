//! Linux stat/statx projection helpers.
//!
//! The syscall path owns user-memory copying; this module owns Linux-facing
//! statx flag and mask semantics.

use axerrno::LinuxError;
use linux_raw_sys::general;

pub fn validate_statx_flags(_flags: u32) -> Result<(), LinuxError> {
    Ok(())
}

pub const fn statx_accepts_empty_path(_flags: u32) -> bool {
    false
}

pub fn stat_to_statx(_st: &general::stat, _mask: u32) -> general::statx {
    unsafe { core::mem::zeroed() }
}
