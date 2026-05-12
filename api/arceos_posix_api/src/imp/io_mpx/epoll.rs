//! `epoll` implementation.
//!
//! TODO: do not support `EPOLLET` flag

use alloc::collections::BTreeMap;
use alloc::collections::btree_map::Entry;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::{ffi::c_int, time::Duration};

use axerrno::{LinuxError, LinuxResult};
use axhal::time::wall_time;
use axsync::Mutex;

use crate::ctypes;
use crate::imp::fd_ops::{FileLike, add_file_like, get_file_like};

pub struct EpollInstance {
    events: Mutex<BTreeMap<usize, ctypes::epoll_event>>,
}

unsafe impl Send for ctypes::epoll_event {}
unsafe impl Sync for ctypes::epoll_event {}

impl EpollInstance {
    // TODO: parse flags
    pub fn new(_flags: usize) -> Self {
        Self {
            events: Mutex::new(BTreeMap::new()),
        }
    }

    fn from_fd(fd: c_int) -> LinuxResult<Arc<Self>> {
        get_file_like(fd)?
            .into_any()
            .downcast::<EpollInstance>()
            .map_err(|_| LinuxError::EINVAL)
    }

    fn control(
        &self,
        op: usize,
        fd: usize,
        event: Option<&ctypes::epoll_event>,
    ) -> LinuxResult<usize> {
        match get_file_like(fd as c_int) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        match op as u32 {
            ctypes::EPOLL_CTL_ADD => {
                let event = event.ok_or(LinuxError::EFAULT)?;
                if let Entry::Vacant(e) = self.events.lock().entry(fd) {
                    e.insert(*event);
                } else {
                    return Err(LinuxError::EEXIST);
                }
            }
            ctypes::EPOLL_CTL_MOD => {
                let event = event.ok_or(LinuxError::EFAULT)?;
                let mut events = self.events.lock();
                if let Entry::Occupied(mut ocp) = events.entry(fd) {
                    ocp.insert(*event);
                } else {
                    return Err(LinuxError::ENOENT);
                }
            }
            ctypes::EPOLL_CTL_DEL => {
                let mut events = self.events.lock();
                if let Entry::Occupied(ocp) = events.entry(fd) {
                    ocp.remove_entry();
                } else {
                    return Err(LinuxError::ENOENT);
                }
            }
            _ => {
                return Err(LinuxError::EINVAL);
            }
        }
        Ok(0)
    }

    fn poll_all(&self, events: &mut [ctypes::epoll_event]) -> LinuxResult<usize> {
        let ready_list = self.events.lock();
        let mut events_num = 0;

        for (infd, ev) in ready_list.iter() {
            match get_file_like(*infd as c_int)?.poll() {
                Err(_) => {
                    if (ev.events & ctypes::EPOLLERR) != 0 {
                        if !push_ready_event(events, &mut events_num, ctypes::EPOLLERR, ev.data) {
                            return Ok(events_num);
                        }
                    }
                }
                Ok(state) => {
                    if state.readable && (ev.events & ctypes::EPOLLIN != 0) {
                        if !push_ready_event(events, &mut events_num, ctypes::EPOLLIN, ev.data) {
                            return Ok(events_num);
                        }
                    }

                    if state.writable && (ev.events & ctypes::EPOLLOUT != 0) {
                        if !push_ready_event(events, &mut events_num, ctypes::EPOLLOUT, ev.data) {
                            return Ok(events_num);
                        }
                    }
                }
            }
        }
        Ok(events_num)
    }
}

impl FileLike for EpollInstance {
    fn read(&self, _buf: &mut [u8]) -> LinuxResult<usize> {
        Err(LinuxError::ENOSYS)
    }

    fn write(&self, _buf: &[u8]) -> LinuxResult<usize> {
        Err(LinuxError::ENOSYS)
    }

    fn stat(&self) -> LinuxResult<ctypes::stat> {
        let st_mode = 0o600u32; // rw-------
        Ok(ctypes::stat {
            st_ino: 1,
            st_nlink: 1,
            st_mode,
            ..Default::default()
        })
    }

    fn into_any(self: Arc<Self>) -> alloc::sync::Arc<dyn core::any::Any + Send + Sync> {
        self
    }

    fn poll(&self) -> LinuxResult<axio::PollState> {
        Err(LinuxError::ENOSYS)
    }

    fn set_nonblocking(&self, _nonblocking: bool) -> LinuxResult {
        Ok(())
    }
}

fn push_ready_event(
    events: &mut [ctypes::epoll_event],
    events_num: &mut usize,
    event_mask: u32,
    data: ctypes::epoll_data_t,
) -> bool {
    if *events_num >= events.len() {
        return false;
    }
    events[*events_num].events = event_mask;
    events[*events_num].data = data;
    *events_num += 1;
    true
}

/// Creates a new epoll instance.
///
/// It returns a file descriptor referring to the new epoll instance.
pub fn sys_epoll_create(size: c_int) -> c_int {
    debug!("sys_epoll_create <= {}", size);
    syscall_body!(sys_epoll_create, {
        if size < 0 {
            return Err(LinuxError::EINVAL);
        }
        let epoll_instance = EpollInstance::new(0);
        add_file_like(Arc::new(epoll_instance))
    })
}

/// Control interface for an epoll file descriptor
///
/// # Safety
///
/// For `EPOLL_CTL_ADD` and `EPOLL_CTL_MOD`, `event` must be valid for reads of
/// one `epoll_event`. `EPOLL_CTL_DEL` ignores `event` and may receive null.
pub unsafe fn sys_epoll_ctl(
    epfd: c_int,
    op: c_int,
    fd: c_int,
    event: *mut ctypes::epoll_event,
) -> c_int {
    debug!("sys_epoll_ctl <= epfd: {} op: {} fd: {}", epfd, op, fd);
    syscall_body!(sys_epoll_ctl, {
        let event = match op as u32 {
            ctypes::EPOLL_CTL_ADD | ctypes::EPOLL_CTL_MOD => {
                if event.is_null() {
                    return Err(LinuxError::EFAULT);
                }
                Some(unsafe { &*event })
            }
            _ => None,
        };
        let ret = EpollInstance::from_fd(epfd)?.control(op as usize, fd as usize, event)? as c_int;
        Ok(ret)
    })
}

/// Waits for events on the epoll instance referred to by the file descriptor epfd.
///
/// # Safety
///
/// `events` must be valid for writes of `maxevents` `epoll_event` entries.
pub unsafe fn sys_epoll_wait(
    epfd: c_int,
    events: *mut ctypes::epoll_event,
    maxevents: c_int,
    timeout: c_int,
) -> c_int {
    debug!(
        "sys_epoll_wait <= epfd: {}, maxevents: {}, timeout: {}",
        epfd, maxevents, timeout
    );

    syscall_body!(sys_epoll_wait, {
        if maxevents <= 0 {
            return Err(LinuxError::EINVAL);
        }
        if events.is_null() {
            return Err(LinuxError::EFAULT);
        }
        let maxevents = maxevents as usize;
        let mut ready_events = Vec::new();
        ready_events
            .try_reserve_exact(maxevents)
            .map_err(|_| LinuxError::ENOMEM)?;
        ready_events.resize(maxevents, ctypes::epoll_event::default());
        let deadline =
            (!timeout.is_negative()).then(|| wall_time() + Duration::from_millis(timeout as u64));
        let epoll_instance = EpollInstance::from_fd(epfd)?;
        loop {
            #[cfg(feature = "net")]
            axnet::poll_interfaces();
            let events_num = epoll_instance.poll_all(&mut ready_events)?;
            if events_num > 0 {
                unsafe {
                    core::ptr::copy_nonoverlapping(ready_events.as_ptr(), events, events_num);
                }
                return Ok(events_num as c_int);
            }

            if deadline.is_some_and(|ddl| wall_time() >= ddl) {
                debug!("    timeout!");
                return Ok(0);
            }
            crate::sys_sched_yield();
        }
    })
}
