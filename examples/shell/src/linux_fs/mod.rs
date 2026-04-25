//! Linux filesystem ABI helpers for the shell userspace syscall path.
//!
//! This module is not a VFS. It owns Linux-facing semantics and delegates real
//! filesystem capability to existing axfs call sites in `uspace.rs`.

pub mod fd;
pub mod mount;
pub mod path;
pub mod stat;
pub mod types;

pub use mount::{MountRequest, MountTable, UmountRequest};
pub use path::{normalize_path, resolve_cwd_path};
pub use stat::{stat_to_statx, statx_accepts_empty_path, validate_statx_flags};
