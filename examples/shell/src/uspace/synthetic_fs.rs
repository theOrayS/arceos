use axerrno::LinuxError;
use linux_raw_sys::general;
use memory_addr::PAGE_SIZE_4K;
use std::string::String;
use std::sync::Arc;
use std::vec::Vec;

use super::UserProcess;
use super::fd_table::{FdEntry, MemoryFileEntry, PathEntry};
use super::linux_abi::{
    DEFAULT_GROUP_CONTENT, DEFAULT_PASSWD_CONTENT, ETC_GROUP_PATH, ETC_PASSWD_PATH,
    PROC_SELF_MAPS_PATH, USER_ASPACE_BASE, USER_STACK_SIZE, USER_STACK_TOP,
};
use super::memory_map::{align_down, align_up};
use super::runtime_paths::normalize_path;

fn proc_self_maps_content(process: &UserProcess) -> Vec<u8> {
    let exec_path = process.exec_path();
    let brk = *process.brk.lock();
    let text_start = USER_ASPACE_BASE;
    let text_end = text_start + PAGE_SIZE_4K;
    let heap_start = align_down(brk.start, PAGE_SIZE_4K);
    let heap_end = align_up(brk.end.max(brk.start + PAGE_SIZE_4K), PAGE_SIZE_4K);
    let stack_top = align_down(USER_STACK_TOP, PAGE_SIZE_4K);
    let stack_base = stack_top - USER_STACK_SIZE;
    format!(
        "{text_start:08x}-{text_end:08x} r-xp 00000000 00:00 0 {exec_path}\n\
         {heap_start:08x}-{heap_end:08x} rw-p 00000000 00:00 0 [heap]\n\
         {stack_base:08x}-{stack_top:08x} rw-p 00000000 00:00 0 [stack]\n"
    )
    .into_bytes()
}

pub(super) fn is_proc_self_maps_path(path: &str) -> bool {
    normalize_path("/", path).as_deref() == Some(PROC_SELF_MAPS_PATH)
}

pub(super) fn synthetic_file_is_writable_open(flags: u32) -> bool {
    let access = flags & general::O_ACCMODE;
    access == general::O_WRONLY
        || access == general::O_RDWR
        || flags & (general::O_TRUNC | general::O_CREAT) != 0
}

pub(super) fn proc_self_maps_is_writable_open(flags: u32) -> bool {
    synthetic_file_is_writable_open(flags)
}

pub(super) fn proc_self_maps_fd_entry(process: &UserProcess) -> FdEntry {
    FdEntry::MemoryFile(MemoryFileEntry {
        path: PROC_SELF_MAPS_PATH.into(),
        data: Arc::new(proc_self_maps_content(process)),
        offset: 0,
    })
}

pub(super) fn proc_self_maps_path_entry(process: &UserProcess) -> FdEntry {
    let content_len = proc_self_maps_content(process).len();
    FdEntry::Path(PathEntry::synthetic_file(PROC_SELF_MAPS_PATH, content_len))
}

pub(super) fn proc_exe_link_target(process: &UserProcess, path: &str) -> Option<String> {
    let pid_path = format!("/proc/{}/exe", process.pid());
    (path == "/proc/self/exe" || path == pid_path).then(|| process.exec_path())
}

pub(super) fn synthetic_userdb_content(path: &str) -> Option<(&'static str, &'static [u8])> {
    match normalize_path("/", path).as_deref() {
        Some(ETC_PASSWD_PATH) => Some((ETC_PASSWD_PATH, DEFAULT_PASSWD_CONTENT)),
        Some(ETC_GROUP_PATH) => Some((ETC_GROUP_PATH, DEFAULT_GROUP_CONTENT)),
        _ => None,
    }
}

pub(super) fn synthetic_userdb_fd_entry(path: &'static str, data: &'static [u8]) -> FdEntry {
    FdEntry::MemoryFile(MemoryFileEntry {
        path: path.into(),
        data: Arc::new(data.to_vec()),
        offset: 0,
    })
}

pub(super) fn synthetic_userdb_path_entry(path: &'static str, data: &'static [u8]) -> FdEntry {
    FdEntry::Path(PathEntry::synthetic_file(path, data.len()))
}

pub(super) fn dev_shm_host_path(path: &str) -> Option<String> {
    let normalized = normalize_path("/", path)?;
    let rel = normalized.strip_prefix("/dev/shm/")?;
    if rel.is_empty() {
        return None;
    }
    Some(format!("/tmp/shm/{rel}"))
}

pub(super) fn ensure_dev_shm_dir() -> Result<(), LinuxError> {
    ensure_host_dir("/tmp")?;
    ensure_host_dir("/tmp/shm")
}

fn ensure_host_dir(path: &str) -> Result<(), LinuxError> {
    if axfs::api::metadata(path).is_ok() {
        return Ok(());
    }
    axfs::api::create_dir(path).map_err(LinuxError::from)
}
