use core::ffi::c_int;

use axerrno::LinuxError;
#[cfg(feature = "uspace")]
use linux_raw_sys::general;

/// Relinquish the CPU, and switches to another task.
///
/// For single-threaded configuration (`multitask` feature is disabled), we just
/// relax the CPU and wait for incoming interrupts.
pub fn sys_sched_yield() -> c_int {
    #[cfg(feature = "multitask")]
    axtask::yield_now();
    #[cfg(not(feature = "multitask"))]
    if cfg!(feature = "irq") {
        axhal::asm::wait_for_irqs();
    } else {
        core::hint::spin_loop();
    }
    0
}

/// Get current thread ID.
pub fn sys_getpid() -> c_int {
    syscall_body!(sys_getpid,
        #[cfg(feature = "multitask")]
        {
            Ok(axtask::current().id().as_u64() as c_int)
        }
        #[cfg(not(feature = "multitask"))]
        {
            Ok(2) // `main` task ID
        }
    )
}

/// Exit current task
pub fn sys_exit(exit_code: c_int) -> ! {
    debug!("sys_exit <= {}", exit_code);
    #[cfg(feature = "multitask")]
    axtask::exit(exit_code);
    #[cfg(not(feature = "multitask"))]
    axhal::power::system_off();
}

pub(crate) fn current_tid() -> c_int {
    #[cfg(feature = "multitask")]
    {
        axtask::current().id().as_u64() as c_int
    }
    #[cfg(not(feature = "multitask"))]
    {
        2
    }
}

#[cfg(feature = "uspace")]
pub(crate) fn is_same_sched_target(process_pid: i32, pid: i32) -> bool {
    pid == 0 || pid == current_tid() || pid == process_pid
}

#[cfg(feature = "uspace")]
pub(crate) fn validate_sched_param(priority: i32) -> Result<(), LinuxError> {
    if priority == 0 {
        Ok(())
    } else {
        Err(LinuxError::EINVAL)
    }
}

#[cfg(feature = "uspace")]
pub(crate) fn validate_scheduler(policy: u32, priority: i32) -> Result<c_int, LinuxError> {
    match policy {
        0 if priority == 0 => Ok(0),
        general::SCHED_FIFO | general::SCHED_RR if (1..=99).contains(&priority) => Ok(0),
        general::SCHED_BATCH | general::SCHED_IDLE if priority == 0 => Ok(0),
        _ => Err(LinuxError::EINVAL),
    }
}

#[cfg(feature = "uspace")]
pub(crate) const fn current_scheduler() -> c_int {
    0
}

#[cfg(feature = "uspace")]
pub(crate) fn setsid(process_pid: c_int) -> Result<c_int, LinuxError> {
    if process_pid <= 0 {
        Err(LinuxError::EINVAL)
    } else {
        Ok(process_pid)
    }
}

pub(crate) fn current_affinity_size() -> usize {
    axhal::cpu_num().div_ceil(8).max(1)
}

pub(crate) fn set_current_affinity_from_bytes(src: &[u8]) -> Result<(), LinuxError> {
    #[cfg(feature = "multitask")]
    {
        let required = current_affinity_size();
        if src.len() < required {
            return Err(LinuxError::EINVAL);
        }
        let mut cpumask = axtask::AxCpuMask::new();
        for cpu in 0..axhal::cpu_num() {
            let byte = cpu / 8;
            let bit = cpu % 8;
            if src[byte] & (1u8 << bit) != 0 {
                cpumask.set(cpu, true);
            }
        }
        if cpumask.is_empty() {
            return Err(LinuxError::EINVAL);
        }
        if axtask::set_current_affinity(cpumask) {
            Ok(())
        } else {
            Err(LinuxError::EINVAL)
        }
    }
    #[cfg(not(feature = "multitask"))]
    {
        let _ = src;
        Err(LinuxError::ENOTSUP)
    }
}

pub(crate) fn current_affinity_to_bytes(dst: &mut [u8]) -> Result<usize, LinuxError> {
    let required = current_affinity_size();
    if dst.len() < required {
        return Err(LinuxError::EINVAL);
    }
    dst.fill(0);
    #[cfg(feature = "multitask")]
    {
        let cpumask = axtask::current().cpumask();
        for cpu in 0..axhal::cpu_num() {
            if cpumask.get(cpu) {
                let byte = cpu / 8;
                let bit = cpu % 8;
                dst[byte] |= 1u8 << bit;
            }
        }
    }
    #[cfg(not(feature = "multitask"))]
    {
        dst[0] = 1;
    }
    Ok(required)
}
