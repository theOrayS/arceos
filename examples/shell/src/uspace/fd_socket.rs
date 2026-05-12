use core::ffi::c_void;
use core::mem::size_of;
use core::sync::atomic::{AtomicUsize, Ordering};

use arceos_posix_api::ctypes as posix_ctypes;
use axerrno::LinuxError;
use axsync::Mutex;
use linux_raw_sys::general;
use std::sync::Arc;
use std::vec::Vec;

use super::linux_abi::{LOCAL_SOCKET_INO_BASE, ST_MODE_SOCKET};
use super::user_memory::{
    read_user_bytes, user_io_buffer, validate_user_read, validate_user_write, write_user_bytes,
    write_user_value,
};
use super::{
    SelectMode, UserProcess, neg_errno, posix_ret_i32, posix_ret_usize,
    recv_with_real_timer_interrupt,
};

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

pub(super) fn read_socket_data_from_user(
    process: &UserProcess,
    ptr: usize,
    len: usize,
) -> Result<Vec<u8>, LinuxError> {
    if ptr == 0 {
        return Err(LinuxError::EFAULT);
    }
    read_user_bytes(process, ptr, len)
}

pub(super) fn read_socket_addr_from_user(
    process: &UserProcess,
    ptr: usize,
    len: usize,
) -> Result<Vec<u8>, LinuxError> {
    validate_user_read(process, ptr, len)?;
    if ptr == 0 {
        return Err(LinuxError::EFAULT);
    }
    if len != size_of::<posix_ctypes::sockaddr>() {
        return Err(LinuxError::EINVAL);
    }
    read_user_bytes(process, ptr, len)
}

fn sockaddr_bytes(addr: &posix_ctypes::sockaddr) -> [u8; size_of::<posix_ctypes::sockaddr>()] {
    let mut bytes = [0u8; size_of::<posix_ctypes::sockaddr>()];
    let family_len = size_of::<posix_ctypes::sa_family_t>();
    bytes[..family_len].copy_from_slice(&addr.sa_family.to_ne_bytes());
    for (dst, src) in bytes[family_len..]
        .iter_mut()
        .zip(addr.sa_data.iter().copied())
    {
        *dst = src as u8;
    }
    bytes
}

pub(super) fn write_socket_addr_to_user(
    process: &UserProcess,
    addr: usize,
    addrlen: usize,
    user_len: usize,
    local_addr: &posix_ctypes::sockaddr,
    local_len: posix_ctypes::socklen_t,
) -> isize {
    let copy_len = core::cmp::min(user_len, size_of::<posix_ctypes::sockaddr>());
    if copy_len > 0 {
        let local_addr_bytes = sockaddr_bytes(local_addr);
        if let Err(err) = write_user_bytes(process, addr, &local_addr_bytes[..copy_len]) {
            return neg_errno(err);
        }
    }
    write_user_value(process, addrlen, &local_len)
}

pub(super) fn recv_socket_data_to_user(
    process: &UserProcess,
    posix_fd: i32,
    buf: usize,
    len: usize,
    flags: i32,
) -> isize {
    recv_socket_data_to_user_inner(process, posix_fd, buf, len, |dst| unsafe {
        arceos_posix_api::sys_recv(posix_fd, dst, len, flags)
    })
}

pub(super) fn recv_socket_data_to_user_with_addr(
    process: &UserProcess,
    posix_fd: i32,
    buf: usize,
    len: usize,
    flags: i32,
    addr: usize,
    addrlen: usize,
    user_addr_len: usize,
) -> isize {
    let mut local_addr: posix_ctypes::sockaddr = unsafe { core::mem::zeroed() };
    let mut local_len = 0 as posix_ctypes::socklen_t;
    let ret = recv_socket_data_to_user_inner(process, posix_fd, buf, len, |dst| unsafe {
        arceos_posix_api::sys_recvfrom(posix_fd, dst, len, flags, &mut local_addr, &mut local_len)
    });
    if ret > 0 && local_len != 0 {
        let addr_ret = write_socket_addr_to_user(
            process,
            addr,
            addrlen,
            user_addr_len,
            &local_addr,
            local_len,
        );
        if addr_ret < 0 {
            return addr_ret;
        }
    }
    ret
}

fn recv_socket_data_to_user_inner(
    process: &UserProcess,
    posix_fd: i32,
    buf: usize,
    len: usize,
    mut recv_once: impl FnMut(*mut c_void) -> isize,
) -> isize {
    if buf == 0 {
        return neg_errno(LinuxError::EFAULT);
    }
    if let Err(err) = validate_user_write(process, buf, len) {
        return neg_errno(err);
    }
    let mut bytes = match user_io_buffer(len) {
        Ok(bytes) => bytes,
        Err(err) => return neg_errno(err),
    };
    let ret = recv_with_real_timer_interrupt(process, posix_fd, || {
        recv_once(bytes.as_mut_ptr() as *mut c_void)
    });
    if ret <= 0 {
        return ret;
    }
    let received = ret as usize;
    if received > len {
        return neg_errno(LinuxError::EINVAL);
    }
    match write_user_bytes(process, buf, &bytes[..received]) {
        Ok(()) => ret,
        Err(err) => neg_errno(err),
    }
}
