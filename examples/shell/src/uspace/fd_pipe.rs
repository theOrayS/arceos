use axerrno::LinuxError;
use axio::PollState;
use axsync::Mutex;
use linux_raw_sys::general;
use std::sync::Arc;

#[derive(Clone, Copy, Eq, PartialEq)]
enum RingBufferStatus {
    Full,
    Empty,
    Normal,
}

const PIPE_BUF_SIZE: usize = 256;

struct PipeRingBuffer {
    data: [u8; PIPE_BUF_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
}

#[derive(Clone)]
pub(super) struct PipeEndpoint {
    readable: bool,
    buffer: Arc<Mutex<PipeRingBuffer>>,
}

impl PipeRingBuffer {
    const fn new() -> Self {
        Self {
            data: [0; PIPE_BUF_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::Empty,
        }
    }

    fn write_byte(&mut self, byte: u8) {
        self.status = RingBufferStatus::Normal;
        self.data[self.tail] = byte;
        self.tail = (self.tail + 1) % PIPE_BUF_SIZE;
        if self.tail == self.head {
            self.status = RingBufferStatus::Full;
        }
    }

    fn read_byte(&mut self) -> u8 {
        self.status = RingBufferStatus::Normal;
        let byte = self.data[self.head];
        self.head = (self.head + 1) % PIPE_BUF_SIZE;
        if self.head == self.tail {
            self.status = RingBufferStatus::Empty;
        }
        byte
    }

    const fn available_read(&self) -> usize {
        if matches!(self.status, RingBufferStatus::Empty) {
            0
        } else if self.tail > self.head {
            self.tail - self.head
        } else {
            self.tail + PIPE_BUF_SIZE - self.head
        }
    }

    const fn available_write(&self) -> usize {
        if matches!(self.status, RingBufferStatus::Full) {
            0
        } else {
            PIPE_BUF_SIZE - self.available_read()
        }
    }
}

impl PipeEndpoint {
    pub(super) fn new_pair() -> (Self, Self) {
        let buffer = Arc::new(Mutex::new(PipeRingBuffer::new()));
        (
            Self {
                readable: true,
                buffer: buffer.clone(),
            },
            Self {
                readable: false,
                buffer,
            },
        )
    }

    const fn writable(&self) -> bool {
        !self.readable
    }

    fn peer_closed(&self) -> bool {
        Arc::strong_count(&self.buffer) == 1
    }

    pub(super) fn read(&self, dst: &mut [u8]) -> Result<usize, LinuxError> {
        if !self.readable {
            return Err(LinuxError::EBADF);
        }
        let mut read_len = 0usize;
        while read_len < dst.len() {
            let mut ring = self.buffer.lock();
            let available = ring.available_read();
            if available == 0 {
                if read_len > 0 || self.peer_closed() {
                    return Ok(read_len);
                }
                drop(ring);
                axtask::yield_now();
                continue;
            }
            for _ in 0..available {
                if read_len == dst.len() {
                    return Ok(read_len);
                }
                dst[read_len] = ring.read_byte();
                read_len += 1;
            }
            if read_len > 0 {
                return Ok(read_len);
            }
        }
        Ok(read_len)
    }

    pub(super) fn write(&self, src: &[u8]) -> Result<usize, LinuxError> {
        if !self.writable() {
            return Err(LinuxError::EBADF);
        }
        let mut written = 0usize;
        while written < src.len() {
            let mut ring = self.buffer.lock();
            let available = ring.available_write();
            if available == 0 {
                drop(ring);
                axtask::yield_now();
                continue;
            }
            for _ in 0..available {
                if written == src.len() {
                    return Ok(written);
                }
                ring.write_byte(src[written]);
                written += 1;
            }
        }
        Ok(written)
    }

    pub(super) fn stat(&self) -> general::stat {
        let mut st: general::stat = unsafe { core::mem::zeroed() };
        st.st_ino = 1;
        st.st_mode = 0o010000 | 0o600;
        st.st_nlink = 1;
        st.st_blksize = PIPE_BUF_SIZE as _;
        st
    }

    pub(super) fn poll(&self) -> PollState {
        let ring = self.buffer.lock();
        PollState {
            readable: self.readable && (ring.available_read() > 0 || self.peer_closed()),
            writable: self.writable() && (ring.available_write() > 0 || self.peer_closed()),
        }
    }
}
