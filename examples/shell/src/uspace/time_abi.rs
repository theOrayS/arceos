use core::ffi::c_long;
use core::sync::atomic::{AtomicI64, Ordering};

use axerrno::LinuxError;
use linux_raw_sys::general;

use super::UserProcess;
use super::user_memory::read_user_value;

static REALTIME_OFFSET_NS: AtomicI64 = AtomicI64::new(0);

const NSEC_PER_SEC: i128 = 1_000_000_000;
pub(super) const USER_HZ: c_long = 100;

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct Tms {
    pub(super) tms_utime: c_long,
    pub(super) tms_stime: c_long,
    pub(super) tms_cutime: c_long,
    pub(super) tms_cstime: c_long,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct RtcTime {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
    tm_wday: i32,
    tm_yday: i32,
    tm_isdst: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct UserTimex {
    pub(super) modes: u32,
    pub(super) offset: c_long,
    pub(super) freq: c_long,
    pub(super) maxerror: c_long,
    pub(super) esterror: c_long,
    pub(super) status: i32,
    pub(super) constant: c_long,
    pub(super) precision: c_long,
    pub(super) tolerance: c_long,
    pub(super) time: general::timeval,
    pub(super) tick: c_long,
    pub(super) ppsfreq: c_long,
    pub(super) jitter: c_long,
    pub(super) shift: i32,
    pub(super) stabil: c_long,
    pub(super) jitcnt: c_long,
    pub(super) calcnt: c_long,
    pub(super) errcnt: c_long,
    pub(super) stbcnt: c_long,
    pub(super) tai: i32,
    pub(super) __padding: [i32; 11],
}

pub(super) fn socket_timeval_to_duration(
    value: general::timeval,
) -> Result<Option<core::time::Duration>, LinuxError> {
    if value.tv_sec < 0 || value.tv_usec < 0 || value.tv_usec >= 1_000_000 {
        return Err(LinuxError::EINVAL);
    }
    if value.tv_sec == 0 && value.tv_usec == 0 {
        Ok(None)
    } else {
        Ok(Some(core::time::Duration::new(
            value.tv_sec as u64,
            value.tv_usec as u32 * 1000,
        )))
    }
}

pub(super) fn socket_duration_to_timeval(
    timeout: Option<core::time::Duration>,
) -> general::timeval {
    match timeout {
        Some(timeout) => general::timeval {
            tv_sec: timeout.as_secs().min(i64::MAX as u64) as _,
            tv_usec: timeout.subsec_micros() as _,
        },
        None => general::timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
    }
}

fn duration_to_micros(duration: core::time::Duration) -> u64 {
    duration
        .as_secs()
        .saturating_mul(1_000_000)
        .saturating_add(duration.subsec_micros() as u64)
}

pub(super) fn micros_to_duration(micros: u64) -> core::time::Duration {
    core::time::Duration::new(micros / 1_000_000, ((micros % 1_000_000) as u32) * 1000)
}

pub(super) fn timeval_to_micros(value: general::timeval) -> Result<u64, LinuxError> {
    Ok(socket_timeval_to_duration(value)?
        .map(duration_to_micros)
        .unwrap_or(0))
}

pub(super) fn micros_to_timeval(micros: u64) -> general::timeval {
    general::timeval {
        tv_sec: (micros / 1_000_000).min(i64::MAX as u64) as _,
        tv_usec: (micros % 1_000_000) as _,
    }
}

pub(super) fn timespec_from_duration(duration: core::time::Duration) -> general::timespec {
    general::timespec {
        tv_sec: duration.as_secs() as _,
        tv_nsec: duration.subsec_nanos() as _,
    }
}

pub(super) fn clock_gettime_timespec(clockid: u32) -> Result<general::timespec, LinuxError> {
    clock_now_duration(clockid).map(timespec_from_duration)
}

pub(super) fn timeval_from_duration(duration: core::time::Duration) -> general::timeval {
    general::timeval {
        tv_sec: duration.as_secs() as _,
        tv_usec: duration.subsec_micros() as _,
    }
}

pub(super) fn clock_resolution_timespec() -> general::timespec {
    general::timespec {
        tv_sec: 0,
        tv_nsec: 1,
    }
}

pub(super) fn clock_getres_timespec(clockid: u32) -> Result<general::timespec, LinuxError> {
    validate_clock_id(clockid)?;
    Ok(clock_resolution_timespec())
}

pub(super) fn zero_timespec() -> general::timespec {
    general::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    }
}

pub(super) fn zero_timezone() -> general::timezone {
    general::timezone {
        tz_minuteswest: 0,
        tz_dsttime: 0,
    }
}

pub(super) fn current_timeval() -> general::timeval {
    timeval_from_duration(adjusted_wall_time())
}

pub(super) fn default_tms() -> Tms {
    Tms {
        tms_utime: 0,
        tms_stime: 0,
        tms_cutime: 0,
        tms_cstime: 0,
    }
}

pub(super) fn monotonic_time_micros() -> u64 {
    axhal::time::monotonic_time()
        .as_micros()
        .min(u64::MAX as u128) as u64
}

pub(super) fn times_ticks() -> isize {
    axhal::time::monotonic_time().as_millis() as isize
}

pub(super) fn timespec_to_duration(
    ts: general::timespec,
) -> Result<core::time::Duration, LinuxError> {
    if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000 {
        return Err(LinuxError::EINVAL);
    }
    Ok(core::time::Duration::new(
        ts.tv_sec as u64,
        ts.tv_nsec as u32,
    ))
}

pub(super) fn clock_now_duration(clockid: u32) -> Result<core::time::Duration, LinuxError> {
    match clockid {
        general::CLOCK_REALTIME | general::CLOCK_REALTIME_COARSE | general::CLOCK_TAI => {
            Ok(adjusted_wall_time())
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

pub(super) fn adjusted_wall_time() -> core::time::Duration {
    let raw_ns = duration_to_ns_i128(axhal::time::wall_time());
    let offset_ns = REALTIME_OFFSET_NS.load(Ordering::Acquire) as i128;
    let adjusted_ns = raw_ns + offset_ns;
    if adjusted_ns <= 0 {
        return core::time::Duration::ZERO;
    }
    let secs = (adjusted_ns / NSEC_PER_SEC).min(u64::MAX as i128) as u64;
    let nanos = (adjusted_ns % NSEC_PER_SEC) as u32;
    core::time::Duration::new(secs, nanos)
}

pub(super) fn set_realtime_offset_from_timespec(ts: general::timespec) {
    let target_ns = ts.tv_sec as i128 * NSEC_PER_SEC + ts.tv_nsec as i128;
    let raw_ns = duration_to_ns_i128(axhal::time::wall_time());
    REALTIME_OFFSET_NS.store(clamp_i128_to_i64(target_ns - raw_ns), Ordering::Release);
}

fn duration_to_ns_i128(duration: core::time::Duration) -> i128 {
    duration.as_secs() as i128 * NSEC_PER_SEC + duration.subsec_nanos() as i128
}

fn clamp_i128_to_i64(value: i128) -> i64 {
    value.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

pub(super) fn rtc_time_from_wall_time() -> RtcTime {
    let now = adjusted_wall_time();
    let total_secs = now.as_secs() as i64;
    let days = total_secs.div_euclid(86_400);
    let secs_of_day = total_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);

    RtcTime {
        tm_sec: (secs_of_day % 60) as i32,
        tm_min: ((secs_of_day / 60) % 60) as i32,
        tm_hour: (secs_of_day / 3600) as i32,
        tm_mday: day,
        tm_mon: month - 1,
        tm_year: year - 1900,
        tm_wday: (days + 4).rem_euclid(7) as i32,
        tm_yday: year_day(year, month, day),
        tm_isdst: 0,
    }
}

fn civil_from_days(days: i64) -> (i32, i32, i32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year as i32, month as i32, day as i32)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn year_day(year: i32, month: i32, day: i32) -> i32 {
    const DAYS_BEFORE_MONTH: [i32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let mut yday = DAYS_BEFORE_MONTH[(month - 1) as usize] + day - 1;
    if month > 2 && is_leap_year(year) {
        yday += 1;
    }
    yday
}

pub(super) fn validate_clock_id(clockid: u32) -> Result<(), LinuxError> {
    clock_now_duration(clockid).map(|_| ())
}

pub(super) fn adjtimex_input_valid(input: UserTimex) -> bool {
    let modes = input.modes;
    if !adjtimex_modes_valid(modes) {
        return false;
    }
    if modes & ADJ_TICK != 0 {
        return adjtimex_tick_valid(input.tick);
    }
    true
}

pub(super) fn adjtimex_changes_clock(input: UserTimex) -> bool {
    input.modes != 0
}

pub(super) fn default_timex() -> UserTimex {
    let now = adjusted_wall_time();
    let mut output: UserTimex = unsafe { core::mem::zeroed() };
    output.precision = 1;
    output.time = timeval_from_duration(now);
    output.tick = 10_000;
    output
}

pub(super) fn itimerval_to_micros_pair(
    value: general::itimerval,
) -> Result<(u64, u64), LinuxError> {
    let first_us = timeval_to_micros(value.it_value)?;
    let interval_us = timeval_to_micros(value.it_interval)?;
    Ok((first_us, interval_us))
}

pub(super) fn read_timespec_duration(
    process: &UserProcess,
    ptr: usize,
) -> Result<core::time::Duration, LinuxError> {
    let ts = read_user_value::<general::timespec>(process, ptr)?;
    timespec_to_duration(ts)
}

pub(super) fn sleep_duration(duration: core::time::Duration) {
    if duration.as_nanos() == 0 {
        return;
    }
    let deadline = axhal::time::wall_time() + duration;
    while axhal::time::wall_time() < deadline {
        axtask::yield_now();
    }
}

const ADJ_OFFSET: u32 = 0x0001;
const ADJ_FREQUENCY: u32 = 0x0002;
const ADJ_MAXERROR: u32 = 0x0004;
const ADJ_ESTERROR: u32 = 0x0008;
const ADJ_STATUS: u32 = 0x0010;
const ADJ_TIMECONST: u32 = 0x0020;
const ADJ_TAI: u32 = 0x0080;
const ADJ_SETOFFSET: u32 = 0x0100;
const ADJ_MICRO: u32 = 0x1000;
const ADJ_NANO: u32 = 0x2000;
const ADJ_TICK: u32 = 0x4000;
const ADJ_OFFSET_SINGLESHOT: u32 = 0x8001;
const ADJ_OFFSET_SS_READ: u32 = 0xa001;

const ADJ_REGULAR_MASK: u32 = ADJ_OFFSET
    | ADJ_FREQUENCY
    | ADJ_MAXERROR
    | ADJ_ESTERROR
    | ADJ_STATUS
    | ADJ_TIMECONST
    | ADJ_TAI
    | ADJ_SETOFFSET
    | ADJ_MICRO
    | ADJ_NANO
    | ADJ_TICK;

fn adjtimex_modes_valid(modes: u32) -> bool {
    modes & !ADJ_REGULAR_MASK == 0 || modes == ADJ_OFFSET_SINGLESHOT || modes == ADJ_OFFSET_SS_READ
}

fn adjtimex_tick_valid(tick: c_long) -> bool {
    let min_tick = 900_000 / USER_HZ;
    let max_tick = 1_100_000 / USER_HZ;
    tick >= min_tick && tick <= max_tick
}
