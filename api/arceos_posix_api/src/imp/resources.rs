use crate::ctypes;
use axerrno::LinuxError;
use core::ffi::c_int;

fn current_rlimit(resource: u32) -> Option<ctypes::rlimit> {
    match resource {
        ctypes::RLIMIT_STACK => Some(ctypes::rlimit {
            rlim_cur: axconfig::TASK_STACK_SIZE as _,
            rlim_max: axconfig::TASK_STACK_SIZE as _,
        }),
        #[cfg(feature = "fd")]
        ctypes::RLIMIT_NOFILE => Some(ctypes::rlimit {
            rlim_cur: super::fd_ops::AX_FILE_LIMIT as _,
            rlim_max: super::fd_ops::AX_FILE_LIMIT as _,
        }),
        _ => None,
    }
}

unsafe fn write_rlimit_output(rlimits: *mut ctypes::rlimit, value: ctypes::rlimit) {
    unsafe { core::ptr::write_unaligned(rlimits, value) }
}

/// Get resource limitations
///
/// TODO: support more resource types
///
/// # Safety
///
/// `rlimits` must be writable for one `rlimit` value when non-null.
pub unsafe fn sys_getrlimit(resource: c_int, rlimits: *mut ctypes::rlimit) -> c_int {
    debug!("sys_getrlimit <= {} {:#x}", resource, rlimits as usize);
    syscall_body!(sys_getrlimit, {
        match resource as u32 {
            ctypes::RLIMIT_DATA => {}
            ctypes::RLIMIT_STACK => {}
            ctypes::RLIMIT_NOFILE => {}
            _ => return Err(LinuxError::EINVAL),
        }
        if rlimits.is_null() {
            return Ok(0);
        }
        if let Some(limit) = current_rlimit(resource as u32) {
            unsafe { write_rlimit_output(rlimits, limit) };
        }
        Ok(0)
    })
}

/// Set resource limitations
///
/// TODO: support more resource types
///
/// # Safety
///
/// `rlimits` must point to a readable `rlimit` value when non-null.
pub unsafe fn sys_setrlimit(resource: c_int, rlimits: *mut crate::ctypes::rlimit) -> c_int {
    debug!("sys_setrlimit <= {} {:#x}", resource, rlimits as usize);
    syscall_body!(sys_setrlimit, {
        match resource as u32 {
            crate::ctypes::RLIMIT_DATA => {}
            crate::ctypes::RLIMIT_STACK => {}
            crate::ctypes::RLIMIT_NOFILE => {}
            _ => return Err(LinuxError::EINVAL),
        }
        // Currently do not support set resources
        Ok(0)
    })
}
