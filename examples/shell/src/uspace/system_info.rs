use core::cmp;

use axerrno::LinuxError;
use linux_raw_sys::{general, system};

use super::user_memory::write_user_value;
use super::{UserProcess, neg_errno};

pub(super) enum SyslogAction {
    EmptyRead,
    SizeBuffer,
    ConsoleControl,
    Invalid,
}

pub(super) fn syslog_action(log_type: i32) -> SyslogAction {
    match log_type {
        // SYSLOG_ACTION_READ_ALL and READ_CLEAR. Expose an empty kernel log.
        3 | 4 => SyslogAction::EmptyRead,
        // SYSLOG_ACTION_SIZE_BUFFER.
        10 => SyslogAction::SizeBuffer,
        // Console control operations are accepted as no-ops.
        6..=8 => SyslogAction::ConsoleControl,
        _ => SyslogAction::Invalid,
    }
}

pub(super) fn syslog_empty_read_bytes(buf: usize, len: usize) -> Option<&'static [u8]> {
    if len > 0 && buf != 0 {
        Some(&[0])
    } else {
        None
    }
}

fn default_rusage() -> general::rusage {
    unsafe { core::mem::zeroed() }
}

fn rusage_target_valid(who: i32) -> bool {
    who == general::RUSAGE_SELF as i32
        || who == general::RUSAGE_THREAD as i32
        || who == general::RUSAGE_CHILDREN
}

pub(super) fn write_default_rusage(process: &UserProcess, who: i32, usage: usize) -> isize {
    if !rusage_target_valid(who) {
        return neg_errno(LinuxError::EINVAL);
    }
    let value = default_rusage();
    write_user_value(process, usage, &value)
}

pub(super) fn default_winsize() -> general::winsize {
    general::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    }
}

fn default_utsname() -> system::new_utsname {
    let mut uts = system::new_utsname {
        sysname: [0; 65],
        nodename: [0; 65],
        release: [0; 65],
        version: [0; 65],
        machine: [0; 65],
        domainname: [0; 65],
    };
    write_c_string(&mut uts.sysname, b"Linux");
    write_c_string(&mut uts.nodename, b"arceos");
    write_c_string(&mut uts.release, b"6.0.0");
    write_c_string(&mut uts.version, b"ArceOS");
    #[cfg(target_arch = "riscv64")]
    write_c_string(&mut uts.machine, b"riscv64");
    #[cfg(target_arch = "loongarch64")]
    write_c_string(&mut uts.machine, b"loongarch64");
    write_c_string(&mut uts.domainname, b"localdomain");
    uts
}

pub(super) fn write_default_utsname(process: &UserProcess, buf: usize) -> isize {
    let value = default_utsname();
    write_user_value(process, buf, &value)
}

trait CCharSlot: Copy {
    fn from_byte(byte: u8) -> Self;
}

impl CCharSlot for u8 {
    fn from_byte(byte: u8) -> Self {
        byte
    }
}

impl CCharSlot for i8 {
    fn from_byte(byte: u8) -> Self {
        byte as i8
    }
}

fn write_c_string<T: CCharSlot>(dst: &mut [T], src: &[u8]) {
    let len = cmp::min(dst.len().saturating_sub(1), src.len());
    for (idx, byte) in src[..len].iter().enumerate() {
        dst[idx] = T::from_byte(*byte);
    }
    if !dst.is_empty() {
        dst[len] = T::from_byte(0);
    }
}
