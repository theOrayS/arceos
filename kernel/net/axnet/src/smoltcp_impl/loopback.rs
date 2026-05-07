use alloc::{collections::VecDeque, sync::Arc};
use core::cmp::min;

use axerrno::{AxError, AxResult, ax_err};
use axio::PollState;
use axsync::Mutex;
use smoltcp::wire::IpEndpoint;

const LOOPBACK_BUFFER_LIMIT: usize = 1024 * 1024;

struct LoopbackTcpState {
    client_to_server: VecDeque<u8>,
    server_to_client: VecDeque<u8>,
    client_send_open: bool,
    server_send_open: bool,
    client_recv_open: bool,
    server_recv_open: bool,
}

#[derive(Clone)]
pub(crate) struct LoopbackTcpEndpoint {
    state: Arc<Mutex<LoopbackTcpState>>,
    server_side: bool,
    local_addr: IpEndpoint,
    peer_addr: IpEndpoint,
}

impl LoopbackTcpEndpoint {
    pub(crate) fn pair(
        client_addr: IpEndpoint,
        server_addr: IpEndpoint,
    ) -> (LoopbackTcpEndpoint, LoopbackTcpEndpoint) {
        let state = Arc::new(Mutex::new(LoopbackTcpState {
            client_to_server: VecDeque::new(),
            server_to_client: VecDeque::new(),
            client_send_open: true,
            server_send_open: true,
            client_recv_open: true,
            server_recv_open: true,
        }));
        (
            LoopbackTcpEndpoint {
                state: state.clone(),
                server_side: false,
                local_addr: client_addr,
                peer_addr: server_addr,
            },
            LoopbackTcpEndpoint {
                state,
                server_side: true,
                local_addr: server_addr,
                peer_addr: client_addr,
            },
        )
    }

    pub(crate) fn local_addr(&self) -> IpEndpoint {
        self.local_addr
    }

    pub(crate) fn peer_addr(&self) -> IpEndpoint {
        self.peer_addr
    }

    pub(crate) fn send(&self, buf: &[u8]) -> AxResult<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let mut state = self.state.lock();
        let local_send_open = if self.server_side {
            state.server_send_open
        } else {
            state.client_send_open
        };
        let peer_recv_open = if self.server_side {
            state.client_recv_open
        } else {
            state.server_recv_open
        };
        if !local_send_open || !peer_recv_open {
            return ax_err!(BrokenPipe, "loopback TCP send() failed");
        }

        let tx_queue = if self.server_side {
            &mut state.server_to_client
        } else {
            &mut state.client_to_server
        };
        if tx_queue.len() >= LOOPBACK_BUFFER_LIMIT {
            return Err(AxError::WouldBlock);
        }

        let len = min(buf.len(), LOOPBACK_BUFFER_LIMIT - tx_queue.len());
        tx_queue.extend(buf[..len].iter().copied());
        Ok(len)
    }

    pub(crate) fn recv(&self, buf: &mut [u8]) -> AxResult<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let mut state = self.state.lock();
        let local_recv_open = if self.server_side {
            state.server_recv_open
        } else {
            state.client_recv_open
        };
        if !local_recv_open {
            return Ok(0);
        }
        let peer_send_open = if self.server_side {
            state.client_send_open
        } else {
            state.server_send_open
        };
        let rx_queue = if self.server_side {
            &mut state.client_to_server
        } else {
            &mut state.server_to_client
        };

        if rx_queue.is_empty() {
            return if peer_send_open {
                Err(AxError::WouldBlock)
            } else {
                Ok(0)
            };
        }

        let len = min(buf.len(), rx_queue.len());
        for slot in buf.iter_mut().take(len) {
            *slot = rx_queue.pop_front().unwrap();
        }
        Ok(len)
    }

    pub(crate) fn poll(&self) -> PollState {
        let state = self.state.lock();
        let (local_recv_open, peer_send_open, peer_recv_open, rx_len, tx_len) = if self.server_side
        {
            (
                state.server_recv_open,
                state.client_send_open,
                state.client_recv_open,
                state.client_to_server.len(),
                state.server_to_client.len(),
            )
        } else {
            (
                state.client_recv_open,
                state.server_send_open,
                state.server_recv_open,
                state.server_to_client.len(),
                state.client_to_server.len(),
            )
        };
        PollState {
            readable: rx_len > 0 || !local_recv_open || !peer_send_open,
            writable: peer_recv_open && tx_len < LOOPBACK_BUFFER_LIMIT,
        }
    }

    pub(crate) fn shutdown_read(&self) {
        let mut state = self.state.lock();
        if self.server_side {
            state.server_recv_open = false;
        } else {
            state.client_recv_open = false;
        }
    }

    pub(crate) fn shutdown_write(&self) {
        let mut state = self.state.lock();
        if self.server_side {
            state.server_send_open = false;
        } else {
            state.client_send_open = false;
        }
    }

    pub(crate) fn shutdown(&self) {
        let mut state = self.state.lock();
        if self.server_side {
            state.server_send_open = false;
            state.server_recv_open = false;
        } else {
            state.client_send_open = false;
            state.client_recv_open = false;
        }
    }
}
