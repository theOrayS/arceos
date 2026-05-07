use core::net::SocketAddr;
use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;

use axerrno::{AxError, AxResult, ax_err, ax_err_type};
use axio::PollState;
use axsync::Mutex;
use spin::RwLock;

use smoltcp::iface::SocketHandle;
use smoltcp::socket::udp::{self, BindError, SendError};
use smoltcp::wire::{IpEndpoint, IpListenEndpoint};

use super::addr::UNSPECIFIED_ENDPOINT;
use super::udp_loopback::{
    UdpLoopbackQueue, is_loopback_endpoint, register_udp_loopback, send_udp_loopback,
    unregister_udp_loopback,
};
use super::{SOCKET_SET, SocketSetWrapper};

/// A UDP socket that provides POSIX-like APIs.
pub struct UdpSocket {
    handle: SocketHandle,
    local_addr: RwLock<Option<IpEndpoint>>,
    peer_addr: RwLock<Option<IpEndpoint>>,
    loopback_queue: UdpLoopbackQueue,
    nonblock: AtomicBool,
    recv_timeout: Mutex<Option<Duration>>,
    send_timeout: Mutex<Option<Duration>>,
}

impl UdpSocket {
    /// Creates a new UDP socket.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let socket = SocketSetWrapper::new_udp_socket();
        let handle = SOCKET_SET.add(socket);
        Self {
            handle,
            local_addr: RwLock::new(None),
            peer_addr: RwLock::new(None),
            loopback_queue: UdpLoopbackQueue::new(),
            nonblock: AtomicBool::new(false),
            recv_timeout: Mutex::new(None),
            send_timeout: Mutex::new(None),
        }
    }

    /// Returns the local address and port, or
    /// [`Err(NotConnected)`](AxError::NotConnected) if not connected.
    pub fn local_addr(&self) -> AxResult<SocketAddr> {
        match self.local_addr.try_read() {
            Some(addr) => addr.map(Into::into).ok_or(AxError::NotConnected),
            None => Err(AxError::NotConnected),
        }
    }

    /// Returns the remote address and port, or
    /// [`Err(NotConnected)`](AxError::NotConnected) if not connected.
    pub fn peer_addr(&self) -> AxResult<SocketAddr> {
        self.remote_endpoint().map(Into::into)
    }

    /// Returns whether this socket is in nonblocking mode.
    #[inline]
    pub fn is_nonblocking(&self) -> bool {
        self.nonblock.load(Ordering::Acquire)
    }

    /// Moves this UDP socket into or out of nonblocking mode.
    ///
    /// This will result in `recv`, `recv_from`, `send`, and `send_to`
    /// operations becoming nonblocking, i.e., immediately returning from their
    /// calls. If the IO operation is successful, `Ok` is returned and no
    /// further action is required. If the IO operation could not be completed
    /// and needs to be retried, an error with kind
    /// [`Err(WouldBlock)`](AxError::WouldBlock) is returned.
    #[inline]
    pub fn set_nonblocking(&self, nonblocking: bool) {
        self.nonblock.store(nonblocking, Ordering::Release);
    }

    /// Sets the timeout used by blocking receive operations.
    #[inline]
    pub fn set_recv_timeout(&self, timeout: Option<Duration>) {
        *self.recv_timeout.lock() = timeout;
    }

    /// Returns the timeout used by blocking receive operations.
    #[inline]
    pub fn recv_timeout(&self) -> Option<Duration> {
        *self.recv_timeout.lock()
    }

    /// Sets the timeout used by blocking send operations.
    #[inline]
    pub fn set_send_timeout(&self, timeout: Option<Duration>) {
        *self.send_timeout.lock() = timeout;
    }

    /// Returns the timeout used by blocking send operations.
    #[inline]
    pub fn send_timeout(&self) -> Option<Duration> {
        *self.send_timeout.lock()
    }

    /// Binds an unbound socket to the given address and port.
    ///
    /// It's must be called before [`send_to`](Self::send_to) and
    /// [`recv_from`](Self::recv_from).
    pub fn bind(&self, mut local_addr: SocketAddr) -> AxResult {
        let mut self_local_addr = self.local_addr.write();

        if local_addr.port() == 0 {
            local_addr.set_port(get_ephemeral_port()?);
        }
        if self_local_addr.is_some() {
            return ax_err!(InvalidInput, "socket bind() failed: already bound");
        }

        let local_endpoint = IpEndpoint::from(local_addr);
        let endpoint = IpListenEndpoint {
            addr: (!local_endpoint.addr.is_unspecified()).then_some(local_endpoint.addr),
            port: local_endpoint.port,
        };
        SOCKET_SET.with_socket_mut::<udp::Socket, _, _>(self.handle, |socket| {
            socket.bind(endpoint).or_else(|e| match e {
                BindError::InvalidState => ax_err!(AlreadyExists, "socket bind() failed"),
                BindError::Unaddressable => ax_err!(InvalidInput, "socket bind() failed"),
            })
        })?;

        *self_local_addr = Some(local_endpoint);
        register_udp_loopback(local_endpoint, self.loopback_queue.clone());
        debug!("UDP socket {}: bound on {}", self.handle, endpoint);
        Ok(())
    }

    /// Sends data on the socket to the given address. On success, returns the
    /// number of bytes written.
    pub fn send_to(&self, buf: &[u8], remote_addr: SocketAddr) -> AxResult<usize> {
        if remote_addr.port() == 0 || remote_addr.ip().is_unspecified() {
            return ax_err!(InvalidInput, "socket send_to() failed: invalid address");
        }
        let remote_endpoint = IpEndpoint::from(remote_addr);
        if is_loopback_endpoint(remote_endpoint) {
            return self.send_loopback(buf, remote_endpoint);
        }
        self.send_impl(buf, remote_endpoint)
    }

    /// Receives a single datagram message on the socket. On success, returns
    /// the number of bytes read and the origin.
    pub fn recv_from(&self, buf: &mut [u8]) -> AxResult<(usize, SocketAddr)> {
        if self.local_addr.read().is_none() {
            return ax_err!(NotConnected, "socket recv_from() failed");
        }
        self.block_on_timeout(self.recv_timeout(), || {
            if let Some(datagram) = self.try_recv_loopback_from(buf, None) {
                return Ok(datagram);
            }
            SOCKET_SET.with_socket_mut::<udp::Socket, _, _>(self.handle, |socket| {
                if socket.can_recv() {
                    match socket.recv_slice(buf) {
                        Ok((len, meta)) => Ok((len, SocketAddr::from(meta.endpoint))),
                        Err(_) => ax_err!(BadState, "socket recv_from() failed"),
                    }
                } else {
                    Err(AxError::WouldBlock)
                }
            })
        })
    }

    /// Receives a single datagram message on the socket, without removing it from
    /// the queue. On success, returns the number of bytes read and the origin.
    pub fn peek_from(&self, buf: &mut [u8]) -> AxResult<(usize, SocketAddr)> {
        self.recv_impl(|socket| match socket.peek_slice(buf) {
            Ok((len, meta)) => Ok((len, SocketAddr::from(meta.endpoint))),
            Err(_) => ax_err!(BadState, "socket recv_from() failed"),
        })
    }

    /// Connects this UDP socket to a remote address, allowing the `send` and
    /// `recv` to be used to send data and also applies filters to only receive
    /// data from the specified address.
    ///
    /// The local port will be generated automatically if the socket is not bound.
    /// It's must be called before [`send`](Self::send) and
    /// [`recv`](Self::recv).
    pub fn connect(&self, addr: SocketAddr) -> AxResult {
        let mut self_peer_addr = self.peer_addr.write();

        if self.local_addr.read().is_none() {
            self.bind(SocketAddr::from(UNSPECIFIED_ENDPOINT))?;
        }

        *self_peer_addr = Some(IpEndpoint::from(addr));
        debug!("UDP socket {}: connected to {}", self.handle, addr);
        Ok(())
    }

    /// Sends data on the socket to the remote address to which it is connected.
    pub fn send(&self, buf: &[u8]) -> AxResult<usize> {
        let remote_endpoint = self.remote_endpoint()?;
        if is_loopback_endpoint(remote_endpoint) {
            return self.send_loopback(buf, remote_endpoint);
        }
        self.send_impl(buf, remote_endpoint)
    }

    /// Receives a single datagram message on the socket from the remote address
    /// to which it is connected. On success, returns the number of bytes read.
    pub fn recv(&self, buf: &mut [u8]) -> AxResult<usize> {
        let remote_endpoint = self.remote_endpoint()?;
        self.block_on_timeout(self.recv_timeout(), || {
            if let Some((len, _)) = self.try_recv_loopback_from(buf, Some(remote_endpoint)) {
                return Ok(len);
            }
            SOCKET_SET.with_socket_mut::<udp::Socket, _, _>(self.handle, |socket| {
                if socket.can_recv() {
                    let (len, meta) = socket
                        .recv_slice(buf)
                        .map_err(|_| ax_err_type!(BadState, "socket recv() failed"))?;
                    if !remote_endpoint.addr.is_unspecified()
                        && remote_endpoint.addr != meta.endpoint.addr
                    {
                        return Err(AxError::WouldBlock);
                    }
                    if remote_endpoint.port != 0 && remote_endpoint.port != meta.endpoint.port {
                        return Err(AxError::WouldBlock);
                    }
                    Ok(len)
                } else {
                    Err(AxError::WouldBlock)
                }
            })
        })
    }

    /// Close the socket.
    pub fn shutdown(&self) -> AxResult {
        if let Some(local) = self.local_addr.write().take() {
            unregister_udp_loopback(local, &self.loopback_queue);
        }
        SOCKET_SET.with_socket_mut::<udp::Socket, _, _>(self.handle, |socket| {
            debug!("UDP socket {}: shutting down", self.handle);
            socket.close();
        });
        SOCKET_SET.poll_interfaces();
        Ok(())
    }

    /// Whether the socket is readable or writable.
    pub fn poll(&self) -> AxResult<PollState> {
        if self.local_addr.read().is_none() {
            return Ok(PollState {
                readable: false,
                writable: false,
            });
        }
        SOCKET_SET.with_socket_mut::<udp::Socket, _, _>(self.handle, |socket| {
            Ok(PollState {
                readable: self.loopback_queue.has_packet() || socket.can_recv(),
                writable: socket.can_send(),
            })
        })
    }
}

/// Private methods
impl UdpSocket {
    fn remote_endpoint(&self) -> AxResult<IpEndpoint> {
        match self.peer_addr.try_read() {
            Some(addr) => addr.ok_or(AxError::NotConnected),
            None => Err(AxError::NotConnected),
        }
    }

    fn send_impl(&self, buf: &[u8], remote_endpoint: IpEndpoint) -> AxResult<usize> {
        if self.local_addr.read().is_none() {
            return ax_err!(NotConnected, "socket send() failed");
        }

        self.block_on_timeout(self.send_timeout(), || {
            SOCKET_SET.with_socket_mut::<udp::Socket, _, _>(self.handle, |socket| {
                if socket.can_send() {
                    socket
                        .send_slice(buf, remote_endpoint)
                        .map_err(|e| match e {
                            SendError::BufferFull => AxError::WouldBlock,
                            SendError::Unaddressable => {
                                ax_err_type!(ConnectionRefused, "socket send() failed")
                            }
                        })?;
                    Ok(buf.len())
                } else {
                    // tx buffer is full
                    Err(AxError::WouldBlock)
                }
            })
        })
    }

    fn send_loopback(&self, buf: &[u8], remote_endpoint: IpEndpoint) -> AxResult<usize> {
        let local_endpoint = match self.local_addr.read().as_ref() {
            Some(local) => *local,
            None => return ax_err!(NotConnected, "socket send() failed"),
        };
        let len = send_udp_loopback(local_endpoint, remote_endpoint, buf);
        axtask::yield_now();
        Ok(len)
    }

    fn recv_loopback(&self, buf: &mut [u8], remote: Option<IpEndpoint>) -> AxResult<usize> {
        self.recv_loopback_from(buf, remote).map(|(len, _)| len)
    }

    fn recv_loopback_from(
        &self,
        buf: &mut [u8],
        remote: Option<IpEndpoint>,
    ) -> AxResult<(usize, SocketAddr)> {
        self.block_on_timeout(self.recv_timeout(), || {
            self.try_recv_loopback_from(buf, remote)
                .ok_or(AxError::WouldBlock)
        })
    }

    fn try_recv_loopback_from(
        &self,
        buf: &mut [u8],
        remote: Option<IpEndpoint>,
    ) -> Option<(usize, SocketAddr)> {
        let packet = self.loopback_queue.pop_matching(remote)?;
        let len = packet.data.len().min(buf.len());
        buf[..len].copy_from_slice(&packet.data[..len]);
        Some((len, SocketAddr::from(packet.peer)))
    }

    fn recv_impl<F, T>(&self, mut op: F) -> AxResult<T>
    where
        F: FnMut(&mut udp::Socket) -> AxResult<T>,
    {
        if self.local_addr.read().is_none() {
            return ax_err!(NotConnected, "socket send() failed");
        }

        self.block_on_timeout(self.recv_timeout(), || {
            SOCKET_SET.with_socket_mut::<udp::Socket, _, _>(self.handle, |socket| {
                if socket.can_recv() {
                    // data available
                    op(socket)
                } else {
                    // no more data
                    Err(AxError::WouldBlock)
                }
            })
        })
    }

    fn block_on_timeout<F, T>(&self, timeout: Option<Duration>, mut f: F) -> AxResult<T>
    where
        F: FnMut() -> AxResult<T>,
    {
        if self.is_nonblocking() {
            f()
        } else {
            let deadline = timeout.map(|dur| axhal::time::wall_time() + dur);
            loop {
                SOCKET_SET.poll_interfaces();
                match f() {
                    Ok(t) => return Ok(t),
                    Err(AxError::WouldBlock) => {
                        if deadline.is_some_and(|ddl| axhal::time::wall_time() >= ddl) {
                            return Err(AxError::WouldBlock);
                        }
                        axtask::yield_now();
                    }
                    Err(e) => return Err(e),
                }
            }
        }
    }
}

impl Drop for UdpSocket {
    fn drop(&mut self) {
        self.shutdown().ok();
        SOCKET_SET.remove(self.handle);
    }
}

fn get_ephemeral_port() -> AxResult<u16> {
    const PORT_START: u16 = 0xc000;
    const PORT_END: u16 = 0xffff;
    static CURR: Mutex<u16> = Mutex::new(PORT_START);
    let mut curr = CURR.lock();

    let port = *curr;
    if *curr == PORT_END {
        *curr = PORT_START;
    } else {
        *curr += 1;
    }
    Ok(port)
}
