use axerrno::LinuxError;
use linux_raw_sys::general;

use super::linux_abi::{BITS_PER_USIZE, FD_SET_WORDS};
use super::user_memory::{read_user_value, write_user_value};
use super::{FdTable, UserProcess};

#[repr(C)]
#[derive(Clone, Copy)]
struct UserFdSet {
    fds_bits: [usize; FD_SET_WORDS],
}

#[derive(Clone, Copy)]
pub(super) enum SelectMode {
    Read,
    Write,
    Except,
}

pub(super) fn read_pselect_deadline(
    process: &UserProcess,
    timeout: usize,
) -> Result<Option<core::time::Duration>, LinuxError> {
    if timeout == 0 {
        return Ok(None);
    }
    let ts = read_user_value::<general::timespec>(process, timeout)?;
    if ts.tv_sec < 0 || !(0..1_000_000_000).contains(&ts.tv_nsec) {
        return Err(LinuxError::EINVAL);
    }
    Ok(Some(
        axhal::time::wall_time() + core::time::Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32),
    ))
}

pub(super) fn read_fd_set(
    process: &UserProcess,
    ptr: usize,
) -> Result<[usize; FD_SET_WORDS], LinuxError> {
    if ptr == 0 {
        return Ok([0; FD_SET_WORDS]);
    }
    Ok(read_user_value::<UserFdSet>(process, ptr)?.fds_bits)
}

pub(super) fn write_fd_set(
    process: &UserProcess,
    ptr: usize,
    bits: &[usize; FD_SET_WORDS],
) -> isize {
    if ptr == 0 {
        return 0;
    }
    write_user_value(process, ptr, &UserFdSet { fds_bits: *bits })
}

pub(super) fn poll_fd_set(
    table: &FdTable,
    nfds: usize,
    requested: &[usize; FD_SET_WORDS],
    ready: &mut [usize; FD_SET_WORDS],
    mode: SelectMode,
) -> usize {
    let mut count = 0usize;
    let words = nfds.div_ceil(BITS_PER_USIZE);
    for word_idx in 0..words {
        let mut bits = requested[word_idx];
        while bits != 0 {
            let bit_idx = bits.trailing_zeros() as usize;
            let fd = word_idx * BITS_PER_USIZE + bit_idx;
            if fd >= nfds {
                break;
            }
            if table.poll(fd as i32, mode) {
                ready[word_idx] |= 1usize << bit_idx;
                count += 1;
            }
            bits &= bits - 1;
        }
    }
    count
}
