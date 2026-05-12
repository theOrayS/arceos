use core::{ffi::c_int, ptr};

use arceos_posix_api::sys_pipe;
use axerrno::LinuxError;

use crate::utils::e;

/// Create a pipe
///
/// Return 0 if succeed
///
/// # Safety
///
/// `fd` must either be null, or point to writable storage for two `c_int`
/// values where the resulting pipe file descriptors can be written.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pipe(fd: *mut c_int) -> c_int {
    if fd.is_null() {
        return e((LinuxError::EFAULT as c_int).wrapping_neg());
    }

    let mut fds = [0; 2];
    let ret = e(sys_pipe(&mut fds));
    if ret != 0 {
        return ret;
    }

    unsafe {
        ptr::write(fd, fds[0]);
        ptr::write(fd.add(1), fds[1]);
    }
    0
}
