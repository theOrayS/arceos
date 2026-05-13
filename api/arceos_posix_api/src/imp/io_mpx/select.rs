use core::ffi::{c_int, c_ulong};

use axerrno::{LinuxError, LinuxResult};
use axhal::time::wall_time;

use crate::{ctypes, imp::fd_ops::get_file_like};

const FD_SETSIZE: usize = 1024;
const BITS_PER_WORD: usize = c_ulong::BITS as usize;
const FD_SETSIZE_WORDS: usize = FD_SETSIZE.div_ceil(BITS_PER_WORD);
const READ_SET: usize = 0;
const WRITE_SET: usize = 1;
const EXCEPT_SET: usize = 2;
const FD_SET_GROUPS: usize = 3;

struct FdSets {
    nfds: usize,
    bits: [[c_ulong; FD_SETSIZE_WORDS]; FD_SET_GROUPS],
}

impl FdSets {
    fn empty(nfds: usize) -> Self {
        Self {
            nfds: nfds.min(FD_SETSIZE),
            bits: [[0; FD_SETSIZE_WORDS]; FD_SET_GROUPS],
        }
    }

    fn from(
        nfds: usize,
        read_fds: *const ctypes::fd_set,
        write_fds: *const ctypes::fd_set,
        except_fds: *const ctypes::fd_set,
    ) -> Self {
        let mut sets = Self::empty(nfds);
        sets.copy_from_fd_set(READ_SET, read_fds);
        sets.copy_from_fd_set(WRITE_SET, write_fds);
        sets.copy_from_fd_set(EXCEPT_SET, except_fds);
        sets
    }

    fn nfds_words(&self) -> usize {
        self.nfds.div_ceil(BITS_PER_WORD)
    }

    fn clear(&mut self) {
        let words = self.nfds_words();
        for set in &mut self.bits {
            set[..words].fill(0);
        }
    }

    fn copy_from_fd_set(&mut self, set_idx: usize, fds: *const ctypes::fd_set) {
        if fds.is_null() {
            return;
        }
        let words = self.nfds_words();
        let src = unsafe { &(*fds).fds_bits[..words] };
        self.bits[set_idx][..words].copy_from_slice(src);
    }

    fn set_fd(&mut self, set_idx: usize, fd: usize) {
        self.bits[set_idx][fd / BITS_PER_WORD] |= 1 << (fd % BITS_PER_WORD);
    }

    unsafe fn write_back_to(
        &self,
        read_fds: *mut ctypes::fd_set,
        write_fds: *mut ctypes::fd_set,
        except_fds: *mut ctypes::fd_set,
    ) {
        unsafe {
            self.copy_to_fd_set(READ_SET, read_fds);
            self.copy_to_fd_set(WRITE_SET, write_fds);
            self.copy_to_fd_set(EXCEPT_SET, except_fds);
        }
    }

    unsafe fn copy_to_fd_set(&self, set_idx: usize, fds: *mut ctypes::fd_set) {
        if fds.is_null() {
            return;
        }
        let words = self.nfds_words();
        unsafe {
            (*fds).fds_bits[..words].copy_from_slice(&self.bits[set_idx][..words]);
        }
    }

    fn poll_all(&self, result_sets: &mut FdSets) -> LinuxResult<usize> {
        result_sets.clear();
        let mut res_num = 0;
        for word_idx in 0..self.nfds_words() {
            let read_bits = self.bits[READ_SET][word_idx];
            let write_bits = self.bits[WRITE_SET][word_idx];
            let except_bits = self.bits[EXCEPT_SET][word_idx];

            let all_bits = read_bits | write_bits | except_bits;
            if all_bits == 0 {
                continue;
            }
            let mut j = 0;
            let fd_base = word_idx * BITS_PER_WORD;
            while j < BITS_PER_WORD && fd_base + j < self.nfds {
                let bit = 1 << j;
                if all_bits & bit == 0 {
                    j += 1;
                    continue;
                }
                let fd = fd_base + j;
                match get_file_like(fd as _)?.poll() {
                    Ok(state) => {
                        if state.readable && read_bits & bit != 0 {
                            result_sets.set_fd(READ_SET, fd);
                            res_num += 1;
                        }
                        if state.writable && write_bits & bit != 0 {
                            result_sets.set_fd(WRITE_SET, fd);
                            res_num += 1;
                        }
                    }
                    Err(e) => {
                        debug!("    except: {} {:?}", fd, e);
                        if except_bits & bit != 0 {
                            result_sets.set_fd(EXCEPT_SET, fd);
                            res_num += 1;
                        }
                    }
                }
                j += 1;
            }
        }
        Ok(res_num)
    }
}

/// Monitor multiple file descriptors, waiting until one or more of the file descriptors become "ready" for some class of I/O operation
///
/// # Safety
///
/// Any non-null fd-set pointer must be valid for both reads and writes of the
/// fd-set words covered by `nfds`; a non-null `timeout` must be valid for reads
/// of one `timeval`.
pub unsafe fn sys_select(
    nfds: c_int,
    readfds: *mut ctypes::fd_set,
    writefds: *mut ctypes::fd_set,
    exceptfds: *mut ctypes::fd_set,
    timeout: *mut ctypes::timeval,
) -> c_int {
    debug!(
        "sys_select <= {} {:#x} {:#x} {:#x}",
        nfds, readfds as usize, writefds as usize, exceptfds as usize
    );
    syscall_body!(sys_select, {
        if nfds < 0 {
            return Err(LinuxError::EINVAL);
        }
        let nfds = (nfds as usize).min(FD_SETSIZE);
        let deadline = unsafe { timeout.as_ref().map(|t| wall_time() + (*t).into()) };
        let fd_sets = FdSets::from(nfds, readfds, writefds, exceptfds);
        let mut result_sets = FdSets::empty(nfds);

        loop {
            #[cfg(feature = "net")]
            axnet::poll_interfaces();
            let res = match fd_sets.poll_all(&mut result_sets) {
                Ok(res) => res,
                Err(err) => {
                    unsafe { result_sets.write_back_to(readfds, writefds, exceptfds) };
                    return Err(err);
                }
            };
            if res > 0 {
                unsafe { result_sets.write_back_to(readfds, writefds, exceptfds) };
                return Ok(res);
            }

            if deadline.is_some_and(|ddl| wall_time() >= ddl) {
                debug!("    timeout!");
                unsafe { result_sets.write_back_to(readfds, writefds, exceptfds) };
                return Ok(0);
            }
            crate::sys_sched_yield();
        }
    })
}
