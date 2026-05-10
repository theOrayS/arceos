use axalloc::global_allocator;
use axfs::fops::{FileAttr, FileType};
use linux_raw_sys::general;
use std::string::String;

use super::UserProcess;
use super::fd_table::FdEntry;
use super::linux_abi::{
    DEVFS_MAGIC, FILE_MODE_PERMISSION_MASK, PIPEFS_MAGIC, PROC_SUPER_MAGIC, ST_MODE_CHR,
    ST_MODE_DIR, ST_MODE_FILE, STATFS_BLOCK_SIZE, STATFS_NAME_MAX, SYSFS_MAGIC, TMPFS_MAGIC,
};
use super::synthetic_fs::dev_shm_host_path;

pub(super) fn file_attr_to_stat(attr: &FileAttr, path: Option<&str>) -> general::stat {
    let st_mode = file_type_mode(attr.file_type()) | attr.perm().bits() as u32;
    let mut st: general::stat = unsafe { core::mem::zeroed() };
    st.st_dev = 1;
    st.st_ino = path_inode(path);
    st.st_mode = st_mode;
    st.st_nlink = 1;
    st.st_size = attr.size() as _;
    st.st_blksize = 512;
    st.st_blocks = attr.blocks() as _;
    st
}

pub(super) fn normalize_file_mode(mode: u32) -> u32 {
    mode & FILE_MODE_PERMISSION_MASK
}

pub(super) fn apply_recorded_path_metadata(
    process: &UserProcess,
    path: &str,
    mut st: general::stat,
) -> general::stat {
    if let Some(mode) = process.path_mode(path) {
        st.st_mode = (st.st_mode & !FILE_MODE_PERMISSION_MASK) | mode;
    }
    if let Some((uid, gid)) = process.path_owner(path) {
        st.st_uid = uid;
        st.st_gid = gid;
    }
    st
}

pub(super) fn canonical_permission_path(path: String) -> String {
    dev_shm_host_path(path.as_str()).unwrap_or(path)
}

pub(super) fn fd_entry_path(entry: &FdEntry) -> Option<&str> {
    match entry {
        FdEntry::File(file) => Some(file.path.as_str()),
        FdEntry::Directory(dir) => Some(dir.path.as_str()),
        FdEntry::Path(path) => Some(path.path.as_str()),
        FdEntry::MemoryFile(file) => Some(file.path.as_str()),
        _ => None,
    }
}

pub(super) fn fd_entry_statfs_path(entry: &FdEntry) -> Option<&str> {
    match entry {
        FdEntry::DevNull => Some("/dev/null"),
        FdEntry::Rtc => Some("/dev/misc/rtc"),
        FdEntry::Pipe(_) => Some("pipe:"),
        FdEntry::Socket(_) | FdEntry::LocalSocket(_) => Some("socket:"),
        _ => fd_entry_path(entry),
    }
}

fn statfs_type_for_path(path: Option<&str>) -> i64 {
    match path {
        Some(path) if path == "/proc" || path.starts_with("/proc/") => PROC_SUPER_MAGIC,
        Some(path) if path == "/sys" || path.starts_with("/sys/") => SYSFS_MAGIC,
        Some(path) if path == "/dev" || path.starts_with("/dev/") => DEVFS_MAGIC,
        Some(path) if path.starts_with("pipe:") => PIPEFS_MAGIC,
        _ => TMPFS_MAGIC,
    }
}

pub(super) fn generic_statfs(path: Option<&str>) -> general::statfs {
    let alloc = global_allocator();
    let available_pages = alloc.available_pages() as i64;
    let total_pages = (alloc.used_pages() as i64 + available_pages).max(1);
    let fs_type = statfs_type_for_path(path);
    general::statfs {
        f_type: fs_type as _,
        f_bsize: STATFS_BLOCK_SIZE as _,
        f_blocks: total_pages as _,
        f_bfree: available_pages as _,
        f_bavail: available_pages as _,
        f_files: total_pages as _,
        f_ffree: available_pages as _,
        f_fsid: general::__kernel_fsid_t {
            val: [fs_type as i32, 0],
        },
        f_namelen: STATFS_NAME_MAX as _,
        f_frsize: STATFS_BLOCK_SIZE as _,
        f_flags: 0,
        f_spare: [0; 4],
    }
}

pub(super) fn path_inode(path: Option<&str>) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let Some(path) = path else {
        return 1;
    };
    let mut hash = FNV_OFFSET;
    for &byte in path.as_bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash.max(1)
}

pub(super) fn file_type_mode(ty: FileType) -> u32 {
    match ty {
        FileType::Dir => ST_MODE_DIR,
        FileType::CharDevice => ST_MODE_CHR,
        _ => ST_MODE_FILE,
    }
}
