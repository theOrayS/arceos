use core::time::Duration;

use axerrno::LinuxError;
use axhal::time::wall_time;
use axio::PollState;
use linux_raw_sys::general;

const POLL_READABLE: i16 = general::POLLIN as i16;
const POLL_WRITABLE: i16 = general::POLLOUT as i16;
const POLL_ERROR: i16 = general::POLLERR as i16;
const POLL_HANGUP: i16 = general::POLLHUP as i16;
const POLL_INVALID: i16 = general::POLLNVAL as i16;

#[derive(Clone, Copy, Debug)]
pub(crate) struct PollEvent {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

pub(crate) fn poll_events<F>(
    events: &mut [PollEvent],
    timeout: Option<Duration>,
    mut poll_state: F,
    mut interrupted: impl FnMut() -> bool,
) -> Result<usize, LinuxError>
where
    F: FnMut(i32) -> Result<PollState, LinuxError>,
{
    let deadline = timeout.map(|timeout| wall_time() + timeout);
    loop {
        if interrupted() {
            return Err(LinuxError::EINTR);
        }

        #[cfg(feature = "net")]
        axnet::poll_interfaces();

        let mut ready = 0usize;
        for event in events.iter_mut() {
            event.revents = 0;
            if event.fd < 0 {
                continue;
            }
            match poll_state(event.fd) {
                Ok(state) => {
                    if state.readable && event.events & POLL_READABLE != 0 {
                        event.revents |= POLL_READABLE;
                    }
                    if state.writable && event.events & POLL_WRITABLE != 0 {
                        event.revents |= POLL_WRITABLE;
                    }
                }
                Err(LinuxError::EBADF) => event.revents |= POLL_INVALID,
                Err(_) => event.revents |= POLL_ERROR | POLL_HANGUP,
            }
            if event.revents != 0 {
                ready += 1;
            }
        }

        if ready > 0 {
            return Ok(ready);
        }
        if deadline.is_some_and(|deadline| wall_time() >= deadline) {
            return Ok(0);
        }
        if interrupted() {
            return Err(LinuxError::EINTR);
        }
        crate::imp::task::sys_sched_yield();
    }
}
