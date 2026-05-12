use crate::ctypes;
use axerrno::{LinuxError, LinuxResult};
use core::ffi::{c_int, c_void};
use core::ptr::NonNull;

#[cfg(feature = "fd")]
use crate::imp::fd_ops::get_file_like;
#[cfg(not(feature = "fd"))]
use axio::prelude::*;

/// Read data from the file indicated by `fd`.
///
/// Return the read size if success.
///
/// # Safety
///
/// `buf` must either be null with `count == 0`, or be valid for writes of
/// `count` bytes.
pub unsafe fn sys_read(fd: c_int, buf: *mut c_void, count: usize) -> ctypes::ssize_t {
    debug!("sys_read <= {} {:#x} {}", fd, buf as usize, count);
    syscall_body!(sys_read, {
        let dst = unsafe { writable_user_buffer(buf, count)? };
        #[cfg(feature = "fd")]
        {
            Ok(get_file_like(fd)?.read(dst)? as ctypes::ssize_t)
        }
        #[cfg(not(feature = "fd"))]
        match fd {
            0 => Ok(super::stdio::stdin().read(dst)? as ctypes::ssize_t),
            1 | 2 => Err(LinuxError::EPERM),
            _ => Err(LinuxError::EBADF),
        }
    })
}

unsafe fn writable_user_buffer<'a>(buf: *mut c_void, count: usize) -> LinuxResult<&'a mut [u8]> {
    let ptr = if count == 0 {
        NonNull::<u8>::dangling().as_ptr()
    } else {
        if buf.is_null() {
            return Err(LinuxError::EFAULT);
        }
        buf as *mut u8
    };
    Ok(unsafe { core::slice::from_raw_parts_mut(ptr, count) })
}

unsafe fn readable_user_buffer<'a>(buf: *const c_void, count: usize) -> LinuxResult<&'a [u8]> {
    let ptr = if count == 0 {
        NonNull::<u8>::dangling().as_ptr()
    } else {
        if buf.is_null() {
            return Err(LinuxError::EFAULT);
        }
        buf as *const u8
    };
    Ok(unsafe { core::slice::from_raw_parts(ptr, count) })
}

unsafe fn write_impl(fd: c_int, buf: *const c_void, count: usize) -> LinuxResult<ctypes::ssize_t> {
    let src = unsafe { readable_user_buffer(buf, count)? };
    #[cfg(feature = "fd")]
    {
        Ok(get_file_like(fd)?.write(src)? as ctypes::ssize_t)
    }
    #[cfg(not(feature = "fd"))]
    match fd {
        0 => Err(LinuxError::EPERM),
        1 | 2 => Ok(super::stdio::stdout().write(src)? as ctypes::ssize_t),
        _ => Err(LinuxError::EBADF),
    }
}

/// Write data to the file indicated by `fd`.
///
/// Return the written size if success.
///
/// # Safety
///
/// `buf` must either be null with `count == 0`, or be valid for reads of
/// `count` bytes.
pub unsafe fn sys_write(fd: c_int, buf: *const c_void, count: usize) -> ctypes::ssize_t {
    debug!("sys_write <= {} {:#x} {}", fd, buf as usize, count);
    syscall_body!(sys_write, unsafe { write_impl(fd, buf, count) })
}

/// Write a vector.
///
/// # Safety
///
/// `iov` must either be null with `iocnt == 0`, or point to a readable array of
/// `iocnt` iovec entries.
pub unsafe fn sys_writev(fd: c_int, iov: *const ctypes::iovec, iocnt: c_int) -> ctypes::ssize_t {
    debug!("sys_writev <= fd: {}", fd);
    syscall_body!(sys_writev, {
        if !(0..=1024).contains(&iocnt) {
            return Err(LinuxError::EINVAL);
        }
        if iocnt == 0 {
            return Ok(0);
        }
        if iov.is_null() {
            return Err(LinuxError::EFAULT);
        }

        let iovs = unsafe { core::slice::from_raw_parts(iov, iocnt as usize) };
        let mut ret = 0;
        for iov in iovs.iter() {
            if iov.iov_len == 0 {
                continue;
            }
            let result = unsafe { write_impl(fd, iov.iov_base, iov.iov_len) }?;
            if result < 0 {
                return Ok(result);
            }
            ret += result;

            if result < iov.iov_len as isize {
                break;
            }
        }

        Ok(ret)
    })
}
