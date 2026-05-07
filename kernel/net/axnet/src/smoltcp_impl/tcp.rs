use core::cell::UnsafeCell;
use core::net::SocketAddr;
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use core::time::Duration;

use axerrno::{AxError, AxResult, ax_err, ax_err_type};
use axio::PollState;
use axsync::Mutex;

use smoltcp::iface::SocketHandle;
use smoltcp::socket::tcp::{self, ConnectError, State};
use smoltcp::wire::{IpEndpoint, IpListenEndpoint};

use super::addr::UNSPECIFIED_ENDPOINT;
use super::loopback::LoopbackTcpEndpoint;
use super::{ETH0, LISTEN_TABLE, SOCKET_SET, SocketSetWrapper};

// State transitions:
// CLOSED -(connect)-> BUSY -> CONNECTING -> CONNECTED -(shutdown)-> BUSY -> CLOSED
//       |
//       |-(listen)-> BUSY -> LISTENING -(shutdown)-> BUSY -> CLOSED
//       |
//        -(bind)-> BUSY -> CLOSED
const STATE_CLOSED: u8 = 0;
const STATE_BUSY: u8 = 1;
const STATE_CONNECTING: u8 = 2;
const STATE_CONNECTED: u8 = 3;
const STATE_LISTENING: u8 = 4;

/// A TCP socket that provides POSIX-like APIs.
///
/// - [`connect`] is for TCP clients.
/// - [`bind`], [`listen`], and [`accept`] are for TCP servers.
/// - Other methods are for both TCP clients and servers.
///
/// [`connect`]: TcpSocket::connect
/// [`bind`]: TcpSocket::bind
/// [`listen`]: TcpSocket::listen
/// [`accept`]: TcpSocket::accept
pub struct TcpSocket {
    state: AtomicU8,
    handle: UnsafeCell<Option<SocketHandle>>,
    local_addr: UnsafeCell<IpEndpoint>,
    peer_addr: UnsafeCell<IpEndpoint>,
    loopback: UnsafeCell<Option<LoopbackTcpEndpoint>>,
    nonblock: AtomicBool,
    recv_shutdown: AtomicBool,
    send_shutdown: AtomicBool,
    recv_timeout: Mutex<Option<Duration>>,
    send_timeout: Mutex<Option<Duration>>,
}

unsafe impl Sync for TcpSocket {}

impl TcpSocket {
    /// Creates a new TCP socket.
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(STATE_CLOSED),
            handle: UnsafeCell::new(None),
            local_addr: UnsafeCell::new(UNSPECIFIED_ENDPOINT),
            peer_addr: UnsafeCell::new(UNSPECIFIED_ENDPOINT),
            loopback: UnsafeCell::new(None),
            nonblock: AtomicBool::new(false),
            recv_shutdown: AtomicBool::new(false),
            send_shutdown: AtomicBool::new(false),
            recv_timeout: Mutex::new(None),
            send_timeout: Mutex::new(None),
        }
    }

    /// Creates a new TCP socket that is already connected.
    const fn new_connected(
        handle: SocketHandle,
        local_addr: IpEndpoint,
        peer_addr: IpEndpoint,
    ) -> Self {
        Self {
            state: AtomicU8::new(STATE_CONNECTED),
            handle: UnsafeCell::new(Some(handle)),
            local_addr: UnsafeCell::new(local_addr),
            peer_addr: UnsafeCell::new(peer_addr),
            loopback: UnsafeCell::new(None),
            nonblock: AtomicBool::new(false),
            recv_shutdown: AtomicBool::new(false),
            send_shutdown: AtomicBool::new(false),
            recv_timeout: Mutex::new(None),
            send_timeout: Mutex::new(None),
        }
    }

    fn new_loopback_connected(endpoint: LoopbackTcpEndpoint) -> Self {
        Self {
            state: AtomicU8::new(STATE_CONNECTED),
            handle: UnsafeCell::new(None),
            local_addr: UnsafeCell::new(endpoint.local_addr()),
            peer_addr: UnsafeCell::new(endpoint.peer_addr()),
            loopback: UnsafeCell::new(Some(endpoint)),
            nonblock: AtomicBool::new(false),
            recv_shutdown: AtomicBool::new(false),
            send_shutdown: AtomicBool::new(false),
            recv_timeout: Mutex::new(None),
            send_timeout: Mutex::new(None),
        }
    }

    /// Returns the bound local address and port, or
    /// [`Err(NotConnected)`](AxError::NotConnected) if not bound or connected.
    pub fn local_addr(&self) -> AxResult<SocketAddr> {
        let local_addr = unsafe { self.local_addr.get().read() };
        match (self.get_state(), local_addr) {
            (STATE_CONNECTED | STATE_LISTENING, _) => Ok(SocketAddr::from(local_addr)),
            (_, endpoint) if endpoint != UNSPECIFIED_ENDPOINT => Ok(SocketAddr::from(endpoint)),
            _ => Err(AxError::NotConnected),
        }
    }

    /// Returns the remote address and port, or
    /// [`Err(NotConnected)`](AxError::NotConnected) if not connected.
    pub fn peer_addr(&self) -> AxResult<SocketAddr> {
        match self.get_state() {
            STATE_CONNECTED | STATE_LISTENING => {
                Ok(SocketAddr::from(unsafe { self.peer_addr.get().read() }))
            }
            _ => Err(AxError::NotConnected),
        }
    }

    /// Returns whether this socket is in nonblocking mode.
    #[inline]
    pub fn is_nonblocking(&self) -> bool {
        self.nonblock.load(Ordering::Acquire)
    }

    /// Moves this TCP stream into or out of nonblocking mode.
    ///
    /// This will result in `read`, `write`, `recv` and `send` operations
    /// becoming nonblocking, i.e., immediately returning from their calls.
    /// If the IO operation is successful, `Ok` is returned and no further
    /// action is required. If the IO operation could not be completed and needs
    /// to be retried, an error with kind  [`Err(WouldBlock)`](AxError::WouldBlock) is
    /// returned.
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

    /// Connects to the given address and port.
    ///
    /// The local port is generated automatically.
    pub fn connect(&self, remote_addr: SocketAddr) -> AxResult {
        if remote_addr.ip().is_loopback() {
            return self.connect_loopback(remote_addr);
        }

        self.update_state(STATE_CLOSED, STATE_CONNECTING, || {
            // SAFETY: no other threads can read or write these fields.
            let handle = unsafe { self.handle.get().read() }
                .unwrap_or_else(|| SOCKET_SET.add(SocketSetWrapper::new_tcp_socket()));

            // TODO: check remote addr unreachable
            let bound_endpoint = self.bound_endpoint()?;
            let iface = &ETH0.iface;
            let (local_endpoint, remote_endpoint) = SOCKET_SET
                .with_socket_mut::<tcp::Socket, _, _>(handle, |socket| {
                    socket
                        .connect(iface.lock().context(), remote_addr, bound_endpoint)
                        .or_else(|e| match e {
                            ConnectError::InvalidState => {
                                ax_err!(BadState, "socket connect() failed")
                            }
                            ConnectError::Unaddressable => {
                                ax_err!(ConnectionRefused, "socket connect() failed")
                            }
                        })?;
                    Ok((
                        socket.local_endpoint().unwrap(),
                        socket.remote_endpoint().unwrap(),
                    ))
                })?;
            unsafe {
                // SAFETY: no other threads can read or write these fields as we
                // have changed the state to `BUSY`.
                self.local_addr.get().write(local_endpoint);
                self.peer_addr.get().write(remote_endpoint);
                self.handle.get().write(Some(handle));
            }
            Ok(())
        })
        .unwrap_or_else(|_| ax_err!(AlreadyExists, "socket connect() failed: already connected"))?; // EISCONN

        // Here our state must be `CONNECTING`, and only one thread can run here.
        if self.is_nonblocking() {
            Err(AxError::WouldBlock)
        } else {
            self.block_on_timeout(None, || {
                let PollState { writable, .. } = self.poll_connect()?;
                if !writable {
                    Err(AxError::WouldBlock)
                } else if self.get_state() == STATE_CONNECTED {
                    Ok(())
                } else {
                    ax_err!(ConnectionRefused, "socket connect() failed")
                }
            })
        }
    }

    /// Binds an unbound socket to the given address and port.
    ///
    /// If the given port is 0, it generates one automatically.
    ///
    /// It's must be called before [`listen`](Self::listen) and
    /// [`accept`](Self::accept).
    pub fn bind(&self, mut local_addr: SocketAddr) -> AxResult {
        self.update_state(STATE_CLOSED, STATE_CLOSED, || {
            // TODO: check addr is available
            if local_addr.port() == 0 {
                local_addr.set_port(get_ephemeral_port()?);
            }
            // SAFETY: no other threads can read or write `self.local_addr` as we
            // have changed the state to `BUSY`.
            unsafe {
                let old = self.local_addr.get().read();
                if old != UNSPECIFIED_ENDPOINT {
                    return ax_err!(InvalidInput, "socket bind() failed: already bound");
                }
                self.local_addr.get().write(IpEndpoint::from(local_addr));
            }
            Ok(())
        })
        .unwrap_or_else(|_| ax_err!(InvalidInput, "socket bind() failed: already bound"))
    }

    /// Starts listening on the bound address and port.
    ///
    /// It's must be called after [`bind`](Self::bind) and before
    /// [`accept`](Self::accept).
    pub fn listen(&self) -> AxResult {
        self.update_state(STATE_CLOSED, STATE_LISTENING, || {
            let bound_endpoint = self.bound_endpoint()?;
            unsafe {
                (*self.local_addr.get()).port = bound_endpoint.port;
            }
            LISTEN_TABLE.listen(bound_endpoint)?;
            debug!("TCP socket listening on {}", bound_endpoint);
            Ok(())
        })
        .unwrap_or(Ok(())) // ignore simultaneous `listen`s.
    }

    /// Accepts a new connection.
    ///
    /// This function will block the calling thread until a new TCP connection
    /// is established. When established, a new [`TcpSocket`] is returned.
    ///
    /// It's must be called after [`bind`](Self::bind) and [`listen`](Self::listen).
    pub fn accept(&self) -> AxResult<TcpSocket> {
        if !self.is_listening() {
            return ax_err!(InvalidInput, "socket accept() failed: not listen");
        }

        // SAFETY: `self.local_addr` should be initialized after `bind()`.
        let local_port = unsafe { self.local_addr.get().read().port };
        self.block_on_timeout(self.recv_timeout(), || {
            if let Some(endpoint) = LISTEN_TABLE.accept_loopback(local_port)? {
                debug!("TCP loopback socket accepted a new connection");
                return Ok(TcpSocket::new_loopback_connected(endpoint));
            }
            let (handle, (local_addr, peer_addr)) = LISTEN_TABLE.accept(local_port)?;
            debug!("TCP socket accepted a new connection {}", peer_addr);
            Ok(TcpSocket::new_connected(handle, local_addr, peer_addr))
        })
    }

    /// Close the connection.
    pub fn shutdown(&self) -> AxResult {
        // stream
        self.update_state(STATE_CONNECTED, STATE_CLOSED, || {
            self.recv_shutdown.store(true, Ordering::Release);
            self.send_shutdown.store(true, Ordering::Release);
            if let Some(endpoint) = unsafe { (&mut *self.loopback.get()).take() } {
                endpoint.shutdown();
            } else {
                // SAFETY: `self.handle` should be initialized in a connected socket, and
                // no other threads can read or write it.
                let handle = unsafe { self.handle.get().read().unwrap() };
                SOCKET_SET.with_socket_mut::<tcp::Socket, _, _>(handle, |socket| {
                    debug!("TCP socket {}: shutting down", handle);
                    socket.close();
                });
                SOCKET_SET.poll_interfaces();
            }
            unsafe {
                self.local_addr.get().write(UNSPECIFIED_ENDPOINT);
                self.peer_addr.get().write(UNSPECIFIED_ENDPOINT);
            }
            Ok(())
        })
        .unwrap_or(Ok(()))?;

        // listener
        self.update_state(STATE_LISTENING, STATE_CLOSED, || {
            // SAFETY: `self.local_addr` should be initialized in a listening socket,
            // and no other threads can read or write it.
            let local_port = unsafe { self.local_addr.get().read().port };
            unsafe { self.local_addr.get().write(UNSPECIFIED_ENDPOINT) }; // clear bound address
            LISTEN_TABLE.unlisten(local_port);
            SOCKET_SET.poll_interfaces();
            Ok(())
        })
        .unwrap_or(Ok(()))?;

        // ignore for other states
        Ok(())
    }

    /// Shut down the receive half of the connection while keeping the socket open.
    pub fn shutdown_read(&self) -> AxResult {
        if !self.is_connected() {
            return ax_err!(NotConnected, "socket shutdown(SHUT_RD) failed");
        }
        self.recv_shutdown.store(true, Ordering::Release);
        if let Some(endpoint) = self.loopback_endpoint() {
            endpoint.shutdown_read();
        }
        Ok(())
    }

    /// Shut down the send half of the connection while keeping the receive half open.
    pub fn shutdown_write(&self) -> AxResult {
        if !self.is_connected() {
            return ax_err!(NotConnected, "socket shutdown(SHUT_WR) failed");
        }
        if self.send_shutdown.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        if let Some(endpoint) = self.loopback_endpoint() {
            endpoint.shutdown_write();
            return Ok(());
        }

        // SAFETY: `self.handle` should be initialized in a connected socket.
        let handle = unsafe { self.handle.get().read().unwrap() };
        SOCKET_SET.with_socket_mut::<tcp::Socket, _, _>(handle, |socket| {
            debug!("TCP socket {}: shutting down write half", handle);
            socket.close();
        });
        SOCKET_SET.poll_interfaces();
        Ok(())
    }

    /// Receives data from the socket, stores it in the given buffer.
    pub fn recv(&self, buf: &mut [u8]) -> AxResult<usize> {
        if self.is_connecting() {
            return Err(AxError::WouldBlock);
        } else if !self.is_connected() {
            return ax_err!(NotConnected, "socket recv() failed");
        }
        if self.recv_shutdown.load(Ordering::Acquire) {
            return Ok(0);
        }

        if let Some(endpoint) = self.loopback_endpoint() {
            return self.block_on_timeout(self.recv_timeout(), || endpoint.recv(buf));
        }

        // SAFETY: `self.handle` should be initialized in a connected socket.
        let handle = unsafe { self.handle.get().read().unwrap() };
        self.block_on_timeout(self.recv_timeout(), || {
            SOCKET_SET.with_socket_mut::<tcp::Socket, _, _>(handle, |socket| {
                if !socket.is_active() {
                    // not open
                    ax_err!(ConnectionRefused, "socket recv() failed")
                } else if !socket.may_recv() {
                    // connection closed
                    Ok(0)
                } else if socket.recv_queue() > 0 {
                    // data available
                    // TODO: use socket.recv(|buf| {...})
                    let len = socket
                        .recv_slice(buf)
                        .map_err(|_| ax_err_type!(BadState, "socket recv() failed"))?;
                    Ok(len)
                } else {
                    // no more data
                    Err(AxError::WouldBlock)
                }
            })
        })
    }

    /// Transmits data in the given buffer.
    pub fn send(&self, buf: &[u8]) -> AxResult<usize> {
        if self.is_connecting() {
            return Err(AxError::WouldBlock);
        } else if !self.is_connected() {
            return ax_err!(NotConnected, "socket send() failed");
        }
        if self.send_shutdown.load(Ordering::Acquire) {
            return ax_err!(BrokenPipe, "socket send() failed: write half is shut down");
        }

        if let Some(endpoint) = self.loopback_endpoint() {
            return self.block_on_timeout(self.send_timeout(), || endpoint.send(buf));
        }

        // SAFETY: `self.handle` should be initialized in a connected socket.
        let handle = unsafe { self.handle.get().read().unwrap() };
        self.block_on_timeout(self.send_timeout(), || {
            SOCKET_SET.with_socket_mut::<tcp::Socket, _, _>(handle, |socket| {
                if !socket.is_active() || !socket.may_send() {
                    // closed by remote
                    ax_err!(ConnectionReset, "socket send() failed")
                } else if socket.can_send() {
                    // connected, and the tx buffer is not full
                    // TODO: use socket.send(|buf| {...})
                    let len = socket
                        .send_slice(buf)
                        .map_err(|_| ax_err_type!(BadState, "socket send() failed"))?;
                    Ok(len)
                } else {
                    // tx buffer is full
                    Err(AxError::WouldBlock)
                }
            })
        })
    }

    /// Whether the socket is readable or writable.
    pub fn poll(&self) -> AxResult<PollState> {
        match self.get_state() {
            STATE_CONNECTING => self.poll_connect(),
            STATE_CONNECTED => self.poll_stream(),
            STATE_LISTENING => self.poll_listener(),
            _ => Ok(PollState {
                readable: false,
                writable: false,
            }),
        }
    }

    /// Checks if Nagle's algorithm is enabled for this TCP socket.
    pub fn nodelay(&self) -> AxResult<bool> {
        if self.loopback_endpoint().is_some() {
            return Ok(true);
        }
        if let Some(h) = unsafe { self.handle.get().read() } {
            Ok(SOCKET_SET.with_socket::<tcp::Socket, _, _>(h, |socket| socket.nagle_enabled()))
        } else {
            ax_err!(NotConnected, "socket is not connected")
        }
    }

    /// Enables or disables Nagle's algorithm for this TCP socket.
    pub fn set_nodelay(&self, enabled: bool) -> AxResult<()> {
        if self.loopback_endpoint().is_some() {
            return Ok(());
        }
        if let Some(h) = unsafe { self.handle.get().read() } {
            SOCKET_SET.with_socket_mut::<tcp::Socket, _, _>(h, |socket| {
                socket.set_nagle_enabled(enabled);
            });
            Ok(())
        } else {
            ax_err!(NotConnected, "socket is not connected")
        }
    }

    /// Returns the maximum capacity of the receive buffer in bytes.
    pub fn recv_capacity(&self) -> AxResult<usize> {
        if self.loopback_endpoint().is_some() {
            return Ok(64 * 1024);
        }
        if let Some(h) = unsafe { self.handle.get().read() } {
            Ok(SOCKET_SET.with_socket::<tcp::Socket, _, _>(h, |socket| socket.recv_capacity()))
        } else {
            ax_err!(NotConnected, "socket is not connected")
        }
    }

    /// Returns the maximum capacity of the send buffer in bytes.
    pub fn send_capacity(&self) -> AxResult<usize> {
        if self.loopback_endpoint().is_some() {
            return Ok(64 * 1024);
        }
        if let Some(h) = unsafe { self.handle.get().read() } {
            Ok(SOCKET_SET.with_socket::<tcp::Socket, _, _>(h, |socket| socket.send_capacity()))
        } else {
            ax_err!(NotConnected, "socket is not connected")
        }
    }
}

/// Private methods
impl TcpSocket {
    fn connect_loopback(&self, remote_addr: SocketAddr) -> AxResult {
        self.update_state(STATE_CLOSED, STATE_CONNECTED, || {
            let bound_endpoint = self.bound_endpoint()?;
            let peer_endpoint = IpEndpoint::from(remote_addr);
            let local_endpoint = IpEndpoint::new(
                bound_endpoint.addr.unwrap_or(peer_endpoint.addr),
                bound_endpoint.port,
            );
            let (client_endpoint, server_endpoint) =
                LoopbackTcpEndpoint::pair(local_endpoint, peer_endpoint);

            LISTEN_TABLE.connect_loopback(peer_endpoint, server_endpoint)?;
            unsafe {
                self.local_addr.get().write(local_endpoint);
                self.peer_addr.get().write(peer_endpoint);
                self.loopback.get().write(Some(client_endpoint));
            }
            Ok(())
        })
        .unwrap_or_else(|_| ax_err!(AlreadyExists, "socket connect() failed: already connected"))?;
        Ok(())
    }

    fn loopback_endpoint(&self) -> Option<LoopbackTcpEndpoint> {
        unsafe { (&*self.loopback.get()).clone() }
    }

    #[inline]
    fn get_state(&self) -> u8 {
        self.state.load(Ordering::Acquire)
    }

    #[inline]
    fn set_state(&self, state: u8) {
        self.state.store(state, Ordering::Release);
    }

    /// Update the state of the socket atomically.
    ///
    /// If the current state is `expect`, it first changes the state to `STATE_BUSY`,
    /// then calls the given function. If the function returns `Ok`, it changes the
    /// state to `new`, otherwise it changes the state back to `expect`.
    ///
    /// It returns `Ok` if the current state is `expect`, otherwise it returns
    /// the current state in `Err`.
    fn update_state<F, T>(&self, expect: u8, new: u8, f: F) -> Result<AxResult<T>, u8>
    where
        F: FnOnce() -> AxResult<T>,
    {
        match self
            .state
            .compare_exchange(expect, STATE_BUSY, Ordering::Acquire, Ordering::Acquire)
        {
            Ok(_) => {
                let res = f();
                if res.is_ok() {
                    self.set_state(new);
                } else {
                    self.set_state(expect);
                }
                Ok(res)
            }
            Err(old) => Err(old),
        }
    }

    #[inline]
    fn is_connecting(&self) -> bool {
        self.get_state() == STATE_CONNECTING
    }

    #[inline]
    fn is_connected(&self) -> bool {
        self.get_state() == STATE_CONNECTED
    }

    #[inline]
    fn is_listening(&self) -> bool {
        self.get_state() == STATE_LISTENING
    }

    fn bound_endpoint(&self) -> AxResult<IpListenEndpoint> {
        // SAFETY: no other threads can read or write `self.local_addr`.
        let local_addr = unsafe { self.local_addr.get().read() };
        let port = if local_addr.port != 0 {
            local_addr.port
        } else {
            get_ephemeral_port()?
        };
        assert_ne!(port, 0);
        let addr = if !local_addr.addr.is_unspecified() {
            Some(local_addr.addr)
        } else {
            None
        };
        Ok(IpListenEndpoint { addr, port })
    }

    fn poll_connect(&self) -> AxResult<PollState> {
        // SAFETY: `self.handle` should be initialized above.
        let handle = unsafe { self.handle.get().read().unwrap() };
        let writable =
            SOCKET_SET.with_socket::<tcp::Socket, _, _>(handle, |socket| match socket.state() {
                State::SynSent => false, // wait for connection
                State::Established => {
                    self.set_state(STATE_CONNECTED); // connected
                    debug!(
                        "TCP socket {}: connected to {}",
                        handle,
                        socket.remote_endpoint().unwrap(),
                    );
                    true
                }
                _ => {
                    unsafe {
                        self.local_addr.get().write(UNSPECIFIED_ENDPOINT);
                        self.peer_addr.get().write(UNSPECIFIED_ENDPOINT);
                    }
                    self.set_state(STATE_CLOSED); // connection failed
                    true
                }
            });
        Ok(PollState {
            readable: false,
            writable,
        })
    }

    fn poll_stream(&self) -> AxResult<PollState> {
        let recv_shutdown = self.recv_shutdown.load(Ordering::Acquire);
        let send_shutdown = self.send_shutdown.load(Ordering::Acquire);
        if let Some(endpoint) = self.loopback_endpoint() {
            let mut state = endpoint.poll();
            state.readable |= recv_shutdown;
            state.writable &= !send_shutdown;
            return Ok(state);
        }

        // SAFETY: `self.handle` should be initialized in a connected socket.
        let handle = unsafe { self.handle.get().read().unwrap() };
        SOCKET_SET.with_socket::<tcp::Socket, _, _>(handle, |socket| {
            Ok(PollState {
                readable: recv_shutdown || !socket.may_recv() || socket.can_recv(),
                writable: !send_shutdown && (!socket.may_send() || socket.can_send()),
            })
        })
    }

    fn poll_listener(&self) -> AxResult<PollState> {
        // SAFETY: `self.local_addr` should be initialized in a listening socket.
        let local_addr = unsafe { self.local_addr.get().read() };
        Ok(PollState {
            readable: LISTEN_TABLE.can_accept(local_addr.port)?,
            writable: false,
        })
    }

    /// Block the current thread until the given function completes or fails.
    ///
    /// If the socket is non-blocking, it calls the function once and returns
    /// immediately. Otherwise, it may call the function multiple times if it
    /// returns [`Err(WouldBlock)`](AxError::WouldBlock).
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

impl Drop for TcpSocket {
    fn drop(&mut self) {
        self.shutdown().ok();
        // Safe because we have mut reference to `self`.
        if let Some(handle) = unsafe { self.handle.get().read() } {
            SOCKET_SET.remove(handle);
        }
    }
}

fn get_ephemeral_port() -> AxResult<u16> {
    const PORT_START: u16 = 0xc000;
    const PORT_END: u16 = 0xffff;
    static CURR: Mutex<u16> = Mutex::new(PORT_START);

    let mut curr = CURR.lock();
    let mut tries = 0;
    // TODO: more robust
    while tries <= PORT_END - PORT_START {
        let port = *curr;
        if *curr == PORT_END {
            *curr = PORT_START;
        } else {
            *curr += 1;
        }
        if LISTEN_TABLE.can_listen(port) {
            return Ok(port);
        }
        tries += 1;
    }
    ax_err!(AddrInUse, "no avaliable ports!")
}
