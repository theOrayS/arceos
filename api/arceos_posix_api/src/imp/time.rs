use axerrno::{LinuxError, LinuxResult};
use core::ffi::{c_int, c_long};
use core::time::Duration;

use crate::ctypes;
use crate::ctypes::{CLOCK_MONOTONIC, CLOCK_REALTIME};

impl From<ctypes::timespec> for Duration {
    fn from(ts: ctypes::timespec) -> Self {
        Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
    }
}

impl From<ctypes::timeval> for Duration {
    fn from(tv: ctypes::timeval) -> Self {
        Duration::new(tv.tv_sec as u64, tv.tv_usec as u32 * 1000)
    }
}

impl From<Duration> for ctypes::timespec {
    fn from(d: Duration) -> Self {
        ctypes::timespec {
            tv_sec: d.as_secs() as c_long,
            tv_nsec: d.subsec_nanos() as c_long,
        }
    }
}

impl From<Duration> for ctypes::timeval {
    fn from(d: Duration) -> Self {
        ctypes::timeval {
            tv_sec: d.as_secs() as c_long,
            tv_usec: d.subsec_micros() as c_long,
        }
    }
}

unsafe fn read_nanosleep_request(req: *const ctypes::timespec) -> LinuxResult<ctypes::timespec> {
    if req.is_null() {
        return Err(LinuxError::EINVAL);
    }

    let req = unsafe { core::ptr::read_unaligned(req) };
    if req.tv_nsec < 0 || req.tv_nsec > 999_999_999 {
        return Err(LinuxError::EINVAL);
    }
    Ok(req)
}

unsafe fn write_timespec(dst: *mut ctypes::timespec, value: ctypes::timespec) {
    unsafe { core::ptr::write_unaligned(dst, value) };
}

unsafe fn write_optional_timespec(dst: *mut ctypes::timespec, value: ctypes::timespec) {
    if !dst.is_null() {
        unsafe { write_timespec(dst, value) };
    }
}

/// Get clock time since booting
///
/// # Safety
///
/// `ts` must be writable for one `timespec` value.
pub unsafe fn sys_clock_gettime(clk: ctypes::clockid_t, ts: *mut ctypes::timespec) -> c_int {
    syscall_body!(sys_clock_gettime, {
        if ts.is_null() {
            return Err(LinuxError::EFAULT);
        }
        let now = match clk as u32 {
            CLOCK_REALTIME => axhal::time::wall_time().into(),
            CLOCK_MONOTONIC => axhal::time::monotonic_time().into(),
            _ => {
                warn!("Called sys_clock_gettime for unsupported clock {}", clk);
                return Err(LinuxError::EINVAL);
            }
        };
        unsafe { write_timespec(ts, now) };
        debug!("sys_clock_gettime: {}.{:09}s", now.tv_sec, now.tv_nsec);
        Ok(0)
    })
}

/// Sleep some nanoseconds
///
/// TODO: should be woken by signals, and set errno
///
/// # Safety
///
/// `req` must point to a readable `timespec`. `rem` must be writable for one
/// `timespec` value when non-null.
pub unsafe fn sys_nanosleep(req: *const ctypes::timespec, rem: *mut ctypes::timespec) -> c_int {
    syscall_body!(sys_nanosleep, {
        let req = unsafe { read_nanosleep_request(req)? };

        let dur = {
            debug!("sys_nanosleep <= {}.{:09}s", req.tv_sec, req.tv_nsec);
            Duration::from(req)
        };

        let now = axhal::time::monotonic_time();

        #[cfg(feature = "multitask")]
        axtask::sleep(dur);
        #[cfg(not(feature = "multitask"))]
        axhal::time::busy_wait(dur);

        let after = axhal::time::monotonic_time();
        let actual = after - now;

        if let Some(diff) = dur.checked_sub(actual) {
            unsafe { write_optional_timespec(rem, diff.into()) };
            return Err(LinuxError::EINTR);
        }
        Ok(0)
    })
}
