#[cfg(feature = "uspace")]
use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use axerrno::LinuxError;
#[cfg(feature = "uspace")]
use axsync::Mutex;
use core::ffi::{c_int, c_long};
use core::time::Duration;
#[cfg(feature = "uspace")]
use lazyinit::LazyInit;
#[cfg(feature = "uspace")]
use linux_raw_sys::general;

use crate::ctypes;
use crate::ctypes::{CLOCK_MONOTONIC, CLOCK_REALTIME};

#[cfg(feature = "uspace")]
type TimerCallback = Arc<dyn Fn() + Send + Sync>;

#[cfg(feature = "uspace")]
struct RealTimerState {
    generation: u64,
    deadline: axhal::time::TimeValue,
    interval: Duration,
    callback: TimerCallback,
}

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

/// Get clock time since booting
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
        unsafe { *ts = now };
        debug!("sys_clock_gettime: {}.{:09}s", now.tv_sec, now.tv_nsec);
        Ok(0)
    })
}

/// Sleep some nanoseconds
///
/// TODO: should be woken by signals, and set errno
pub unsafe fn sys_nanosleep(req: *const ctypes::timespec, rem: *mut ctypes::timespec) -> c_int {
    syscall_body!(sys_nanosleep, {
        unsafe {
            if req.is_null() || (*req).tv_nsec < 0 || (*req).tv_nsec > 999999999 {
                return Err(LinuxError::EINVAL);
            }
        }

        let dur = unsafe {
            debug!("sys_nanosleep <= {}.{:09}s", (*req).tv_sec, (*req).tv_nsec);
            Duration::from(*req)
        };

        let now = axhal::time::monotonic_time();

        #[cfg(feature = "multitask")]
        axtask::sleep(dur);
        #[cfg(not(feature = "multitask"))]
        axhal::time::busy_wait(dur);

        let after = axhal::time::monotonic_time();
        let actual = after - now;

        if let Some(diff) = dur.checked_sub(actual) {
            if !rem.is_null() {
                unsafe { (*rem) = diff.into() };
            }
            return Err(LinuxError::EINTR);
        }
        Ok(0)
    })
}

#[cfg(feature = "uspace")]
pub(crate) fn timespec_to_duration(ts: general::timespec) -> Result<Duration, LinuxError> {
    if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000 {
        return Err(LinuxError::EINVAL);
    }
    Ok(Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32))
}

#[cfg(feature = "uspace")]
pub(crate) fn clock_now_duration(clockid: u32) -> Result<Duration, LinuxError> {
    match clockid {
        general::CLOCK_REALTIME | general::CLOCK_REALTIME_COARSE | general::CLOCK_TAI => {
            Ok(axhal::time::wall_time())
        }
        general::CLOCK_MONOTONIC
        | general::CLOCK_MONOTONIC_RAW
        | general::CLOCK_MONOTONIC_COARSE
        | general::CLOCK_BOOTTIME
        | general::CLOCK_PROCESS_CPUTIME_ID
        | general::CLOCK_THREAD_CPUTIME_ID => Ok(axhal::time::monotonic_time()),
        general::CLOCK_REALTIME_ALARM | general::CLOCK_BOOTTIME_ALARM => Err(LinuxError::EINVAL),
        _ => Err(LinuxError::EINVAL),
    }
}

#[cfg(feature = "uspace")]
pub(crate) fn clock_gettime_value(clockid: u32) -> Result<general::timespec, LinuxError> {
    let now = clock_now_duration(clockid)?;
    Ok(general::timespec {
        tv_sec: now.as_secs() as _,
        tv_nsec: now.subsec_nanos() as _,
    })
}

#[cfg(feature = "uspace")]
pub(crate) fn clock_getres_value(clockid: u32) -> Result<general::timespec, LinuxError> {
    clock_now_duration(clockid)?;
    Ok(general::timespec {
        tv_sec: 0,
        tv_nsec: 1,
    })
}

#[cfg(feature = "uspace")]
pub(crate) fn gettimeofday_values() -> (general::timeval, general::timezone) {
    let now = axhal::time::wall_time();
    (
        general::timeval {
            tv_sec: now.as_secs() as _,
            tv_usec: now.subsec_micros() as _,
        },
        general::timezone {
            tz_minuteswest: 0,
            tz_dsttime: 0,
        },
    )
}

#[cfg(feature = "uspace")]
fn real_timer_states() -> &'static Mutex<BTreeMap<i32, RealTimerState>> {
    static REAL_TIMERS: LazyInit<Mutex<BTreeMap<i32, RealTimerState>>> = LazyInit::new();
    REAL_TIMERS.call_once(|| Mutex::new(BTreeMap::new()));
    &REAL_TIMERS
}

#[cfg(feature = "uspace")]
fn duration_from_timeval(tv: general::timeval) -> Result<Duration, LinuxError> {
    if tv.tv_sec < 0 || tv.tv_usec < 0 || tv.tv_usec >= 1_000_000 {
        return Err(LinuxError::EINVAL);
    }
    Ok(Duration::new(tv.tv_sec as u64, (tv.tv_usec as u32) * 1000))
}

#[cfg(feature = "uspace")]
fn timeval_from_duration(duration: Duration) -> general::timeval {
    general::timeval {
        tv_sec: duration.as_secs() as _,
        tv_usec: duration.subsec_micros() as _,
    }
}

#[cfg(feature = "uspace")]
fn zero_itimerval() -> general::itimerval {
    general::itimerval {
        it_interval: timeval_from_duration(Duration::ZERO),
        it_value: timeval_from_duration(Duration::ZERO),
    }
}

#[cfg(feature = "uspace")]
fn itimerval_from_state(state: &RealTimerState, now: axhal::time::TimeValue) -> general::itimerval {
    let remaining = state.deadline.saturating_sub(now);
    general::itimerval {
        it_interval: timeval_from_duration(state.interval),
        it_value: timeval_from_duration(remaining),
    }
}

#[cfg(feature = "uspace")]
fn collect_due_real_timers(now: axhal::time::TimeValue) -> Vec<TimerCallback> {
    let mut callbacks = Vec::new();
    let mut states = real_timer_states().lock();
    let due_pids = states
        .iter()
        .filter_map(|(&pid, state)| (state.deadline <= now).then_some(pid))
        .collect::<Vec<_>>();
    for pid in due_pids {
        let Some(state) = states.get_mut(&pid) else {
            continue;
        };
        callbacks.push(state.callback.clone());
        if state.interval.is_zero() {
            states.remove(&pid);
        } else {
            state.deadline = now + state.interval;
        }
    }
    callbacks
}

#[cfg(feature = "uspace")]
fn fire_due_real_timers(now: axhal::time::TimeValue) {
    for callback in collect_due_real_timers(now) {
        callback();
    }
}

#[cfg(feature = "uspace")]
pub(crate) fn poll_real_timers() {
    fire_due_real_timers(axhal::time::wall_time());
}

#[cfg(feature = "uspace")]
fn run_real_timer(pid: i32, generation: u64) {
    loop {
        let (deadline, callback) = {
            let states = real_timer_states().lock();
            let Some(state) = states.get(&pid) else {
                return;
            };
            if state.generation != generation {
                return;
            }
            (state.deadline, state.callback.clone())
        };

        let now = axhal::time::wall_time();
        if deadline > now {
            axtask::sleep_until(deadline);
        }

        let next_deadline = {
            let now = axhal::time::wall_time();
            let mut states = real_timer_states().lock();
            let Some(state) = states.get_mut(&pid) else {
                return;
            };
            if state.generation != generation {
                return;
            }
            if state.deadline > now {
                continue;
            }
            if state.interval.is_zero() {
                states.remove(&pid);
                None
            } else {
                let next = now + state.interval;
                state.deadline = next;
                Some(next)
            }
        };

        callback();

        if next_deadline.is_none() {
            return;
        }
    }
}

#[cfg(feature = "uspace")]
pub(crate) fn set_real_interval_timer<F>(
    pid: i32,
    value: general::itimerval,
    callback: F,
) -> Result<general::itimerval, LinuxError>
where
    F: Fn() + Send + Sync + 'static,
{
    let interval = duration_from_timeval(value.it_interval)?;
    let initial = duration_from_timeval(value.it_value)?;
    let now = axhal::time::wall_time();
    let callback: TimerCallback = Arc::new(callback);

    let old_value = {
        let states = real_timer_states().lock();
        states
            .get(&pid)
            .map(|state| itimerval_from_state(state, now))
            .unwrap_or_else(zero_itimerval)
    };

    if initial.is_zero() {
        real_timer_states().lock().remove(&pid);
        return Ok(old_value);
    }

    let generation = {
        let mut states = real_timer_states().lock();
        let generation = states
            .get(&pid)
            .map(|state| state.generation.wrapping_add(1))
            .unwrap_or(1);
        states.insert(
            pid,
            RealTimerState {
                generation,
                deadline: now + initial,
                interval,
                callback,
            },
        );
        generation
    };

    axtask::spawn(move || run_real_timer(pid, generation));
    Ok(old_value)
}

#[cfg(feature = "uspace")]
pub(crate) fn clear_process_interval_timer(pid: i32) {
    real_timer_states().lock().remove(&pid);
}
