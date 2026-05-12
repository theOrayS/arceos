use core::ffi::c_void;
use core::sync::atomic::{AtomicUsize, Ordering};

use arceos_posix_api::ctypes as posix_ctypes;
use axerrno::LinuxError;
use axsync::Mutex;
use linux_raw_sys::general;
use std::sync::Arc;

use super::linux_abi::{LOCAL_SOCKET_INO_BASE, ST_MODE_SOCKET};
use super::{SelectMode, posix_ret_i32, posix_ret_usize};

#[derive(Default)]
pub(super) struct SocketOptions {
    pub(super) ip_mcast_joined: bool,
}

#[derive(Clone)]
pub(super) struct SocketEntry {
    pub(super) posix_fd: i32,
    pub(super) socktype: i32,
    pub(super) options: Arc<Mutex<SocketOptions>>,
}

#[derive(Clone)]
pub(super) struct LocalSocketEntry {
    id: usize,
    socktype: i32,
    nonblocking: bool,
    options: Arc<Mutex<SocketOptions>>,
}

static NEXT_LOCAL_SOCKET_ID: AtomicUsize = AtomicUsize::new(1);

impl SocketEntry {
    pub(super) fn new(posix_fd: i32, socktype: i32) -> Self {
        Self {
            posix_fd,
            socktype,
            options: Arc::new(Mutex::new(SocketOptions::default())),
        }
    }

    pub(super) fn duplicate(&self) -> Result<Self, LinuxError> {
        let posix_fd = posix_ret_i32(arceos_posix_api::sys_dup(self.posix_fd))?;
        Ok(Self {
            posix_fd,
            socktype: self.socktype,
            options: self.options.clone(),
        })
    }

    pub(super) fn read(&self, dst: &mut [u8]) -> Result<usize, LinuxError> {
        posix_ret_usize(unsafe {
            arceos_posix_api::sys_recv(self.posix_fd, dst.as_mut_ptr() as *mut c_void, dst.len(), 0)
        })
    }

    pub(super) fn write(&self, src: &[u8]) -> Result<usize, LinuxError> {
        posix_ret_usize(unsafe {
            arceos_posix_api::sys_send(self.posix_fd, src.as_ptr() as *const c_void, src.len(), 0)
        })
    }

    pub(super) fn close(&self) -> Result<(), LinuxError> {
        posix_ret_i32(arceos_posix_api::sys_close(self.posix_fd)).map(|_| ())
    }

    pub(super) fn poll(&self, mode: SelectMode) -> bool {
        match arceos_posix_api::poll_file_like(self.posix_fd) {
            Ok(state) => match mode {
                SelectMode::Read => state.readable,
                SelectMode::Write => state.writable,
                SelectMode::Except => false,
            },
            Err(_) => matches!(mode, SelectMode::Except),
        }
    }

    pub(super) fn stat(&self) -> general::stat {
        let mut st: general::stat = unsafe { core::mem::zeroed() };
        st.st_ino = self.posix_fd as _;
        st.st_mode = ST_MODE_SOCKET | 0o666;
        st.st_nlink = 1;
        st.st_blksize = 512;
        st
    }
}

impl LocalSocketEntry {
    pub(super) fn new(socktype: i32, flags: i32) -> Self {
        Self {
            id: NEXT_LOCAL_SOCKET_ID.fetch_add(1, Ordering::Relaxed),
            socktype,
            nonblocking: flags & posix_ctypes::SOCK_NONBLOCK as i32 != 0,
            options: Arc::new(Mutex::new(SocketOptions::default())),
        }
    }

    pub(super) fn duplicate(&self) -> Self {
        Self {
            id: self.id,
            socktype: self.socktype,
            nonblocking: self.nonblocking,
            options: self.options.clone(),
        }
    }

    pub(super) fn read(&self, _dst: &mut [u8]) -> Result<usize, LinuxError> {
        Err(LinuxError::EINVAL)
    }

    pub(super) fn write(&self, _src: &[u8]) -> Result<usize, LinuxError> {
        Err(LinuxError::EINVAL)
    }

    pub(super) fn poll(&self, mode: SelectMode) -> bool {
        matches!(mode, SelectMode::Write)
    }

    pub(super) fn status_flags(&self) -> i32 {
        let mut flags = self.socktype;
        if self.nonblocking {
            flags |= posix_ctypes::SOCK_NONBLOCK as i32;
        }
        flags
    }

    pub(super) fn stat(&self) -> general::stat {
        let mut st: general::stat = unsafe { core::mem::zeroed() };
        st.st_ino = LOCAL_SOCKET_INO_BASE + self.id as u64;
        st.st_mode = ST_MODE_SOCKET | 0o666;
        st.st_nlink = 1;
        st.st_blksize = 512;
        st
    }
}
