use axerrno::LinuxError;
use axfs::fops::{File, OpenOptions};
use axhal::time::{monotonic_time, wall_time};
#[cfg(feature = "uspace")]
use linux_raw_sys::{general, system};

pub(crate) const CLOCK_TICKS_PER_SECOND: usize = 100;

#[derive(Clone, Copy)]
pub(crate) struct SystemInfo {
    pub uptime: i64,
    pub loads: [u64; 3],
    pub totalram: u64,
    pub freeram: u64,
    pub sharedram: u64,
    pub bufferram: u64,
    pub totalswap: u64,
    pub freeswap: u64,
    pub procs: u16,
    pub totalhigh: u64,
    pub freehigh: u64,
    pub mem_unit: u32,
}

#[derive(Clone, Copy)]
pub(crate) struct TerminalSize {
    pub rows: u16,
    pub cols: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct RtcTime {
    pub tm_sec: i32,
    pub tm_min: i32,
    pub tm_hour: i32,
    pub tm_mday: i32,
    pub tm_mon: i32,
    pub tm_year: i32,
    pub tm_wday: i32,
    pub tm_yday: i32,
    pub tm_isdst: i32,
}

#[derive(Clone, Copy)]
pub(crate) struct UnameInfo {
    pub sysname: &'static str,
    pub nodename: &'static str,
    pub release: &'static str,
    pub version: &'static str,
    pub machine: &'static str,
    pub domainname: &'static str,
}

pub(crate) fn current_system_info() -> SystemInfo {
    let totalram = axhal::mem::total_ram_size() as u64;
    let freeram = free_ram_bytes();
    SystemInfo {
        uptime: monotonic_time().as_secs().min(i64::MAX as u64) as i64,
        loads: [0; 3],
        totalram,
        freeram,
        sharedram: 0,
        bufferram: 0,
        totalswap: 0,
        freeswap: 0,
        procs: 1,
        totalhigh: 0,
        freehigh: 0,
        mem_unit: 1,
    }
}

pub(crate) fn current_terminal_size() -> TerminalSize {
    TerminalSize { rows: 24, cols: 80 }
}

pub(crate) fn current_uname() -> UnameInfo {
    UnameInfo {
        sysname: "Linux",
        nodename: axconfig::PLATFORM,
        release: env!("CARGO_PKG_VERSION"),
        version: concat!("ArceOS ", env!("CARGO_PKG_VERSION")),
        machine: axconfig::ARCH,
        domainname: "localdomain",
    }
}

pub(crate) fn current_rtc_time() -> RtcTime {
    let unix_seconds = wall_time().as_secs() as i64;
    let days = unix_seconds.div_euclid(86_400);
    let secs_of_day = unix_seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let tm_hour = (secs_of_day / 3_600) as i32;
    let tm_min = ((secs_of_day % 3_600) / 60) as i32;
    let tm_sec = (secs_of_day % 60) as i32;
    RtcTime {
        tm_sec,
        tm_min,
        tm_hour,
        tm_mday: day as i32,
        tm_mon: month as i32 - 1,
        tm_year: year - 1900,
        tm_wday: days_since_sunday(days),
        tm_yday: day_of_year(year, month, day),
        tm_isdst: 0,
    }
}

#[cfg(feature = "uspace")]
pub(crate) fn getrandom(buf: &mut [u8], flags: u32) -> Result<usize, LinuxError> {
    let supported = general::GRND_NONBLOCK | general::GRND_RANDOM | general::GRND_INSECURE;
    if flags & !supported != 0 {
        return Err(LinuxError::EINVAL);
    }
    fill_random_bytes(buf)?;
    Ok(buf.len())
}

pub(crate) fn fill_random_bytes(buf: &mut [u8]) -> Result<(), LinuxError> {
    if buf.is_empty() {
        return Ok(());
    }
    let mut opts = OpenOptions::new();
    opts.read(true);
    let mut file = File::open("/dev/urandom", &opts).map_err(LinuxError::from)?;
    let mut filled = 0usize;
    while filled < buf.len() {
        let read = file.read(&mut buf[filled..]).map_err(LinuxError::from)?;
        if read == 0 {
            return Err(LinuxError::EIO);
        }
        filled += read;
    }
    Ok(())
}

pub(crate) fn kernel_log_len() -> usize {
    axlog::kernel_log_len()
}

pub(crate) fn read_kernel_log(buf: &mut [u8], clear: bool) -> usize {
    axlog::read_kernel_log(buf, clear)
}

#[cfg(feature = "uspace")]
pub(crate) fn syslog(log_type: i32, buf: Option<&mut [u8]>) -> Result<usize, LinuxError> {
    match log_type {
        3 | 4 => {
            let Some(dst) = buf else {
                return Err(LinuxError::EINVAL);
            };
            Ok(read_kernel_log(dst, log_type == 4))
        }
        10 => Ok(kernel_log_len()),
        6..=8 => Ok(0),
        _ => Err(LinuxError::EINVAL),
    }
}

pub(crate) fn monotonic_ticks() -> i64 {
    let now = monotonic_time();
    let secs = now.as_secs().saturating_mul(CLOCK_TICKS_PER_SECOND as u64);
    let frac =
        (now.subsec_nanos() as u64).saturating_mul(CLOCK_TICKS_PER_SECOND as u64) / 1_000_000_000;
    secs.saturating_add(frac).min(i64::MAX as u64) as i64
}

#[cfg(feature = "uspace")]
pub(crate) fn getrusage(who: i32) -> Result<general::rusage, LinuxError> {
    match who {
        x if x == general::RUSAGE_SELF as i32
            || x == general::RUSAGE_THREAD as i32
            || x == general::RUSAGE_CHILDREN => {}
        _ => return Err(LinuxError::EINVAL),
    }
    let ticks = monotonic_ticks();
    Ok(general::rusage {
        ru_utime: general::__kernel_old_timeval {
            tv_sec: ticks / CLOCK_TICKS_PER_SECOND as i64,
            tv_usec: ((ticks % CLOCK_TICKS_PER_SECOND as i64) * 1_000_000
                / CLOCK_TICKS_PER_SECOND as i64),
        },
        ru_stime: general::__kernel_old_timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        ..unsafe { core::mem::zeroed() }
    })
}

#[cfg(feature = "uspace")]
pub(crate) fn current_sysinfo() -> system::sysinfo {
    let value = current_system_info();
    system::sysinfo {
        uptime: value.uptime as _,
        loads: [
            value.loads[0] as _,
            value.loads[1] as _,
            value.loads[2] as _,
        ],
        totalram: value.totalram as _,
        freeram: value.freeram as _,
        sharedram: value.sharedram as _,
        bufferram: value.bufferram as _,
        totalswap: value.totalswap as _,
        freeswap: value.freeswap as _,
        procs: value.procs,
        pad: 0,
        totalhigh: value.totalhigh as _,
        freehigh: value.freehigh as _,
        mem_unit: value.mem_unit,
        _f: system::__IncompleteArrayField::new(),
    }
}

#[cfg(feature = "uspace")]
pub(crate) fn current_utsname() -> system::new_utsname {
    let uname = current_uname();
    let mut uts = system::new_utsname {
        sysname: [0; 65],
        nodename: [0; 65],
        release: [0; 65],
        version: [0; 65],
        machine: [0; 65],
        domainname: [0; 65],
    };
    write_c_string(&mut uts.sysname, uname.sysname.as_bytes());
    write_c_string(&mut uts.nodename, uname.nodename.as_bytes());
    write_c_string(&mut uts.release, uname.release.as_bytes());
    write_c_string(&mut uts.version, uname.version.as_bytes());
    write_c_string(&mut uts.machine, uname.machine.as_bytes());
    write_c_string(&mut uts.domainname, uname.domainname.as_bytes());
    uts
}

fn free_ram_bytes() -> u64 {
    #[cfg(feature = "alloc")]
    {
        return axalloc::global_allocator().available_pages() as u64 * 4096;
    }
    #[cfg(not(feature = "alloc"))]
    {
        axhal::mem::total_ram_size() as u64
    }
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

fn days_since_sunday(days: i64) -> i32 {
    (days + 4).rem_euclid(7) as i32
}

fn day_of_year(year: i32, month: u32, day: u32) -> i32 {
    const MONTH_DAYS: [u16; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let mut yday = MONTH_DAYS[(month - 1) as usize] as i32 + day as i32 - 1;
    if month > 2 && is_leap_year(year) {
        yday += 1;
    }
    yday
}

const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(feature = "uspace")]
trait CCharSlot: Copy {
    fn from_u8(byte: u8) -> Self;
}

#[cfg(feature = "uspace")]
impl CCharSlot for u8 {
    fn from_u8(byte: u8) -> Self {
        byte
    }
}

#[cfg(feature = "uspace")]
impl CCharSlot for i8 {
    fn from_u8(byte: u8) -> Self {
        byte as i8
    }
}

#[cfg(feature = "uspace")]
fn write_c_string<T: CCharSlot>(dst: &mut [T], src: &[u8]) {
    if dst.is_empty() {
        return;
    }
    let copy_len = src.len().min(dst.len() - 1);
    for (slot, byte) in dst.iter_mut().zip(src.iter().copied()).take(copy_len) {
        *slot = T::from_u8(byte);
    }
    for slot in &mut dst[copy_len..] {
        *slot = T::from_u8(0);
    }
}
