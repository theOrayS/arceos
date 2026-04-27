use crate::{ctypes, utils::check_null_mut_ptr};

use axerrno::LinuxResult;

use core::ffi::c_int;
use core::mem::size_of;
use core::sync::atomic::{AtomicU64, Ordering};

static_assertions::const_assert_eq!(
    size_of::<ctypes::pthread_mutex_t>(),
    size_of::<PthreadMutex>()
);

#[repr(C)]
pub struct PthreadMutex(AtomicU64);

impl PthreadMutex {
    const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    fn lock(&self) -> LinuxResult {
        let current_id = axtask::current().id().as_u64();
        loop {
            match self
                .0
                .compare_exchange_weak(0, current_id, Ordering::Acquire, Ordering::Relaxed)
            {
                Ok(_) => return Ok(()),
                Err(owner_id) => {
                    assert_ne!(
                        owner_id, current_id,
                        "pthread mutex already owned by current task"
                    );
                    axtask::yield_now();
                }
            }
        }
    }

    fn unlock(&self) -> LinuxResult {
        let current_id = axtask::current().id().as_u64();
        let owner_id = self.0.swap(0, Ordering::Release);
        assert_eq!(
            owner_id, current_id,
            "pthread mutex released by non-owner task"
        );
        Ok(())
    }
}

/// Initialize a mutex.
pub fn sys_pthread_mutex_init(
    mutex: *mut ctypes::pthread_mutex_t,
    _attr: *const ctypes::pthread_mutexattr_t,
) -> c_int {
    debug!("sys_pthread_mutex_init <= {:#x}", mutex as usize);
    syscall_body!(sys_pthread_mutex_init, {
        check_null_mut_ptr(mutex)?;
        unsafe {
            mutex.cast::<PthreadMutex>().write(PthreadMutex::new());
        }
        Ok(0)
    })
}

/// Lock the given mutex.
pub fn sys_pthread_mutex_lock(mutex: *mut ctypes::pthread_mutex_t) -> c_int {
    debug!("sys_pthread_mutex_lock <= {:#x}", mutex as usize);
    syscall_body!(sys_pthread_mutex_lock, {
        check_null_mut_ptr(mutex)?;
        unsafe {
            (*mutex.cast::<PthreadMutex>()).lock()?;
        }
        Ok(0)
    })
}

/// Unlock the given mutex.
pub fn sys_pthread_mutex_unlock(mutex: *mut ctypes::pthread_mutex_t) -> c_int {
    debug!("sys_pthread_mutex_unlock <= {:#x}", mutex as usize);
    syscall_body!(sys_pthread_mutex_unlock, {
        check_null_mut_ptr(mutex)?;
        unsafe {
            (*mutex.cast::<PthreadMutex>()).unlock()?;
        }
        Ok(0)
    })
}
