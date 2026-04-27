#[cfg(feature = "uspace")]
use alloc::collections::BTreeMap;
#[cfg(feature = "uspace")]
use alloc::string::String;
use alloc::sync::Arc;
use core::ffi::{c_char, c_int};

use axerrno::{LinuxError, LinuxResult};
use axfs::api::{MountFsKind, mount_info_for_path};
use axfs::fops::OpenOptions;
use axhal::mem::PAGE_SIZE_4K;
use axio::{PollState, SeekFrom};
use axsync::Mutex;
#[cfg(feature = "uspace")]
use lazyinit::LazyInit;
use linux_raw_sys::general;

use super::fd_ops::{FileLike, get_file_like};
use crate::{ctypes, utils::char_ptr_to_str};

const STATFS_BLOCK_SIZE: u64 = PAGE_SIZE_4K as u64;
const EXT4_SUPER_MAGIC: i64 = 0xEF53;
const MSDOS_SUPER_MAGIC: i64 = 0x4d44;
const PROC_SUPER_MAGIC: i64 = 0x9fa0;
const SYSFS_MAGIC: i64 = 0x62656572;
const TMPFS_MAGIC: i64 = 0x0102_1994;
const DEVFS_MAGIC: i64 = 0x1373;
const ST_RDONLY: i64 = 1;
const ST_MODE_DIR: u32 = 0o040000;
const ST_MODE_FILE: u32 = 0o100000;
const ST_MODE_CHR: u32 = 0o020000;
const ST_MODE_LNK: u32 = 0o120000;
#[cfg(feature = "uspace")]
const DEFAULT_UMASK: ctypes::mode_t = 0o022;

#[cfg(feature = "uspace")]
fn process_umasks() -> &'static Mutex<BTreeMap<i32, ctypes::mode_t>> {
    static PROCESS_UMASKS: LazyInit<Mutex<BTreeMap<i32, ctypes::mode_t>>> = LazyInit::new();
    PROCESS_UMASKS.call_once(|| Mutex::new(BTreeMap::new()));
    &PROCESS_UMASKS
}

#[cfg(feature = "uspace")]
#[derive(Clone, Copy)]
struct PathTimes {
    atime: general::timespec,
    mtime: general::timespec,
    ctime: general::timespec,
}

#[cfg(feature = "uspace")]
fn path_times() -> &'static Mutex<BTreeMap<String, PathTimes>> {
    static PATH_TIMES: LazyInit<Mutex<BTreeMap<String, PathTimes>>> = LazyInit::new();
    PATH_TIMES.call_once(|| Mutex::new(BTreeMap::new()));
    &PATH_TIMES
}

#[cfg(feature = "uspace")]
const fn zero_timespec() -> general::timespec {
    general::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    }
}

#[cfg(feature = "uspace")]
fn current_timespec() -> general::timespec {
    let now = axhal::time::wall_time();
    general::timespec {
        tv_sec: now.as_secs() as _,
        tv_nsec: now.subsec_nanos() as _,
    }
}

#[derive(Clone, Copy)]
pub(crate) struct FileSystemStat {
    pub f_type: i64,
    pub f_bsize: i64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_namelen: i64,
    pub f_frsize: i64,
    pub f_flags: i64,
}

pub struct File {
    inner: Mutex<axfs::fops::File>,
}

impl File {
    fn new(inner: axfs::fops::File) -> Self {
        Self {
            inner: Mutex::new(inner),
        }
    }

    fn add_to_fd_table(self) -> LinuxResult<c_int> {
        super::fd_ops::add_file_like(Arc::new(self))
    }

    fn from_fd(fd: c_int) -> LinuxResult<Arc<Self>> {
        let f = super::fd_ops::get_file_like(fd)?;
        f.into_any()
            .downcast::<Self>()
            .map_err(|_| LinuxError::EINVAL)
    }
}

impl FileLike for File {
    fn read(&self, buf: &mut [u8]) -> LinuxResult<usize> {
        Ok(self.inner.lock().read(buf)?)
    }

    fn write(&self, buf: &[u8]) -> LinuxResult<usize> {
        Ok(self.inner.lock().write(buf)?)
    }

    fn stat(&self) -> LinuxResult<ctypes::stat> {
        let metadata = self.inner.lock().get_attr()?;
        let ty = metadata.file_type() as u8;
        let perm = metadata.perm().bits() as u32;
        let st_mode = ((ty as u32) << 12) | perm;
        Ok(ctypes::stat {
            st_ino: 1,
            st_nlink: 1,
            st_mode,
            st_uid: 1000,
            st_gid: 1000,
            st_size: metadata.size() as _,
            st_blocks: metadata.blocks() as _,
            st_blksize: 512,
            ..Default::default()
        })
    }

    fn into_any(self: Arc<Self>) -> Arc<dyn core::any::Any + Send + Sync> {
        self
    }

    fn poll(&self) -> LinuxResult<PollState> {
        Ok(PollState {
            readable: true,
            writable: true,
        })
    }

    fn set_nonblocking(&self, _nonblocking: bool) -> LinuxResult {
        Ok(())
    }
}

/// Convert open flags to [`OpenOptions`].
fn flags_to_options(flags: c_int, _mode: ctypes::mode_t) -> OpenOptions {
    let flags = flags as u32;
    let mut options = OpenOptions::new();
    match flags & 0b11 {
        ctypes::O_RDONLY => options.read(true),
        ctypes::O_WRONLY => options.write(true),
        _ => {
            options.read(true);
            options.write(true);
        }
    };
    if flags & ctypes::O_APPEND != 0 {
        options.append(true);
    }
    if flags & ctypes::O_TRUNC != 0 {
        options.truncate(true);
    }
    if flags & ctypes::O_CREAT != 0 {
        options.create(true);
    }
    if flags & ctypes::O_EXEC != 0 {
        options.create_new(true);
    }
    options
}

/// Open a file by `filename` and insert it into the file descriptor table.
///
/// Return its index in the file table (`fd`). Return `EMFILE` if it already
/// has the maximum number of files open.
pub fn sys_open(filename: *const c_char, flags: c_int, mode: ctypes::mode_t) -> c_int {
    let filename = char_ptr_to_str(filename);
    debug!("sys_open <= {:?} {:#o} {:#o}", filename, flags, mode);
    syscall_body!(sys_open, {
        let options = flags_to_options(flags, mode);
        let file = axfs::fops::File::open(filename?, &options)?;
        File::new(file).add_to_fd_table()
    })
}

/// Set the position of the file indicated by `fd`.
///
/// Return its position after seek.
pub fn sys_lseek(fd: c_int, offset: ctypes::off_t, whence: c_int) -> ctypes::off_t {
    debug!("sys_lseek <= {} {} {}", fd, offset, whence);
    syscall_body!(sys_lseek, {
        let pos = match whence {
            0 => SeekFrom::Start(offset as _),
            1 => SeekFrom::Current(offset as _),
            2 => SeekFrom::End(offset as _),
            _ => return Err(LinuxError::EINVAL),
        };
        let off = File::from_fd(fd)?.inner.lock().seek(pos)?;
        Ok(off)
    })
}

/// Get the file metadata by `path` and write into `buf`.
///
/// Return 0 if success.
pub unsafe fn sys_stat(path: *const c_char, buf: *mut ctypes::stat) -> c_int {
    let path = char_ptr_to_str(path);
    debug!("sys_stat <= {:?} {:#x}", path, buf as usize);
    syscall_body!(sys_stat, {
        if buf.is_null() {
            return Err(LinuxError::EFAULT);
        }
        let mut options = OpenOptions::new();
        options.read(true);
        let file = axfs::fops::File::open(path?, &options)?;
        let st = File::new(file).stat()?;
        unsafe { *buf = st };
        Ok(0)
    })
}

/// Get file metadata by `fd` and write into `buf`.
///
/// Return 0 if success.
pub unsafe fn sys_fstat(fd: c_int, buf: *mut ctypes::stat) -> c_int {
    debug!("sys_fstat <= {} {:#x}", fd, buf as usize);
    syscall_body!(sys_fstat, {
        if buf.is_null() {
            return Err(LinuxError::EFAULT);
        }

        unsafe { *buf = get_file_like(fd)?.stat()? };
        Ok(0)
    })
}

/// Get the metadata of the symbolic link and write into `buf`.
///
/// Return 0 if success.
pub unsafe fn sys_lstat(path: *const c_char, buf: *mut ctypes::stat) -> ctypes::ssize_t {
    let path = char_ptr_to_str(path);
    debug!("sys_lstat <= {:?} {:#x}", path, buf as usize);
    syscall_body!(sys_lstat, {
        if buf.is_null() {
            return Err(LinuxError::EFAULT);
        }
        unsafe { *buf = Default::default() }; // TODO
        Ok(0)
    })
}

/// Get the path of the current directory.
#[allow(clippy::unnecessary_cast)] // `c_char` is either `i8` or `u8`
pub fn sys_getcwd(buf: *mut c_char, size: usize) -> *mut c_char {
    debug!("sys_getcwd <= {:#x} {}", buf as usize, size);
    syscall_body!(sys_getcwd, {
        if buf.is_null() {
            return Ok(core::ptr::null::<c_char>() as _);
        }
        let dst = unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, size as _) };
        let cwd = axfs::api::current_dir()?;
        let cwd = cwd.as_bytes();
        if cwd.len() < size {
            dst[..cwd.len()].copy_from_slice(cwd);
            dst[cwd.len()] = 0;
            Ok(buf)
        } else {
            Err(LinuxError::ERANGE)
        }
    })
}

/// Rename `old` to `new`
/// If new exists, it is first removed.
///
/// Return 0 if the operation succeeds, otherwise return -1.
pub fn sys_rename(old: *const c_char, new: *const c_char) -> c_int {
    syscall_body!(sys_rename, {
        let old_path = char_ptr_to_str(old)?;
        let new_path = char_ptr_to_str(new)?;
        debug!("sys_rename <= old: {:?}, new: {:?}", old_path, new_path);
        axfs::api::rename(old_path, new_path)?;
        Ok(0)
    })
}

pub(crate) fn statfs_for_path(path: &str) -> LinuxResult<FileSystemStat> {
    axfs::api::metadata(path).map_err(LinuxError::from)?;
    let mount = mount_info_for_path(path).ok_or(LinuxError::ENOENT)?;
    let total_bytes = match mount.kind {
        MountFsKind::ProcFs | MountFsKind::SysFs | MountFsKind::DevFs => 0,
        MountFsKind::Fat | MountFsKind::Ext4 | MountFsKind::RamFs => {
            axhal::mem::total_ram_size() as u64
        }
    };
    let free_bytes = match mount.kind {
        MountFsKind::ProcFs | MountFsKind::SysFs | MountFsKind::DevFs => 0,
        MountFsKind::Fat | MountFsKind::Ext4 | MountFsKind::RamFs => free_ram_bytes(),
    };
    Ok(FileSystemStat {
        f_type: statfs_magic(mount.kind),
        f_bsize: STATFS_BLOCK_SIZE as i64,
        f_blocks: total_bytes / STATFS_BLOCK_SIZE,
        f_bfree: free_bytes / STATFS_BLOCK_SIZE,
        f_bavail: free_bytes / STATFS_BLOCK_SIZE,
        f_files: 1024,
        f_ffree: 1024,
        f_namelen: 255,
        f_frsize: STATFS_BLOCK_SIZE as i64,
        f_flags: statfs_flags(mount.options),
    })
}

#[cfg(feature = "uspace")]
pub(crate) fn set_process_umask(pid: i32, mask: ctypes::mode_t) -> ctypes::mode_t {
    let mask = mask & 0o777;
    process_umasks()
        .lock()
        .insert(pid, mask)
        .unwrap_or(DEFAULT_UMASK)
}

#[cfg(feature = "uspace")]
pub(crate) fn clear_process_umask(pid: i32) {
    process_umasks().lock().remove(&pid);
}

#[cfg(feature = "uspace")]
pub(crate) fn update_path_times(
    path: &str,
    atime: Option<general::timespec>,
    mtime: Option<general::timespec>,
) {
    let now = current_timespec();
    let mut times = path_times().lock();
    let entry = times.entry(path.into()).or_insert(PathTimes {
        atime: zero_timespec(),
        mtime: zero_timespec(),
        ctime: zero_timespec(),
    });
    if let Some(ts) = atime {
        entry.atime = ts;
    }
    if let Some(ts) = mtime {
        entry.mtime = ts;
    }
    entry.ctime = now;
}

#[cfg(feature = "uspace")]
pub(crate) fn apply_path_times_to_stat(st: &mut general::stat, path: Option<&str>) {
    let Some(path) = path else {
        return;
    };
    let Some(times) = path_times().lock().get(path).copied() else {
        return;
    };
    st.st_atime = times.atime.tv_sec;
    st.st_atime_nsec = times.atime.tv_nsec as _;
    st.st_mtime = times.mtime.tv_sec;
    st.st_mtime_nsec = times.mtime.tv_nsec as _;
    st.st_ctime = times.ctime.tv_sec;
    st.st_ctime_nsec = times.ctime.tv_nsec as _;
}

#[cfg(not(feature = "uspace"))]
pub(crate) fn apply_path_times_to_stat(_st: &mut general::stat, _path: Option<&str>) {}

pub(crate) fn metadata_to_linux_stat(
    metadata: &axfs::api::Metadata,
    path: Option<&str>,
) -> general::stat {
    let mut st: general::stat = unsafe { core::mem::zeroed() };
    st.st_dev = 1;
    st.st_ino = path_inode(path);
    st.st_mode = file_type_mode(metadata.file_type()) | metadata.permissions().bits() as u32;
    st.st_nlink = 1;
    st.st_size = metadata.size() as _;
    st.st_blksize = 512;
    st.st_blocks = metadata.blocks() as _;
    apply_path_times_to_stat(&mut st, path);
    st
}

pub(crate) fn symlink_to_linux_stat(path: &str, target_len: usize) -> general::stat {
    let mut st: general::stat = unsafe { core::mem::zeroed() };
    st.st_dev = 1;
    st.st_ino = path_inode(Some(path));
    st.st_mode = ST_MODE_LNK | 0o777;
    st.st_nlink = 1;
    st.st_size = target_len as _;
    st.st_blksize = 512;
    apply_path_times_to_stat(&mut st, Some(path));
    st
}

fn free_ram_bytes() -> u64 {
    #[cfg(feature = "alloc")]
    {
        return axalloc::global_allocator().available_pages() as u64 * STATFS_BLOCK_SIZE;
    }
    #[cfg(not(feature = "alloc"))]
    {
        axhal::mem::total_ram_size() as u64
    }
}

const fn statfs_magic(kind: MountFsKind) -> i64 {
    match kind {
        MountFsKind::Fat => MSDOS_SUPER_MAGIC,
        MountFsKind::Ext4 => EXT4_SUPER_MAGIC,
        MountFsKind::DevFs => DEVFS_MAGIC,
        MountFsKind::RamFs => TMPFS_MAGIC,
        MountFsKind::ProcFs => PROC_SUPER_MAGIC,
        MountFsKind::SysFs => SYSFS_MAGIC,
    }
}

fn statfs_flags(options: &str) -> i64 {
    let mut flags = 0;
    if options.split(',').any(|opt| opt == "ro") {
        flags |= ST_RDONLY;
    }
    flags
}

fn path_inode(path: Option<&str>) -> u64 {
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

const fn file_type_mode(ty: axfs::api::FileType) -> u32 {
    match ty {
        axfs::api::FileType::Dir => ST_MODE_DIR,
        axfs::api::FileType::CharDevice => ST_MODE_CHR,
        axfs::api::FileType::SymLink => ST_MODE_LNK,
        _ => ST_MODE_FILE,
    }
}
