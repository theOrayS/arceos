use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp;
use core::ffi::{CStr, c_char, c_int, c_long};
use core::mem::{offset_of, size_of};
#[cfg(feature = "net")]
use core::net::SocketAddr;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;

use axalloc::global_allocator;
use axerrno::LinuxError;
use axfs::fops::{self, Directory, File, FileAttr, FileType, OpenOptions};
use axhal::context::{TrapFrame, UspaceContext};
use axhal::mem::virt_to_phys;
use axhal::paging::MappingFlags;
use axhal::trap::{
    PAGE_FAULT, PageFaultFlags, SYSCALL, register_trap_handler, register_user_return_handler,
};
use axio::{PollState, SeekFrom};
use axmm::AddrSpace;
#[cfg(feature = "net")]
use axnet::{TcpSocket, UdpSocket};
use axns::AxNamespace;
use axsync::Mutex;
use axtask::{AxTaskRef, TaskInner, WaitQueue};
use lazyinit::LazyInit;
use linux_raw_sys::{auxvec, general, ioctl};
use memory_addr::{PAGE_SIZE_4K, PageIter4K, VirtAddr};
use xmas_elf::ElfFile;
use xmas_elf::header::{Machine, Type as ElfType};
use xmas_elf::program::{Flags as PhFlags, ProgramHeader, Type as PhType};

#[cfg(feature = "net")]
use crate::ctypes;
use crate::imp::{
    fs::{
        FileSystemStat, apply_path_times_to_stat, metadata_to_linux_stat, statfs_for_path,
        symlink_to_linux_stat, update_path_times,
    },
    io_mpx::{PollEvent, poll_events},
    net::resolve_socket_spec,
    system as imp_system, task as imp_task, time as imp_time,
};

#[cfg(target_arch = "riscv64")]
use riscv::register::sstatus::{FS, Sstatus};

const USER_ASPACE_BASE: usize = 0x1_0000;
const USER_ASPACE_SIZE: usize = 0x20_0000_0000;
const USER_STACK_SIZE: usize = 8 * 1024 * 1024;
const USER_STACK_GUARD: usize = 0x1_0000;
const USER_STACK_TOP: usize = USER_ASPACE_BASE + USER_ASPACE_SIZE - USER_STACK_GUARD;
const USER_MMAP_BASE: usize = 0x10_0000_0000;
const USER_BRK_GROW_SIZE: usize = 64 * 1024 * 1024;
const USER_PIE_LOAD_BASE: usize = USER_ASPACE_BASE;
const MAX_SCRIPT_INTERPRETER_DEPTH: usize = 4;
const TESTSUITE_STAGE_ROOT: &str = "/tmp/testsuite";
const AUX_CLOCK_TICKS: usize = 100;
const SIGCHLD_NUM: isize = 17;
const SIGCANCEL_NUM: i32 = 33;
const SI_TKILL_CODE: i32 = -6;
const SA_NODEFER_FLAG: u64 = 0x4000_0000;
const KERNEL_SIGSET_BYTES: usize = size_of::<u64>();
const SIG_BLOCK_HOW: usize = 0;
const SIG_UNBLOCK_HOW: usize = 1;
const SIG_SETMASK_HOW: usize = 2;
const RLIMIT_STACK_RESOURCE: u32 = 3;
const RLIMIT_NOFILE_RESOURCE: u32 = 7;
const DEFAULT_NOFILE_LIMIT: u64 = 1024;
const FD_SETSIZE: usize = 1024;
const BITS_PER_USIZE: usize = usize::BITS as usize;
const FD_SET_WORDS: usize = FD_SETSIZE.div_ceil(BITS_PER_USIZE);
const FD_CLOEXEC_FLAG: u32 = 1;
const IPC_PRIVATE_KEY: i32 = 0;
const IPC_CREAT_FLAG: i32 = 0o1000;
const IPC_EXCL_FLAG: i32 = 0o2000;
const IPC_RMID_CMD: i32 = 0;
const IPC_SET_CMD: i32 = 1;
const IPC_STAT_CMD: i32 = 2;
const SHM_RDONLY_FLAG: i32 = 0o10000;
const SHM_RND_FLAG: i32 = 0o20000;
const SHM_REMAP_FLAG: i32 = 0o40000;
const SHM_DEST_FLAG: u32 = 0o1000;
#[cfg(feature = "net")]
const SOL_SOCKET_LEVEL: i32 = 1;
#[cfg(feature = "net")]
const SO_REUSEADDR_OPT: i32 = 2;
#[cfg(feature = "net")]
const SO_TYPE_OPT: i32 = 3;
#[cfg(feature = "net")]
const SO_ERROR_OPT: i32 = 4;
#[cfg(feature = "net")]
const SO_BROADCAST_OPT: i32 = 6;
#[cfg(feature = "net")]
const SO_SNDBUF_OPT: i32 = 7;
#[cfg(feature = "net")]
const SO_RCVBUF_OPT: i32 = 8;
#[cfg(feature = "net")]
const SO_KEEPALIVE_OPT: i32 = 9;
#[cfg(feature = "net")]
const SO_REUSEPORT_OPT: i32 = 15;
#[cfg(feature = "net")]
const SO_RCVTIMEO_OPT: i32 = 20;
#[cfg(feature = "net")]
const SO_SNDTIMEO_OPT: i32 = 21;
#[cfg(feature = "net")]
const TCP_NODELAY_OPT: i32 = 1;
#[cfg(feature = "net")]
const TCP_MAXSEG_OPT: i32 = 2;
#[cfg(feature = "net")]
const TCP_INFO_OPT: i32 = 11;
#[cfg(feature = "net")]
const TCP_CONGESTION_OPT: i32 = 13;
#[cfg(feature = "net")]
const IPPROTO_TCP_LEVEL: i32 = ctypes::IPPROTO_TCP as i32;
#[cfg(feature = "net")]
const DEFAULT_SOCKET_BUFFER_SIZE: u32 = 256 * 1024;
#[cfg(feature = "net")]
const DEFAULT_TCP_MAXSEG: i32 = 1460;
#[cfg(feature = "net")]
const DEFAULT_TCP_CONGESTION: &[u8] = b"reno\0";
#[cfg(target_arch = "riscv64")]
const RISCV_SIGNAL_SIGSET_RESERVED_BYTES: usize = 120;
#[cfg(target_arch = "riscv64")]
const RISCV_SIGNAL_FPSTATE_BYTES: usize = 528;
const SS_DISABLE: i32 = 2;
#[cfg(target_arch = "riscv64")]
const RISCV_SIGTRAMP_CODE: [u32; 3] = [0x08b0_0893, 0x0000_0073, 0x0010_0073];
#[cfg(target_arch = "loongarch64")]
const LOONGARCH_SIGTRAMP_CODE: [u32; 2] = [0x02c2_2c0b, 0x002b_0000];

const ST_MODE_DIR: u32 = 0o040000;
const ST_MODE_FILE: u32 = 0o100000;
const ST_MODE_CHR: u32 = 0o020000;
const UTIME_OMIT_NSEC: c_long = 0x3fff_fffe;
const UTIME_NOW_NSEC: c_long = 0x3fff_ffff;
const SOCK_CLOEXEC_FLAG: u32 = 0o2000000;
const SOCK_NONBLOCK_FLAG: u32 = 0o4000;
const SOCK_TYPE_MASK: u32 = 0xf;
const AF_INET_NUM: u32 = 2;
const SOCK_STREAM_NUM: u32 = 1;
const SOCK_DGRAM_NUM: u32 = 2;
const SOL_SOCKET_NUM: u32 = 1;
const SO_RCVTIMEO_NUM: u32 = 20;
const IPPROTO_TCP_NUM: u32 = 6;
const IPPROTO_UDP_NUM: u32 = 17;
const FUTEX_REQUEUE_CMD: u32 = 3;
const FUTEX_CMP_REQUEUE_CMD: u32 = 4;
const FUTEX_WAKE_OP_CMD: u32 = 5;
const FUTEX_WAIT_BITSET_CMD: u32 = 9;
const FUTEX_WAKE_BITSET_CMD: u32 = 10;
const FUTEX_CLOCK_REALTIME_FLAG: u32 = 0x100;

#[cfg(target_arch = "riscv64")]
const AUX_PLATFORM: &str = "riscv64";
#[cfg(target_arch = "loongarch64")]
const AUX_PLATFORM: &str = "loongarch64";

static USER_RETURN_HOOK_REGISTERED: AtomicBool = AtomicBool::new(false);

macro_rules! user_trace {
    ($($arg:tt)*) => {};
}

struct UserTaskExt {
    process: Arc<UserProcess>,
    clear_child_tid: AtomicUsize,
    pending_signal: AtomicI32,
    signal_mask: AtomicU64,
    signal_wait: WaitQueue,
    futex_wait: AtomicUsize,
    futex_token: Mutex<Option<FutexWaitToken>>,
    robust_list_head: AtomicUsize,
    robust_list_len: AtomicUsize,
    deferred_unmap_start: AtomicUsize,
    deferred_unmap_len: AtomicUsize,
    signal_frame: AtomicUsize,
    sigcancel_delivery_armed: AtomicBool,
    pending_sigreturn: Mutex<Option<TrapFrame>>,
}

axtask::def_task_ext!(UserTaskExt);

struct AxNamespaceImpl;

struct UserProcess {
    aspace: Mutex<AddrSpace>,
    brk: Mutex<BrkState>,
    fds: Mutex<FdTable>,
    shared_attaches: Mutex<BTreeMap<usize, SharedMemAttach>>,
    cwd: Mutex<String>,
    exec_root: Mutex<String>,
    exec_path: Mutex<String>,
    children: Mutex<Vec<ChildTask>>,
    rlimits: Mutex<BTreeMap<u32, UserRlimit>>,
    signal_actions: Mutex<BTreeMap<usize, general::kernel_sigaction>>,
    pid: AtomicI32,
    ppid: i32,
    live_threads: AtomicUsize,
    exit_group_code: AtomicI32,
    exit_code: AtomicI32,
    exit_wait: WaitQueue,
}

#[derive(Clone, Copy)]
struct BrkState {
    start: usize,
    end: usize,
    limit: usize,
    next_mmap: usize,
}

struct FdTable {
    entries: Vec<Option<FdEntry>>,
    path_times: BTreeMap<String, FileTimes>,
    next_socket_port: u16,
    limit: usize,
}

enum FdEntry {
    Stdin,
    Stdout,
    Stderr,
    DevNull,
    File(FileEntry),
    Directory(DirectoryEntry),
    Pipe(PipeEndpoint),
    Socket(SocketEntry),
}

#[derive(Clone)]
struct FileEntry {
    file: File,
    path: String,
    times: FileTimes,
}

#[derive(Clone)]
struct DirectoryEntry {
    dir: Directory,
    attr: FileAttr,
    path: String,
    times: FileTimes,
}

#[derive(Clone)]
struct SocketEntry {
    kind: SocketKind,
    cloexec: bool,
    nonblock: bool,
    local_port: u16,
    recv_queue: Vec<Vec<u8>>,
    listening: bool,
    pending_stream: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SocketKind {
    Datagram,
    Stream,
}

#[derive(Clone, Copy)]
struct FileTimes {
    atime: general::timespec,
    mtime: general::timespec,
    ctime: general::timespec,
}

#[derive(Clone, Copy)]
struct UtimeUpdate {
    time: Option<general::timespec>,
    omit: bool,
}

impl Default for FileTimes {
    fn default() -> Self {
        let zero = zero_timespec();
        Self {
            atime: zero,
            mtime: zero,
            ctime: zero,
        }
    }
}

#[cfg(feature = "net")]
#[derive(Clone)]
struct SocketEntry {
    socket: Arc<UserSocket>,
    status_flags: Arc<AtomicU32>,
    pending_error: Arc<AtomicI32>,
    recv_buf_size: Arc<AtomicU32>,
    send_buf_size: Arc<AtomicU32>,
    recv_timeout_us: Arc<AtomicU64>,
    send_timeout_us: Arc<AtomicU64>,
    socket_type: u32,
}

#[cfg(feature = "net")]
enum UserSocket {
    Udp(Mutex<UdpSocket>),
    Tcp(Mutex<TcpSocket>),
    LocalStream(LocalStreamSocket),
}

#[derive(Clone, Copy)]
struct FdStatTimes {
    atime: general::timespec,
    mtime: general::timespec,
    ctime: general::timespec,
}

#[cfg(feature = "net")]
impl UserSocket {
    fn bind(&self, addr: SocketAddr) -> Result<(), LinuxError> {
        match self {
            Self::Udp(socket) => socket.lock().bind(addr).map_err(LinuxError::from),
            Self::Tcp(socket) => socket.lock().bind(addr).map_err(LinuxError::from),
            Self::LocalStream(_) => Err(LinuxError::EOPNOTSUPP),
        }
    }

    fn connect(&self, addr: SocketAddr) -> Result<(), LinuxError> {
        match self {
            Self::Udp(socket) => socket.lock().connect(addr).map_err(LinuxError::from),
            Self::Tcp(socket) => socket.lock().connect(addr).map_err(LinuxError::from),
            Self::LocalStream(_) => Err(LinuxError::EOPNOTSUPP),
        }
    }

    fn listen(&self) -> Result<(), LinuxError> {
        match self {
            Self::Udp(_) => Err(LinuxError::EOPNOTSUPP),
            Self::Tcp(socket) => socket.lock().listen().map_err(LinuxError::from),
            Self::LocalStream(_) => Err(LinuxError::EOPNOTSUPP),
        }
    }

    fn accept(&self) -> Result<TcpSocket, LinuxError> {
        match self {
            Self::Udp(_) => Err(LinuxError::EOPNOTSUPP),
            Self::Tcp(socket) => socket.lock().accept().map_err(LinuxError::from),
            Self::LocalStream(_) => Err(LinuxError::EOPNOTSUPP),
        }
    }

    fn send(&self, buf: &[u8]) -> Result<usize, LinuxError> {
        match self {
            Self::Udp(socket) => socket.lock().send(buf).map_err(LinuxError::from),
            Self::Tcp(socket) => socket.lock().send(buf).map_err(LinuxError::from),
            Self::LocalStream(socket) => socket.send(buf),
        }
    }

    fn recv(&self, buf: &mut [u8]) -> Result<usize, LinuxError> {
        match self {
            Self::Udp(socket) => socket.lock().recv(buf).map_err(LinuxError::from),
            Self::Tcp(socket) => socket.lock().recv(buf).map_err(LinuxError::from),
            Self::LocalStream(socket) => socket.recv(buf),
        }
    }

    fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize, LinuxError> {
        match self {
            Self::Udp(socket) => socket.lock().send_to(buf, addr).map_err(LinuxError::from),
            Self::Tcp(_) => Err(LinuxError::EISCONN),
            Self::LocalStream(_) => Err(LinuxError::EOPNOTSUPP),
        }
    }

    fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, Option<SocketAddr>), LinuxError> {
        match self {
            Self::Udp(socket) => socket
                .lock()
                .recv_from(buf)
                .map(|(len, addr)| (len, Some(addr)))
                .map_err(LinuxError::from),
            Self::Tcp(socket) => socket
                .lock()
                .recv(buf)
                .map(|len| (len, None))
                .map_err(LinuxError::from),
            Self::LocalStream(socket) => socket.recv(buf).map(|len| (len, None)),
        }
    }

    fn shutdown(&self) -> Result<(), LinuxError> {
        match self {
            Self::Udp(socket) => socket.lock().shutdown().map_err(LinuxError::from),
            Self::Tcp(socket) => socket.lock().shutdown().map_err(LinuxError::from),
            Self::LocalStream(socket) => socket.shutdown(),
        }
    }

    fn local_addr(&self) -> Result<SocketAddr, LinuxError> {
        match self {
            Self::Udp(socket) => socket.lock().local_addr().map_err(LinuxError::from),
            Self::Tcp(socket) => socket.lock().local_addr().map_err(LinuxError::from),
            Self::LocalStream(_) => Err(LinuxError::EOPNOTSUPP),
        }
    }

    fn peer_addr(&self) -> Result<SocketAddr, LinuxError> {
        match self {
            Self::Udp(socket) => socket.lock().peer_addr().map_err(LinuxError::from),
            Self::Tcp(socket) => socket.lock().peer_addr().map_err(LinuxError::from),
            Self::LocalStream(_) => Err(LinuxError::EOPNOTSUPP),
        }
    }

    fn poll(&self) -> Result<PollState, LinuxError> {
        match self {
            Self::Udp(socket) => socket.lock().poll().map_err(LinuxError::from),
            Self::Tcp(socket) => socket.lock().poll().map_err(LinuxError::from),
            Self::LocalStream(socket) => Ok(socket.poll()),
        }
    }

    fn set_nonblocking(&self, nonblocking: bool) {
        match self {
            Self::Udp(socket) => socket.lock().set_nonblocking(nonblocking),
            Self::Tcp(socket) => socket.lock().set_nonblocking(nonblocking),
            Self::LocalStream(socket) => socket.set_nonblocking(nonblocking),
        }
    }

    fn set_nodelay(&self, enabled: bool) -> Result<(), LinuxError> {
        match self {
            Self::Udp(_) => Err(LinuxError::EOPNOTSUPP),
            Self::Tcp(socket) => socket.lock().set_nodelay(enabled).map_err(LinuxError::from),
            Self::LocalStream(_) => Err(LinuxError::EOPNOTSUPP),
        }
    }

    fn nodelay(&self) -> Result<bool, LinuxError> {
        match self {
            Self::Udp(_) => Err(LinuxError::EOPNOTSUPP),
            Self::Tcp(socket) => socket.lock().nodelay().map_err(LinuxError::from),
            Self::LocalStream(_) => Err(LinuxError::EOPNOTSUPP),
        }
    }
}

struct ChildTask {
    pid: i32,
    task: AxTaskRef,
    process: Arc<UserProcess>,
}

#[derive(Clone)]
struct UserThreadEntry {
    task: AxTaskRef,
    process: Arc<UserProcess>,
}

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

struct PipeInner {
    ring: Mutex<PipeRingBuffer>,
    readers: AtomicUsize,
    writers: AtomicUsize,
}

struct PipeEndpoint {
    readable: bool,
    inner: Arc<PipeInner>,
}

struct LoadedProgram {
    process: Arc<UserProcess>,
    context: UspaceContext,
}

struct LoadedImage {
    entry: usize,
    stack_ptr: usize,
    argc: usize,
    brk: BrkState,
    exec_root: String,
    exec_path: String,
}

struct PreparedProgram {
    image: Vec<u8>,
    argv: Vec<String>,
    path: String,
    exec_root: String,
}

struct ElfLoadInfo {
    load_bias: usize,
    entry: usize,
    phdr: usize,
    max_segment_end: usize,
    base: usize,
    interpreter: Option<String>,
}

struct FutexState {
    seq: Arc<AtomicU32>,
    queue: WaitQueue,
}

#[derive(Clone)]
struct FutexWaitToken {
    seq: Arc<AtomicU32>,
    expected: u32,
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
struct FutexKey {
    uaddr: usize,
    bitset: u32,
}

#[derive(Clone, Copy)]
enum FutexTimeout {
    Relative(core::time::Duration),
    Absolute(core::time::Duration),
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Tms {
    tms_utime: c_long,
    tms_stime: c_long,
    tms_cutime: c_long,
    tms_cstime: c_long,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct AuxEntry {
    key: usize,
    value: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UserRlimit {
    rlim_cur: u64,
    rlim_max: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SignalInfo {
    bytes: [u8; 128],
}

#[cfg(target_arch = "riscv64")]
type RiscvSignalInfo = SignalInfo;

#[cfg(target_arch = "loongarch64")]
type LoongArchSignalInfo = SignalInfo;

#[cfg(target_arch = "riscv64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct RiscvSignalStack {
    sp: usize,
    stack_flags: i32,
    stack_pad: i32,
    size: usize,
}

#[cfg(target_arch = "riscv64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct RiscvKernelSigset {
    sig: [u64; 1],
    reserved: [u8; RISCV_SIGNAL_SIGSET_RESERVED_BYTES],
}

#[cfg(target_arch = "riscv64")]
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct RiscvSignalFpState {
    bytes: [u8; RISCV_SIGNAL_FPSTATE_BYTES],
}

#[cfg(target_arch = "riscv64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct RiscvSignalSigcontext {
    gregs: [usize; 32],
    fpstate: RiscvSignalFpState,
}

#[cfg(target_arch = "riscv64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct RiscvSignalUcontext {
    flags: usize,
    link: usize,
    stack: RiscvSignalStack,
    sigmask: RiscvKernelSigset,
    mcontext: RiscvSignalSigcontext,
}

#[cfg(target_arch = "riscv64")]
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct RiscvSignalFrame {
    info: RiscvSignalInfo,
    ucontext: RiscvSignalUcontext,
    trampoline: [u32; 3],
}

#[cfg(target_arch = "loongarch64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct LoongArchSignalStack {
    sp: usize,
    stack_flags: i32,
    stack_pad: i32,
    size: usize,
}

#[cfg(target_arch = "loongarch64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct LoongArchKernelSigset {
    sig: [u64; 1],
    reserved: [u8; LOONGARCH_SIGNAL_SIGSET_RESERVED_BYTES],
}

#[cfg(target_arch = "loongarch64")]
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct LoongArchSignalMcontext {
    pc: usize,
    regs: [usize; 32],
    flags: u32,
    pad: u32,
}

#[cfg(target_arch = "loongarch64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct LoongArchSignalUcontext {
    flags: usize,
    link: usize,
    stack: LoongArchSignalStack,
    sigmask: LoongArchKernelSigset,
    pad: isize,
    mcontext: LoongArchSignalMcontext,
}

#[cfg(target_arch = "loongarch64")]
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct LoongArchSignalExtcontextEnd {
    magic: u32,
    size: u32,
    padding: u64,
}

#[cfg(target_arch = "loongarch64")]
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct LoongArchSignalFrame {
    info: LoongArchSignalInfo,
    ucontext: LoongArchSignalUcontext,
    extcontext_end: LoongArchSignalExtcontextEnd,
    trampoline: [u32; 3],
}

#[cfg(target_arch = "riscv64")]
const _: [(); RISCV_SIGNAL_FPSTATE_BYTES] = [(); size_of::<RiscvSignalFpState>()];
#[cfg(target_arch = "riscv64")]
const _: [(); 784] = [(); size_of::<RiscvSignalSigcontext>()];
#[cfg(target_arch = "riscv64")]
const _: [(); 960] = [(); size_of::<RiscvSignalUcontext>()];
#[cfg(target_arch = "riscv64")]
const _: [(); 1104] = [(); size_of::<RiscvSignalFrame>()];
#[cfg(target_arch = "loongarch64")]
const _: [(); 128] = [(); size_of::<LoongArchKernelSigset>()];
#[cfg(target_arch = "loongarch64")]
const _: [(); 272] = [(); size_of::<LoongArchSignalMcontext>()];
#[cfg(target_arch = "loongarch64")]
const _: [(); 448] = [(); size_of::<LoongArchSignalUcontext>()];
#[cfg(target_arch = "loongarch64")]
const _: [(); 608] = [(); size_of::<LoongArchSignalFrame>()];

#[cfg(target_arch = "loongarch64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct LoongarchSignalInfo {
    bytes: [u8; 128],
}

#[cfg(target_arch = "loongarch64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct LoongarchSignalStack {
    sp: usize,
    stack_flags: i32,
    stack_pad: i32,
    size: usize,
}

#[cfg(target_arch = "loongarch64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct LoongarchSignalSigset {
    sig: [u64; 16],
}

#[cfg(target_arch = "loongarch64")]
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct LoongarchSignalMcontext {
    pc: usize,
    regs: [usize; 32],
    flags: u32,
    pad: u32,
}

#[cfg(target_arch = "loongarch64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct LoongarchSignalUcontext {
    flags: usize,
    link: usize,
    stack: LoongarchSignalStack,
    sigmask: LoongarchSignalSigset,
    pad: c_long,
    mcontext: LoongarchSignalMcontext,
}

#[cfg(target_arch = "loongarch64")]
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct LoongarchSignalFrame {
    info: LoongarchSignalInfo,
    ucontext: LoongarchSignalUcontext,
    trampoline: [u32; 2],
}

#[cfg(target_arch = "loongarch64")]
const _: [(); 272] = [(); size_of::<LoongarchSignalMcontext>()];
#[cfg(target_arch = "loongarch64")]
const _: [(); 448] = [(); size_of::<LoongarchSignalUcontext>()];

#[repr(C)]
#[derive(Clone, Copy)]
struct UserFdSet {
    fds_bits: [usize; FD_SET_WORDS],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SockAddrIn {
    sin_family: u16,
    sin_port: u16,
    sin_addr: u32,
    sin_zero: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UserSchedParam {
    sched_priority: i32,
}

const NO_EXIT_GROUP_CODE: i32 = i32::MIN;

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
    fn new_pair() -> (Self, Self) {
        let inner = Arc::new(PipeInner {
            ring: Mutex::new(PipeRingBuffer::new()),
            readers: AtomicUsize::new(1),
            writers: AtomicUsize::new(1),
        });
        (
            Self {
                readable: true,
                inner: inner.clone(),
            },
            Self {
                readable: false,
                inner,
            },
        )
    }

    const fn writable(&self) -> bool {
        !self.readable
    }

    fn peer_closed(&self) -> bool {
        if self.readable {
            self.inner.writers.load(Ordering::Acquire) == 0
        } else {
            self.inner.readers.load(Ordering::Acquire) == 0
        }
    }

    fn read(&self, dst: &mut [u8]) -> Result<usize, LinuxError> {
        self.read_with_mode(dst, self.nonblocking())
    }

    fn read_with_mode(&self, dst: &mut [u8], nonblocking: bool) -> Result<usize, LinuxError> {
        if !self.readable {
            return Err(LinuxError::EBADF);
        }
        let mut read_len = 0usize;
        while read_len < dst.len() {
            let mut ring = self.inner.ring.lock();
            let available = ring.available_read();
            if available == 0 {
                if read_len > 0 || self.peer_closed() {
                    return Ok(read_len);
                }
                if nonblocking {
                    return Err(LinuxError::EAGAIN);
                }
                drop(ring);
                if let Some(ext) = current_task_ext() {
                    if let Some(code) = ext.process.pending_exit_group() {
                        terminate_current_thread(ext.process.as_ref(), code);
                    }
                }
                if current_unblocked_pending_signal().is_some() {
                    return Err(LinuxError::EINTR);
                }
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

    fn write(&self, src: &[u8]) -> Result<usize, LinuxError> {
        self.write_with_mode(src, self.nonblocking())
    }

    fn write_with_mode(&self, src: &[u8], nonblocking: bool) -> Result<usize, LinuxError> {
        if !self.writable() {
            return Err(LinuxError::EBADF);
        }
        if self.peer_closed() {
            return Err(LinuxError::EPIPE);
        }
        let mut written = 0usize;
        while written < src.len() {
            let mut ring = self.inner.ring.lock();
            let available = ring.available_write();
            if self.peer_closed() {
                return if written > 0 {
                    Ok(written)
                } else {
                    Err(LinuxError::EPIPE)
                };
            }
            if available == 0 {
                if self.peer_closed() {
                    return if written > 0 {
                        Ok(written)
                    } else {
                        Err(LinuxError::EPIPE)
                    };
                }
                drop(ring);
                if let Some(ext) = current_task_ext() {
                    if let Some(code) = ext.process.pending_exit_group() {
                        terminate_current_thread(ext.process.as_ref(), code);
                    }
                }
                if current_unblocked_pending_signal().is_some() {
                    return if written > 0 {
                        Ok(written)
                    } else {
                        Err(LinuxError::EINTR)
                    };
                }
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

    fn stat(&self) -> general::stat {
        let mut st: general::stat = unsafe { core::mem::zeroed() };
        st.st_ino = 1;
        st.st_mode = 0o010000 | 0o600;
        st.st_nlink = 1;
        st.st_blksize = PIPE_BUF_SIZE as _;
        st
    }

    fn poll(&self) -> PollState {
        let ring = self.inner.ring.lock();
        PollState {
            readable: self.readable && (ring.available_read() > 0 || self.peer_closed()),
            writable: self.writable() && (ring.available_write() > 0 || self.peer_closed()),
        }
    }

    fn nonblocking(&self) -> bool {
        self.status_flags.load(Ordering::Acquire) & general::O_NONBLOCK != 0
    }

    fn status_flags(&self) -> u32 {
        self.status_flags.load(Ordering::Acquire) & general::O_NONBLOCK
    }

    fn set_status_flags(&self, flags: u32) {
        self.status_flags
            .store(flags & general::O_NONBLOCK, Ordering::Release);
    }
}

impl Clone for PipeEndpoint {
    fn clone(&self) -> Self {
        if self.readable {
            self.state.readers.fetch_add(1, Ordering::AcqRel);
        } else {
            self.state.writers.fetch_add(1, Ordering::AcqRel);
        }
        Self {
            readable: self.readable,
            state: self.state.clone(),
            status_flags: self.status_flags.clone(),
        }
    }
}

impl Drop for PipeEndpoint {
    fn drop(&mut self) {
        if self.readable {
            self.state.readers.fetch_sub(1, Ordering::AcqRel);
        } else {
            self.state.writers.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

impl LocalStreamSocket {
    fn new_pair() -> (Self, Self) {
        let (left_read, right_write) = PipeEndpoint::new_pair();
        let (right_read, left_write) = PipeEndpoint::new_pair();
        (
            Self {
                read_end: left_read,
                write_end: left_write,
                nonblocking: Arc::new(AtomicBool::new(false)),
            },
            Self {
                read_end: right_read,
                write_end: right_write,
                nonblocking: Arc::new(AtomicBool::new(false)),
            },
        )
    }

    fn recv(&self, buf: &mut [u8]) -> Result<usize, LinuxError> {
        self.read_end
            .read_with_mode(buf, self.nonblocking.load(Ordering::Acquire))
    }

    fn send(&self, buf: &[u8]) -> Result<usize, LinuxError> {
        self.write_end
            .write_with_mode(buf, self.nonblocking.load(Ordering::Acquire))
    }

    fn shutdown(&self) -> Result<(), LinuxError> {
        Ok(())
    }

    fn poll(&self) -> PollState {
        let read = self.read_end.poll();
        let write = self.write_end.poll();
        PollState {
            readable: read.readable,
            writable: write.writable,
        }
    }

    fn set_nonblocking(&self, nonblocking: bool) {
        self.nonblocking.store(nonblocking, Ordering::Release);
    }
}

impl SharedMemState {
    fn new() -> Self {
        Self {
            next_id: 1,
            next_seq: 1,
            segments: BTreeMap::new(),
            key_map: BTreeMap::new(),
        }
    }
}

impl Clone for PipeEndpoint {
    fn clone(&self) -> Self {
        if self.readable {
            self.inner.readers.fetch_add(1, Ordering::AcqRel);
        } else {
            self.inner.writers.fetch_add(1, Ordering::AcqRel);
        }
        Self {
            readable: self.readable,
            inner: self.inner.clone(),
        }
    }
}

impl Drop for PipeEndpoint {
    fn drop(&mut self) {
        if self.readable {
            self.inner.readers.fetch_sub(1, Ordering::AcqRel);
        } else {
            self.inner.writers.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

#[crate_interface::impl_interface]
impl axns::AxNamespaceIf for AxNamespaceImpl {
    fn current_namespace_base() -> *mut u8 {
        AxNamespace::global().base()
    }
}

pub fn run_user_program(argv: &[&str]) -> Result<i32, String> {
    run_user_program_in(current_cwd().as_str(), argv)
}

pub fn run_user_program_in(cwd: &str, argv: &[&str]) -> Result<i32, String> {
    ensure_user_return_hook_registered();
    let loaded = load_program(cwd, argv)?;
    let process = loaded.process.clone();
    let task_process = process.clone();
    let context = loaded.context;
    let mut task = TaskInner::new(
        move || user_task_entry(task_process, context),
        format!("user:{}", argv[0]),
        64 * 1024,
    );
    let root = loaded.process.aspace.lock().page_table_root();
    task.ctx_mut().set_page_table_root(root);
    task.init_task_ext(UserTaskExt {
        process: loaded.process.clone(),
        clear_child_tid: AtomicUsize::new(0),
        pending_signal: AtomicI32::new(0),
        signal_mask: AtomicU64::new(0),
        signal_wait: WaitQueue::new(),
        futex_wait: AtomicUsize::new(0),
        futex_token: Mutex::new(None),
        robust_list_head: AtomicUsize::new(0),
        robust_list_len: AtomicUsize::new(0),
        deferred_unmap_start: AtomicUsize::new(0),
        deferred_unmap_len: AtomicUsize::new(0),
        signal_frame: AtomicUsize::new(0),
        sigcancel_delivery_armed: AtomicBool::new(false),
        pending_sigreturn: Mutex::new(None),
    });
    let task = axtask::spawn_task(task);
    process.set_pid(task.id().as_u64() as i32);
    register_user_task(task.clone(), process.clone());
    let exit_code = process.wait_for_exit();
    let _ = task.join();
    // Reclaim the user address space immediately after exit. Exited tasks may
    // stay queued for GC a bit longer, and keeping all user pages pinned leaks
    // enough memory to break later launches.
    process.teardown();
    drop(task);
    axtask::yield_now();
    Ok(exit_code)
}

fn user_task_entry(_process: Arc<UserProcess>, context: UspaceContext) {
    let curr = axtask::current();
    let kstack_top = curr
        .kernel_stack_top()
        .expect("user task must have a kernel stack");
    unsafe { context.enter_uspace(kstack_top) }
}

fn user_thread_entry(process: Arc<UserProcess>, context: UspaceContext, child_tid_ptr: usize) {
    if child_tid_ptr != 0 {
        let tid = axtask::current().id().as_u64() as i32;
        let _ = write_user_value(process.as_ref(), child_tid_ptr, &tid);
    }
    user_task_entry(process, context)
}

fn load_program(cwd: &str, argv: &[&str]) -> Result<LoadedProgram, String> {
    let mut aspace = axmm::new_user_aspace(VirtAddr::from(USER_ASPACE_BASE), USER_ASPACE_SIZE)
        .map_err(|err| format!("failed to create user address space: {err}"))?;
    let image = load_program_image(&mut aspace, cwd, argv)?;

    let process = Arc::new(UserProcess {
        aspace: Mutex::new(aspace),
        brk: Mutex::new(image.brk),
        fds: Mutex::new(FdTable::new()),
        shared_attaches: Mutex::new(BTreeMap::new()),
        cwd: Mutex::new(cwd.into()),
        exec_root: Mutex::new(image.exec_root.clone()),
        exec_path: Mutex::new(image.exec_path.clone()),
        children: Mutex::new(Vec::new()),
        rlimits: Mutex::new(BTreeMap::new()),
        signal_actions: Mutex::new(BTreeMap::new()),
        pid: AtomicI32::new(0),
        ppid: 1,
        live_threads: AtomicUsize::new(1),
        exit_group_code: AtomicI32::new(NO_EXIT_GROUP_CODE),
        exit_code: AtomicI32::new(0),
        exit_wait: WaitQueue::new(),
    });

    Ok(LoadedProgram {
        process,
        context: make_uspace_context(image.entry, image.stack_ptr, image.argc),
    })
}

fn load_program_image(
    aspace: &mut AddrSpace,
    cwd: &str,
    argv: &[&str],
) -> Result<LoadedImage, String> {
    let prepared = prepare_program(cwd, argv, 0)?;
    let elf = ElfFile::new(&prepared.image).map_err(|err| format!("invalid ELF: {err}"))?;
    let main = analyze_elf(&elf, USER_PIE_LOAD_BASE)?;

    aspace.clear();

    map_elf_image(aspace, &prepared.image, &elf, &main)?;
    let mut max_mapped_end = main.max_segment_end;
    let mut runtime_entry = main.entry;
    let mut interp_base = 0usize;

    if let Some(raw_interp) = main.interpreter.as_deref() {
        let interp_path = resolve_runtime_support_file(prepared.exec_root.as_str(), raw_interp)?;
        let interp_image = axfs::api::read(interp_path.as_str())
            .map_err(|err| format!("failed to read interpreter {interp_path}: {err}"))?;
        let interp_elf =
            ElfFile::new(&interp_image).map_err(|err| format!("invalid interpreter ELF: {err}"))?;
        let interp = analyze_elf(
            &interp_elf,
            align_up(
                cmp::max(max_mapped_end + PAGE_SIZE_4K, USER_MMAP_BASE),
                PAGE_SIZE_4K,
            ),
        )?;
        map_elf_image(aspace, &interp_image, &interp_elf, &interp)?;
        max_mapped_end = cmp::max(max_mapped_end, interp.max_segment_end);
        runtime_entry = interp.entry;
        interp_base = interp.base;
    }

    let brk_start = align_up(main.max_segment_end, PAGE_SIZE_4K);
    let brk_limit = align_up(brk_start + USER_BRK_GROW_SIZE, PAGE_SIZE_4K);
    if brk_limit > USER_STACK_TOP - USER_STACK_SIZE {
        return Err("user virtual address space is too small".into());
    }

    aspace
        .map_alloc(
            VirtAddr::from(brk_start),
            brk_limit - brk_start,
            user_mapping_flags(true, true, false),
            false,
        )
        .map_err(|err| format!("failed to reserve brk area: {err}"))?;

    let stack_top = align_down(USER_STACK_TOP, PAGE_SIZE_4K);
    let stack_base = stack_top - USER_STACK_SIZE;
    aspace
        .map_alloc(
            VirtAddr::from(stack_base),
            USER_STACK_SIZE,
            user_mapping_flags(true, true, false),
            false,
        )
        .map_err(|err| format!("failed to map user stack: {err}"))?;

    let argv_refs = prepared.argv.iter().map(String::as_str).collect::<Vec<_>>();
    let stack_ptr = build_initial_stack(
        aspace,
        stack_base,
        stack_top,
        &argv_refs,
        prepared.path.as_str(),
        main.entry,
        interp_base,
        main.phdr,
        elf.header.pt2.ph_entry_size() as usize,
        elf.header.pt2.ph_count() as usize,
    )?;

    Ok(LoadedImage {
        entry: runtime_entry,
        stack_ptr,
        argc: prepared.argv.len(),
        brk: BrkState {
            start: brk_start,
            end: brk_start,
            limit: brk_limit,
            next_mmap: align_up(
                cmp::max(
                    max_mapped_end + PAGE_SIZE_4K,
                    cmp::max(brk_limit + PAGE_SIZE_4K, USER_MMAP_BASE),
                ),
                PAGE_SIZE_4K,
            ),
        },
        exec_root: prepared.exec_root,
        exec_path: prepared.path,
    })
}

fn prepare_program(cwd: &str, argv: &[&str], depth: usize) -> Result<PreparedProgram, String> {
    if argv.is_empty() {
        return Err("empty argv".into());
    }
    if depth > MAX_SCRIPT_INTERPRETER_DEPTH {
        return Err("script interpreter recursion limit exceeded".into());
    }

    let path = resolve_host_path(cwd.to_string(), argv[0])?;
    let image =
        axfs::api::read(path.as_str()).map_err(|err| format!("failed to read {path}: {err}"))?;

    if let Some(next_argv) = parse_shebang_argv(path.as_str(), &image, argv)? {
        let next_refs = next_argv.iter().map(String::as_str).collect::<Vec<_>>();
        return prepare_program(cwd, &next_refs, depth + 1);
    }

    Ok(PreparedProgram {
        image,
        argv: argv.iter().map(|arg| (*arg).to_string()).collect(),
        path: path.clone(),
        exec_root: derive_exec_root_from_path(path.as_str()),
    })
}

fn parse_shebang_argv(
    script_path: &str,
    image: &[u8],
    argv: &[&str],
) -> Result<Option<Vec<String>>, String> {
    if image.len() < 2 || &image[..2] != b"#!" {
        return Ok(None);
    }

    let line_end = image
        .iter()
        .position(|&byte| byte == b'\n')
        .unwrap_or(image.len());
    let line = core::str::from_utf8(&image[2..line_end])
        .map_err(|_| format!("invalid shebang in {script_path}"))?
        .trim_end_matches('\r')
        .trim();
    if line.is_empty() {
        return Err(format!("empty shebang interpreter in {script_path}"));
    }

    let mut parts = line.split_whitespace();
    let raw_interpreter = parts.next().unwrap();
    let mut next_argv = resolve_script_interpreter(script_path, raw_interpreter)?;
    next_argv.extend(parts.map(str::to_string));
    next_argv.push(script_path.to_string());
    next_argv.extend(argv.iter().skip(1).map(|arg| (*arg).to_string()));
    Ok(Some(next_argv))
}

fn resolve_script_interpreter(
    script_path: &str,
    raw_interpreter: &str,
) -> Result<Vec<String>, String> {
    let base = script_dir(script_path);
    let resolved = resolve_host_path(base, raw_interpreter)?;
    if matches!(axfs::api::metadata(&resolved), Ok(meta) if meta.is_file()) {
        return Ok(vec![resolved]);
    }

    if raw_interpreter == "/bin/sh" || raw_interpreter == "/busybox" {
        if let Some(busybox) = find_busybox_for_script(script_path) {
            return Ok(vec![busybox, "sh".into()]);
        }
    } else if raw_interpreter == "/bin/busybox" {
        if let Some(busybox) = find_busybox_for_script(script_path) {
            return Ok(vec![busybox]);
        }
    }

    Err(format!("script interpreter not found: {raw_interpreter}"))
}

fn find_busybox_for_script(script_path: &str) -> Option<String> {
    let mut candidates = Vec::new();
    match derive_exec_root_from_path(script_path).as_str() {
        "/musl" => candidates.push("/musl/busybox"),
        "/glibc" => candidates.push("/glibc/busybox"),
        _ => {}
    }
    candidates.push("/musl/busybox");
    candidates.push("/glibc/busybox");

    candidates.into_iter().find_map(|path| {
        matches!(axfs::api::metadata(path), Ok(meta) if meta.is_file()).then(|| path.to_string())
    })
}

fn script_dir(path: &str) -> String {
    match path.rfind('/') {
        Some(0) | None => "/".into(),
        Some(idx) => path[..idx].to_string(),
    }
}

fn analyze_elf(elf: &ElfFile<'_>, preferred_base: usize) -> Result<ElfLoadInfo, String> {
    let elf_type = elf.header.pt2.type_().as_type();
    let expected_machine = if cfg!(target_arch = "riscv64") {
        Machine::RISC_V
    } else {
        Machine::Other(258)
    };
    if elf.header.pt2.machine().as_machine() != expected_machine {
        return Err("ELF machine does not match current architecture".into());
    }
    let mut min_load_addr: Option<usize> = None;
    let mut max_segment_end = 0usize;
    let mut interpreter = None;
    for ph in elf.program_iter() {
        match ph.get_type().map_err(str_err)? {
            PhType::Load => {
                let seg_start = align_down(ph.virtual_addr() as usize, PAGE_SIZE_4K);
                let seg_end = align_up(
                    ph.virtual_addr() as usize + ph.mem_size() as usize,
                    PAGE_SIZE_4K,
                );
                min_load_addr = Some(match min_load_addr {
                    Some(curr) => curr.min(seg_start),
                    None => seg_start,
                });
                max_segment_end = cmp::max(max_segment_end, seg_end);
            }
            PhType::Interp => interpreter = Some(read_interp_path(elf, &ph)?),
            _ => {}
        }
    }
    let Some(min_load_addr) = min_load_addr else {
        return Err("ELF has no LOAD segments".into());
    };

    let (load_bias, base) = match elf_type {
        ElfType::Executable => (0usize, 0usize),
        ElfType::SharedObject => {
            let mapped_min = align_up(cmp::max(preferred_base, min_load_addr), PAGE_SIZE_4K);
            let load_bias = mapped_min
                .checked_sub(min_load_addr)
                .ok_or_else(|| "failed to compute PIE load bias".to_string())?;
            (load_bias, load_bias)
        }
        _ => return Err("unsupported ELF type".into()),
    };

    Ok(ElfLoadInfo {
        load_bias,
        entry: load_bias + elf.header.pt2.entry_point() as usize,
        phdr: phdr_addr(elf, load_bias).unwrap_or(0),
        max_segment_end: load_bias + max_segment_end,
        base,
        interpreter,
    })
}

fn read_interp_path(elf: &ElfFile<'_>, ph: &ProgramHeader<'_>) -> Result<String, String> {
    let offset = ph.offset() as usize;
    let file_size = ph.file_size() as usize;
    let end = offset
        .checked_add(file_size)
        .ok_or_else(|| "PT_INTERP range overflow".to_string())?;
    let image = elf.input;
    if end > image.len() {
        return Err("PT_INTERP exceeds ELF image".into());
    }
    let raw = &image[offset..end];
    let path = raw.split(|byte| *byte == 0).next().unwrap_or(raw);
    let path = core::str::from_utf8(path).map_err(|_| "invalid PT_INTERP path".to_string())?;
    if path.is_empty() {
        return Err("empty PT_INTERP path".into());
    }
    Ok(path.to_string())
}

fn map_elf_image(
    aspace: &mut AddrSpace,
    image: &[u8],
    elf: &ElfFile<'_>,
    info: &ElfLoadInfo,
) -> Result<(), String> {
    for ph in elf.program_iter() {
        if ph.get_type().map_err(str_err)? == PhType::Load {
            map_load_segment(aspace, image, &ph, info.load_bias)?;
        }
    }
    Ok(())
}

fn map_load_segment(
    aspace: &mut AddrSpace,
    image: &[u8],
    ph: &ProgramHeader<'_>,
    load_bias: usize,
) -> Result<(), String> {
    let start = load_bias + ph.virtual_addr() as usize;
    let mem_size = ph.mem_size() as usize;
    if mem_size == 0 {
        return Ok(());
    }
    let seg_start = align_down(start, PAGE_SIZE_4K);
    let seg_end = align_up(start + mem_size, PAGE_SIZE_4K);
    let seg_size = seg_end - seg_start;
    aspace
        .map_alloc(
            VirtAddr::from(seg_start),
            seg_size,
            flags_from_ph(ph.flags()),
            true,
        )
        .map_err(|err| format!("failed to map ELF segment at {seg_start:#x}: {err}"))?;

    let file_size = ph.file_size() as usize;
    if file_size != 0 {
        let offset = ph.offset() as usize;
        let end = offset
            .checked_add(file_size)
            .ok_or_else(|| "ELF segment range overflow".to_string())?;
        if end > image.len() {
            return Err("ELF segment exceeds image size".into());
        }
        let data = &image[offset..offset + file_size];
        aspace
            .write(VirtAddr::from(start), data)
            .map_err(|err| format!("failed to write ELF segment at {start:#x}: {err}"))?;
    }
    Ok(())
}

fn phdr_addr(elf: &ElfFile<'_>, load_bias: usize) -> Option<usize> {
    let phoff = elf.header.pt2.ph_offset() as usize;
    for ph in elf.program_iter() {
        if ph.get_type().ok()? != PhType::Load {
            continue;
        }
        let seg_offset = ph.offset() as usize;
        let seg_end = seg_offset.checked_add(ph.file_size() as usize)?;
        if (seg_offset..seg_end).contains(&phoff) {
            return Some(load_bias + ph.virtual_addr() as usize + (phoff - seg_offset));
        }
    }
    None
}

fn build_initial_stack(
    aspace: &mut AddrSpace,
    stack_base: usize,
    stack_top: usize,
    argv: &[&str],
    execfn: &str,
    entry: usize,
    interp_base: usize,
    phdr: usize,
    phent: usize,
    phnum: usize,
) -> Result<usize, String> {
    let mut sp = stack_top;
    let random_bytes = [0x55u8; 16];
    let random_ptr = push_stack_bytes(aspace, stack_base, &mut sp, &random_bytes, 16)?;
    let mut execfn_bytes = execfn.as_bytes().to_vec();
    execfn_bytes.push(0);
    let execfn_ptr = push_stack_bytes(aspace, stack_base, &mut sp, &execfn_bytes, 1)?;
    let mut platform_bytes = AUX_PLATFORM.as_bytes().to_vec();
    platform_bytes.push(0);
    let platform_ptr = push_stack_bytes(aspace, stack_base, &mut sp, &platform_bytes, 1)?;

    let mut arg_ptrs = Vec::with_capacity(argv.len());
    for arg in argv.iter().rev() {
        let mut bytes = arg.as_bytes().to_vec();
        bytes.push(0);
        let ptr = push_stack_bytes(aspace, stack_base, &mut sp, &bytes, 1)?;
        arg_ptrs.push(ptr);
    }
    arg_ptrs.reverse();

    let aux = [
        AuxEntry {
            key: auxvec::AT_PAGESZ as usize,
            value: PAGE_SIZE_4K,
        },
        AuxEntry {
            key: auxvec::AT_UID as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_EUID as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_GID as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_EGID as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_SECURE as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_FLAGS as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_CLKTCK as usize,
            value: AUX_CLOCK_TICKS,
        },
        AuxEntry {
            key: auxvec::AT_HWCAP as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_HWCAP2 as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_PLATFORM as usize,
            value: platform_ptr,
        },
        AuxEntry {
            key: auxvec::AT_BASE_PLATFORM as usize,
            value: platform_ptr,
        },
        AuxEntry {
            key: auxvec::AT_RANDOM as usize,
            value: random_ptr,
        },
        AuxEntry {
            key: auxvec::AT_PHDR as usize,
            value: phdr,
        },
        AuxEntry {
            key: auxvec::AT_PHENT as usize,
            value: phent,
        },
        AuxEntry {
            key: auxvec::AT_PHNUM as usize,
            value: phnum,
        },
        AuxEntry {
            key: auxvec::AT_BASE as usize,
            value: interp_base,
        },
        AuxEntry {
            key: auxvec::AT_ENTRY as usize,
            value: entry,
        },
        AuxEntry {
            key: auxvec::AT_EXECFN as usize,
            value: execfn_ptr,
        },
        AuxEntry {
            key: auxvec::AT_NULL as usize,
            value: 0,
        },
    ];

    let mut words = Vec::with_capacity(1 + arg_ptrs.len() + 1 + 1 + aux.len() * 2);
    words.push(argv.len());
    words.extend(arg_ptrs.iter().copied());
    words.push(0);
    words.push(0);
    for item in aux {
        words.push(item.key);
        words.push(item.value);
    }
    let bytes = words_to_bytes(&words);
    sp = align_down(sp.saturating_sub(bytes.len()), 16);
    let end = sp + bytes.len();
    if sp < stack_base || end > stack_top {
        return Err("user stack overflow".into());
    }
    fault_user_stack_pages(aspace, sp, bytes.len())?;
    aspace
        .write(VirtAddr::from(sp), &bytes)
        .map_err(|err| format!("failed to populate user stack: {err}"))?;
    Ok(sp)
}

fn push_stack_bytes(
    aspace: &mut AddrSpace,
    stack_base: usize,
    sp: &mut usize,
    data: &[u8],
    align: usize,
) -> Result<usize, String> {
    *sp = align_down(sp.saturating_sub(data.len()), align.max(1));
    if *sp < stack_base {
        return Err("user stack overflow".into());
    }
    fault_user_stack_pages(aspace, *sp, data.len())?;
    aspace
        .write(VirtAddr::from(*sp), data)
        .map_err(|err| format!("failed to write user stack data: {err}"))?;
    Ok(*sp)
}

fn fault_user_stack_pages(aspace: &mut AddrSpace, start: usize, size: usize) -> Result<(), String> {
    if size == 0 {
        return Ok(());
    }
    let end = start
        .checked_add(size)
        .ok_or_else(|| "user stack write range overflow".to_string())?;
    let fault_start = align_down(start, PAGE_SIZE_4K);
    let fault_end = align_up(end, PAGE_SIZE_4K);
    for vaddr in PageIter4K::new(VirtAddr::from(fault_start), VirtAddr::from(fault_end))
        .expect("user stack range must be 4K aligned")
    {
        if aspace.page_table().query(vaddr).is_err()
            && !aspace.handle_page_fault(vaddr, PageFaultFlags::WRITE)
        {
            return Err(format!(
                "failed to fault user stack page at {:#x}",
                vaddr.as_usize()
            ));
        }
    }
    Ok(())
}

fn words_to_bytes(words: &[usize]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(words.len() * size_of::<usize>());
    for word in words {
        bytes.extend_from_slice(&word.to_ne_bytes());
    }
    bytes
}

fn make_uspace_context(entry: usize, stack_ptr: usize, argc: usize) -> UspaceContext {
    #[cfg(target_arch = "riscv64")]
    {
        let mut sstatus = Sstatus::from_bits(0);
        sstatus.set_spie(true);
        sstatus.set_sum(true);
        sstatus.set_fs(FS::Initial);
        let mut tf = TrapFrame {
            regs: axhal::context::TrapFrame::default().regs,
            sepc: entry,
            sstatus,
        };
        tf.regs.sp = stack_ptr;
        // RISC-V glibc crt1 treats entry a0 as rtld_fini, while argc/argv/envp
        // are read from the initial stack. Passing argc here makes static glibc
        // call argc as an exit handler.
        tf.regs.a0 = 0;
        tf.regs.a1 = stack_ptr + size_of::<usize>();
        tf.regs.a2 = stack_ptr + (argc + 2) * size_of::<usize>();
        UspaceContext::from(&tf)
    }
    #[cfg(target_arch = "loongarch64")]
    {
        let mut tf = TrapFrame::default();
        tf.prmd = 0b11 | (1 << 2);
        tf.era = entry;
        tf.regs.sp = stack_ptr;
        // LoongArch glibc has the same crt1 convention: a0 is rtld_fini, not
        // argc. The argument vector starts on the user stack.
        tf.regs.a0 = 0;
        tf.regs.a1 = stack_ptr + size_of::<usize>();
        tf.regs.a2 = stack_ptr + (argc + 2) * size_of::<usize>();
        UspaceContext::from(&tf)
    }
}

fn child_trap_frame(parent: &TrapFrame, child_stack: usize) -> TrapFrame {
    let mut child = *parent;
    child.regs.a0 = 0;
    if child_stack != 0 {
        child.regs.sp = child_stack;
    }
    advance_syscall_pc(&mut child);
    child
}

#[cfg(target_arch = "loongarch64")]
fn clone_tls_ctid_args(tf: &TrapFrame) -> (usize, usize) {
    // LoongArch Linux clone ABI is flags, stack, ptid, ctid, tls.
    (tf.arg4(), tf.arg3())
}

#[cfg(not(target_arch = "loongarch64"))]
fn clone_tls_ctid_args(tf: &TrapFrame) -> (usize, usize) {
    (tf.arg3(), tf.arg4())
}

#[cfg(target_arch = "riscv64")]
fn sign_extend(value: usize, bits: usize) -> isize {
    let shift = usize::BITS as usize - bits;
    ((value << shift) as isize) >> shift
}

#[cfg(target_arch = "riscv64")]
fn riscv_b_type_next_pc(pc: usize, inst: u32, a0: usize) -> Option<usize> {
    if inst & 0x7f != 0x63 {
        return None;
    }

    let funct3 = (inst >> 12) & 0x7;
    let rs1 = (inst >> 15) & 0x1f;
    let rs2 = (inst >> 20) & 0x1f;
    if !((rs1 == 10 && rs2 == 0) || (rs1 == 0 && rs2 == 10)) {
        return None;
    }

    let rs1_value = if rs1 == 10 { a0 } else { 0 };
    let rs2_value = if rs2 == 10 { a0 } else { 0 };
    let taken = match funct3 {
        0x0 => rs1_value == rs2_value,
        0x1 => rs1_value != rs2_value,
        0x4 => (rs1_value as isize) < (rs2_value as isize),
        0x5 => (rs1_value as isize) >= (rs2_value as isize),
        0x6 => rs1_value < rs2_value,
        0x7 => rs1_value >= rs2_value,
        _ => return None,
    };

    let imm = (((inst >> 31) & 0x1) << 12)
        | (((inst >> 7) & 0x1) << 11)
        | (((inst >> 25) & 0x3f) << 5)
        | (((inst >> 8) & 0xf) << 1);
    let target = pc.wrapping_add(sign_extend(imm as usize, 13) as usize);
    Some(if taken { target } else { pc + 4 })
}

#[cfg(target_arch = "riscv64")]
fn riscv_compressed_branch_next_pc(pc: usize, inst: u16, a0: usize) -> Option<usize> {
    if inst & 0x3 != 0x1 {
        return None;
    }

    let funct3 = (inst >> 13) & 0x7;
    if funct3 != 0x6 && funct3 != 0x7 {
        return None;
    }
    let rs1 = 8 + ((inst >> 7) & 0x7);
    if rs1 != 10 {
        return None;
    }

    let taken = match funct3 {
        0x6 => a0 == 0,
        0x7 => a0 != 0,
        _ => unreachable!(),
    };

    let imm = (((inst >> 12) & 0x1) << 8)
        | (((inst >> 10) & 0x3) << 3)
        | (((inst >> 2) & 0x1) << 5)
        | (((inst >> 5) & 0x3) << 6)
        | (((inst >> 3) & 0x3) << 1);
    let target = pc.wrapping_add(sign_extend(imm as usize, 9) as usize);
    Some(if taken { target } else { pc + 2 })
}

#[cfg(target_arch = "riscv64")]
fn riscv_branch_next_pc(process: &UserProcess, pc: usize, a0: usize) -> Option<usize> {
    let Ok(low) = read_user_value::<u16>(process, pc) else {
        return None;
    };

    if low & 0x3 == 0x3 {
        read_user_value::<u32>(process, pc)
            .ok()
            .and_then(|inst| riscv_b_type_next_pc(pc, inst, a0))
    } else {
        riscv_compressed_branch_next_pc(pc, low, a0)
    }
}

#[cfg(target_arch = "riscv64")]
fn fixup_riscv_clone_child_return(process: &UserProcess, tf: &mut TrapFrame) {
    // The child starts from a freshly built UspaceContext instead of the
    // original trap-return path. Interpret the clone wrapper's deterministic
    // a0/zero return dispatch so a0 == 0 reaches the real child-side entry.
    let mut pc = tf.sepc;
    for _ in 0..4 {
        let Some(next_pc) = riscv_branch_next_pc(process, pc, tf.regs.a0) else {
            break;
        };
        if next_pc == pc {
            break;
        }
        pc = next_pc;
    }
    tf.sepc = pc;
}

fn advance_syscall_pc(tf: &mut TrapFrame) {
    #[cfg(target_arch = "riscv64")]
    {
        tf.sepc += 4;
    }
    #[cfg(target_arch = "loongarch64")]
    {
        tf.era += 4;
    }
}

fn exec_program(
    process: &UserProcess,
    cwd: &str,
    argv: &[String],
) -> Result<(usize, usize, usize), String> {
    detach_all_shared_mappings(process);
    let argv_refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
    let image = {
        let mut aspace = process.aspace.lock();
        load_program_image(&mut aspace, cwd, &argv_refs)?
    };
    *process.brk.lock() = image.brk;
    process.set_exec_root(image.exec_root);
    process.set_exec_path(image.exec_path);
    process.signal_actions.lock().clear();
    process.fds.lock().close_on_exec();
    Ok((image.entry, image.stack_ptr, image.argc))
}

impl UserProcess {
    fn cwd(&self) -> String {
        self.cwd.lock().clone()
    }

    fn exec_root(&self) -> String {
        self.exec_root.lock().clone()
    }

    fn exec_path(&self) -> String {
        self.exec_path.lock().clone()
    }

    fn set_cwd(&self, cwd: String) {
        *self.cwd.lock() = cwd;
    }

    fn set_exec_root(&self, exec_root: String) {
        *self.exec_root.lock() = exec_root;
    }

    fn set_exec_path(&self, exec_path: String) {
        *self.exec_path.lock() = exec_path;
    }

    fn teardown(&self) {
        detach_all_shared_mappings(self);
        self.aspace.lock().clear();
        *self.fds.lock() = FdTable::new();
    }

    fn ppid(&self) -> i32 {
        self.ppid
    }

    fn pid(&self) -> i32 {
        self.pid.load(Ordering::Acquire)
    }

    fn set_pid(&self, pid: i32) {
        self.pid.store(pid, Ordering::Release);
    }

    fn add_thread(&self) {
        self.live_threads.fetch_add(1, Ordering::AcqRel);
    }

    fn note_thread_exit(&self, code: i32) {
        self.exit_code.store(code, Ordering::Release);
        if self.live_threads.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.teardown();
            self.exit_wait.notify_all(false);
        }
    }

    fn request_exit_group(&self, code: i32) {
        let _ = self.exit_group_code.compare_exchange(
            NO_EXIT_GROUP_CODE,
            code,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
        self.exit_code.store(code, Ordering::Release);
    }

    fn pending_exit_group(&self) -> Option<i32> {
        let code = self.exit_group_code.load(Ordering::Acquire);
        (code != NO_EXIT_GROUP_CODE).then_some(code)
    }

    fn wait_for_exit(&self) -> i32 {
        self.exit_wait
            .wait_until(|| self.live_threads.load(Ordering::Acquire) == 0);
        self.exit_code.load(Ordering::Acquire)
    }

    fn get_rlimit(&self, resource: u32) -> UserRlimit {
        self.rlimits
            .lock()
            .get(&resource)
            .copied()
            .unwrap_or_else(|| default_rlimit(resource))
    }

    fn set_rlimit(&self, resource: u32, limit: UserRlimit) {
        self.rlimits.lock().insert(resource, limit);
    }

    fn nofile_limit(&self) -> usize {
        let limit = self.get_rlimit(RLIMIT_NOFILE_RESOURCE).rlim_cur;
        cmp::min(limit, DEFAULT_NOFILE_LIMIT) as usize
    }

    fn fork(&self) -> Result<Arc<UserProcess>, LinuxError> {
        let mut aspace = axmm::new_user_aspace(VirtAddr::from(USER_ASPACE_BASE), USER_ASPACE_SIZE)
            .map_err(LinuxError::from)?;
        {
            let parent_aspace = self.aspace.lock();
            aspace
                .clone_user_mappings_from(&parent_aspace)
                .map_err(LinuxError::from)?;
        }
        let shared_attaches = self.shared_attaches.lock().clone();
        if !shared_attaches.is_empty() {
            let mut state = shared_mem_state().lock();
            for attach in shared_attaches.values() {
                if let Some(segment) = state.segments.get_mut(&attach.shmid) {
                    segment.nattch += 1;
                }
            }
        }

        Ok(Arc::new(UserProcess {
            aspace: Mutex::new(aspace),
            brk: Mutex::new(*self.brk.lock()),
            fds: Mutex::new(self.fds.lock().fork_copy()?),
            shared_attaches: Mutex::new(shared_attaches),
            cwd: Mutex::new(self.cwd()),
            exec_root: Mutex::new(self.exec_root()),
            exec_path: Mutex::new(self.exec_path()),
            children: Mutex::new(Vec::new()),
            rlimits: Mutex::new(self.rlimits.lock().clone()),
            signal_actions: Mutex::new(self.signal_actions.lock().clone()),
            pid: AtomicI32::new(0),
            ppid: axtask::current().id().as_u64() as i32,
            live_threads: AtomicUsize::new(1),
            exit_group_code: AtomicI32::new(NO_EXIT_GROUP_CODE),
            exit_code: AtomicI32::new(0),
            exit_wait: WaitQueue::new(),
        }))
    }

    fn add_child(&self, task: AxTaskRef, process: Arc<UserProcess>) -> i32 {
        let pid = task.id().as_u64() as i32;
        self.children.lock().push(ChildTask { pid, task, process });
        pid
    }

    fn wait_child(&self, pid: i32, nohang: bool) -> Result<Option<(i32, i32)>, LinuxError> {
        fn is_exited(child: &ChildTask) -> bool {
            child.process.live_threads.load(Ordering::Acquire) == 0
        }
        fn reap_child(child: ChildTask) -> Result<(i32, i32), LinuxError> {
            let status = if child.process.live_threads.load(Ordering::Acquire) == 0 {
                child.process.exit_code.load(Ordering::Acquire)
            } else {
                child.task.join().ok_or(LinuxError::ECHILD)?
            };
            let child_pid = child.pid;
            child.process.teardown();
            drop(child);
            axtask::yield_now();
            Ok((child_pid, status))
        }

        if nohang {
            let mut children = self.children.lock();
            if children.is_empty() {
                return Err(LinuxError::ECHILD);
            }
            let exited_index = match pid {
                -1 => children.iter().position(is_exited),
                p if p > 0 => {
                    let index = children
                        .iter()
                        .position(|child| child.pid == p)
                        .ok_or(LinuxError::ECHILD)?;
                    is_exited(&children[index]).then_some(index)
                }
                _ => return Err(LinuxError::EINVAL),
            };
            let Some(index) = exited_index else {
                return Ok(None);
            };
            return reap_child(children.remove(index)).map(Some);
        }

        let child = loop {
            let maybe_child = {
                let mut children = self.children.lock();
                if children.is_empty() {
                    return Err(LinuxError::ECHILD);
                }

                let exited_index = match pid {
                    -1 => children.iter().position(is_exited),
                    p if p > 0 => {
                        let index = children
                            .iter()
                            .position(|child| child.pid == p)
                            .ok_or(LinuxError::ECHILD)?;
                        is_exited(&children[index]).then_some(index)
                    }
                    _ => return Err(LinuxError::EINVAL),
                };

                exited_index.map(|index| children.remove(index))
            };

            if let Some(child) = maybe_child {
                break child;
            }
            if nohang {
                return Ok(None);
            }
            if let Some(ext) = current_task_ext() {
                if let Some(code) = ext.process.pending_exit_group() {
                    terminate_current_thread(ext.process.as_ref(), code);
                }
            }
            if current_unblocked_pending_signal().is_some() {
                return Err(LinuxError::EINTR);
            }
            axtask::sleep(core::time::Duration::from_millis(10));
        };
        reap_child(child).map(Some)
    }
}

fn current_process() -> Option<Arc<UserProcess>> {
    let ext = current_task_ext()?;
    Some(ext.process.clone())
}

fn current_task_ext() -> Option<&'static UserTaskExt> {
    let curr = axtask::current_may_uninit()?;
    let ptr = unsafe { curr.task_ext_ptr() };
    if ptr.is_null() {
        return None;
    }
    let ext = unsafe { &*(ptr as *const UserTaskExt) };
    Some(ext)
}

fn task_ext(task: &AxTaskRef) -> Option<&UserTaskExt> {
    let ptr = unsafe { task.task_ext_ptr() };
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { &*(ptr as *const UserTaskExt) })
}

fn futex_table() -> &'static Mutex<BTreeMap<FutexKey, Arc<FutexState>>> {
    static FUTEXES: LazyInit<Mutex<BTreeMap<FutexKey, Arc<FutexState>>>> = LazyInit::new();
    FUTEXES.call_once(|| Mutex::new(BTreeMap::new()));
    &FUTEXES
}

fn user_thread_table() -> &'static Mutex<BTreeMap<i32, UserThreadEntry>> {
    static USER_THREADS: LazyInit<Mutex<BTreeMap<i32, UserThreadEntry>>> = LazyInit::new();
    USER_THREADS.call_once(|| Mutex::new(BTreeMap::new()));
    &USER_THREADS
}

fn shared_mem_state() -> &'static Mutex<SharedMemState> {
    static SHARED_MEM: LazyInit<Mutex<SharedMemState>> = LazyInit::new();
    SHARED_MEM.call_once(|| Mutex::new(SharedMemState::new()));
    &SHARED_MEM
}

fn ipc_now_secs() -> i64 {
    axhal::time::wall_time().as_secs() as i64
}

fn alloc_shared_pages(size: usize) -> Result<(usize, usize), LinuxError> {
    let num_pages = align_up(size.max(1), PAGE_SIZE_4K) / PAGE_SIZE_4K;
    let start_vaddr = global_allocator()
        .alloc_pages(num_pages, PAGE_SIZE_4K)
        .map_err(|_| LinuxError::ENOMEM)?;
    unsafe {
        ptr::write_bytes(start_vaddr as *mut u8, 0, num_pages * PAGE_SIZE_4K);
    }
    Ok((start_vaddr, num_pages))
}

fn free_shared_pages(start_vaddr: usize, num_pages: usize) {
    global_allocator().dealloc_pages(start_vaddr, num_pages);
}

fn shared_segment_to_ds(segment: &SharedMemSegment) -> SysvShmidDs {
    SysvShmidDs {
        shm_perm: SysvIpcPerm {
            __ipc_perm_key: segment.key,
            uid: 0,
            gid: 0,
            cuid: 0,
            cgid: 0,
            mode: segment.mode
                | if segment.marked_destroy {
                    SHM_DEST_FLAG
                } else {
                    0
                },
            __ipc_perm_seq: segment.seq,
            __pad1: 0,
            __pad2: 0,
        },
        shm_segsz: segment.size,
        shm_atime: segment.atime as c_long,
        shm_dtime: segment.dtime as c_long,
        shm_ctime: segment.ctime as c_long,
        shm_cpid: segment.cpid,
        shm_lpid: segment.lpid,
        shm_nattch: segment.nattch,
        __pad1: 0,
        __pad2: 0,
    }
}

fn collect_destroyed_shared_segment(
    state: &mut SharedMemState,
    shmid: i32,
) -> Option<(usize, usize)> {
    let segment = state.segments.get(&shmid)?;
    if !segment.marked_destroy || segment.nattch != 0 {
        return None;
    }
    let segment = state.segments.remove(&shmid)?;
    state.key_map.remove(&segment.key);
    Some((segment.start_vaddr, segment.num_pages))
}

fn choose_shmat_addr(
    process: &UserProcess,
    shmaddr: usize,
    flags: i32,
    size: usize,
) -> Result<usize, LinuxError> {
    if shmaddr != 0 {
        let addr = if flags & SHM_RND_FLAG != 0 {
            align_down(shmaddr, PAGE_SIZE_4K)
        } else if shmaddr % PAGE_SIZE_4K == 0 {
            shmaddr
        } else {
            return Err(LinuxError::EINVAL);
        };
        if addr < USER_MMAP_BASE || addr + size >= USER_STACK_TOP - USER_STACK_SIZE {
            return Err(LinuxError::EINVAL);
        }
        return Ok(addr);
    }

    let mut brk = process.brk.lock();
    let start = align_up(brk.next_mmap, PAGE_SIZE_4K);
    if start < USER_MMAP_BASE || start + size >= USER_STACK_TOP - USER_STACK_SIZE {
        return Err(LinuxError::ENOMEM);
    }
    brk.next_mmap = start + size + PAGE_SIZE_4K;
    Ok(start)
}

fn register_shmat_mapping(
    process: &UserProcess,
    shmid: i32,
    shmaddr: usize,
    shmflg: i32,
) -> Result<usize, LinuxError> {
    let readonly = shmflg & SHM_RDONLY_FLAG != 0;
    let remap = shmflg & SHM_REMAP_FLAG != 0;
    let size = {
        let state = shared_mem_state().lock();
        state
            .segments
            .get(&shmid)
            .map(|segment| segment.map_size)
            .ok_or(LinuxError::EINVAL)?
    };
    let target = choose_shmat_addr(process, shmaddr, shmflg, size)?;
    let (size, start_paddr) = {
        let mut state = shared_mem_state().lock();
        let segment = state.segments.get_mut(&shmid).ok_or(LinuxError::EINVAL)?;
        let size = segment.map_size;
        let start_paddr = virt_to_phys(VirtAddr::from(segment.start_vaddr));
        segment.nattch += 1;
        segment.lpid = process.pid();
        segment.atime = ipc_now_secs();
        (size, start_paddr)
    };
    let map_flags = user_mapping_flags(true, !readonly, false);
    let map_result = {
        let mut aspace = process.aspace.lock();
        if remap {
            let _ = aspace.unmap(VirtAddr::from(target), size);
        }
        aspace.map_linear(VirtAddr::from(target), start_paddr, size, map_flags)
    };
    if let Err(err) = map_result {
        let free_pages = {
            let mut state = shared_mem_state().lock();
            if let Some(segment) = state.segments.get_mut(&shmid) {
                segment.nattch = segment.nattch.saturating_sub(1);
                collect_destroyed_shared_segment(&mut state, shmid)
            } else {
                None
            }
        };
        if let Some((start_vaddr, num_pages)) = free_pages {
            free_shared_pages(start_vaddr, num_pages);
        }
        return Err(LinuxError::from(err));
    }
    process
        .shared_attaches
        .lock()
        .insert(target, SharedMemAttach { shmid, size });
    Ok(target)
}

fn detach_shmat_mapping(process: &UserProcess, addr: usize) -> Result<(), LinuxError> {
    let attach = process
        .shared_attaches
        .lock()
        .get(&addr)
        .copied()
        .ok_or(LinuxError::EINVAL)?;
    process
        .aspace
        .lock()
        .unmap(VirtAddr::from(addr), attach.size)
        .map_err(LinuxError::from)?;
    process.shared_attaches.lock().remove(&addr);

    let free_pages = {
        let mut state = shared_mem_state().lock();
        let segment = state
            .segments
            .get_mut(&attach.shmid)
            .ok_or(LinuxError::EINVAL)?;
        segment.nattch = segment.nattch.saturating_sub(1);
        segment.lpid = process.pid();
        segment.dtime = ipc_now_secs();
        collect_destroyed_shared_segment(&mut state, attach.shmid)
    };
    if let Some((start_vaddr, num_pages)) = free_pages {
        free_shared_pages(start_vaddr, num_pages);
    }
    Ok(())
}

fn detach_all_shared_mappings(process: &UserProcess) {
    let addrs = process
        .shared_attaches
        .lock()
        .keys()
        .copied()
        .collect::<Vec<_>>();
    for addr in addrs {
        let _ = detach_shmat_mapping(process, addr);
    }
}

fn register_user_task(task: AxTaskRef, process: Arc<UserProcess>) {
    let tid = task.id().as_u64() as i32;
    user_thread_table()
        .lock()
        .insert(tid, UserThreadEntry { task, process });
}

fn unregister_user_task(tid: i32) {
    user_thread_table().lock().remove(&tid);
}

fn user_thread_entry_by_tid(tid: i32) -> Option<UserThreadEntry> {
    user_thread_table().lock().get(&tid).cloned()
}

fn user_thread_entries_by_pid(pid: i32) -> Vec<UserThreadEntry> {
    user_thread_table()
        .lock()
        .values()
        .filter(|entry| entry.process.pid() == pid)
        .cloned()
        .collect()
}

fn deliver_user_signal(entry: &UserThreadEntry, sig: i32) -> Result<(), LinuxError> {
    if sig == 0 {
        return Ok(());
    }
    let ext = task_ext(&entry.task).ok_or(LinuxError::ESRCH)?;
    ext.pending_signal.store(sig, Ordering::Release);
    if sig == SIGCANCEL_NUM {
        ext.sigcancel_delivery_armed.store(false, Ordering::Release);
    }
    if sig == SIGCANCEL_NUM && !signal_is_blocked(ext, sig) {
        let futex_wait = ext.futex_wait.load(Ordering::Acquire);
        if futex_wait != 0 {
            for state in futex_states_for_addr(futex_wait) {
                state.seq.fetch_add(1, Ordering::Release);
                let _ = state.queue.notify_task(true, &entry.task);
            }
        }
    }
    Ok(())
}

fn futex_state(uaddr: usize, bitset: u32) -> Arc<FutexState> {
    let mut table = futex_table().lock();
    table
        .entry(FutexKey { uaddr, bitset })
        .or_insert_with(|| {
            Arc::new(FutexState {
                seq: Arc::new(AtomicU32::new(0)),
                queue: WaitQueue::new(),
            })
        })
        .clone()
}

fn futex_states_for_addr(uaddr: usize) -> Vec<Arc<FutexState>> {
    futex_table()
        .lock()
        .iter()
        .filter(|(key, _)| key.uaddr == uaddr)
        .map(|(_, state)| state.clone())
        .collect()
}

fn futex_wake_addr(uaddr: usize, count: usize, wake_bitset: u32) -> usize {
    let states = futex_table()
        .lock()
        .iter()
        .filter(|(key, _)| key.uaddr == uaddr && key.bitset & wake_bitset != 0)
        .map(|(_, state)| state.clone())
        .collect::<Vec<_>>();
    let mut woken = 0usize;
    for state in states {
        state.seq.fetch_add(1, Ordering::Release);
        while woken < count {
            if !state.queue.notify_one(true) {
                break;
            }
            woken += 1;
        }
        if woken >= count {
            break;
        }
    }
    woken
}

fn clear_current_futex_wait() {
    let Some(ext) = current_task_ext() else {
        return;
    };
    let wait_addr = ext.futex_wait.swap(0, Ordering::AcqRel);
    if wait_addr == 0 {
        return;
    }
    if let Some(state) = futex_table().lock().get(&wait_addr).cloned() {
        let curr = axtask::current();
        state.queue.remove_task(curr.as_task_ref());
    }
}

fn futex_wake_op(
    process: &UserProcess,
    uaddr: usize,
    wake_count: usize,
    uaddr2: usize,
    wake_count2: usize,
    encoded_op: u32,
) -> Result<usize, LinuxError> {
    if uaddr2 == 0 {
        return Err(LinuxError::EFAULT);
    }
    let old = read_user_value::<u32>(process, uaddr2)?;
    let mut op = (encoded_op >> 28) & 0xf;
    let cmp = (encoded_op >> 24) & 0xf;
    let mut oparg = (encoded_op >> 12) & 0xfff;
    let cmparg = encoded_op & 0xfff;
    if op & 8 != 0 {
        op &= 7;
        oparg = 1u32.checked_shl(oparg).unwrap_or(0);
    }
    let new = match op {
        0 => oparg,
        1 => old.wrapping_add(oparg),
        2 => old | oparg,
        3 => old & !oparg,
        4 => old ^ oparg,
        _ => return Err(LinuxError::ENOSYS),
    };
    let ret = write_user_value(process, uaddr2, &new);
    if ret != 0 {
        return Err(LinuxError::EFAULT);
    }

    let old_signed = old as i32;
    let cmp_signed = cmparg as i32;
    let wake_second = match cmp {
        0 => old_signed == cmp_signed,
        1 => old_signed != cmp_signed,
        2 => old_signed < cmp_signed,
        3 => old_signed <= cmp_signed,
        4 => old_signed > cmp_signed,
        5 => old_signed >= cmp_signed,
        _ => return Err(LinuxError::ENOSYS),
    };

    let mut woken = futex_wake_addr(uaddr, wake_count);
    if wake_second {
        woken += futex_wake_addr(uaddr2, wake_count2);
    }
    Ok(woken)
}

fn clear_current_tid_and_wake() {
    let Some(ext) = current_task_ext() else {
        return;
    };
    let clear_tid = ext.clear_child_tid.swap(0, Ordering::AcqRel);
    if clear_tid == 0 {
        return;
    }
    user_trace!(
        "user-clear-tid: tid={} clear_tid={clear_tid:#x}",
        current_tid()
    );
    let zero: i32 = 0;
    let _ = write_user_value(ext.process.as_ref(), clear_tid, &zero);
    let _ = futex_wake_addr(clear_tid, 1, u32::MAX);
}

fn perform_deferred_self_unmap() {
    let Some(ext) = current_task_ext() else {
        return;
    };
    let start = ext.deferred_unmap_start.swap(0, Ordering::AcqRel);
    let len = ext.deferred_unmap_len.swap(0, Ordering::AcqRel);
    if start == 0 || len == 0 {
        return;
    }
    let _ = ext.process.aspace.lock().unmap(VirtAddr::from(start), len);
}

fn current_tid() -> i32 {
    axtask::current().id().as_u64() as i32
}

fn signal_mask_bit(sig: i32) -> u64 {
    if (1..=64).contains(&sig) {
        1u64 << ((sig - 1) as u32)
    } else {
        0
    }
}

fn signal_is_blocked(ext: &UserTaskExt, sig: i32) -> bool {
    let bit = signal_mask_bit(sig);
    bit != 0 && ext.signal_mask.load(Ordering::Acquire) & bit != 0
}

#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
fn musl_pthread_self(tp: usize) -> usize {
    tp.saturating_sub(200)
}

#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
fn musl_cancel_pending(process: &UserProcess, tp: usize) -> bool {
    let self_ptr = musl_pthread_self(tp);
    matches!(
        read_user_value::<u32>(process, self_ptr + 44),
        Ok(cancel) if cancel != 0
    )
}

#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
fn musl_cancel_disabled(process: &UserProcess, tp: usize) -> bool {
    let self_ptr = musl_pthread_self(tp);
    matches!(
        read_user_value::<u8>(process, self_ptr + 48),
        Ok(state) if state == 1
    )
}

#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
fn musl_cancel_async(process: &UserProcess, tp: usize) -> bool {
    let self_ptr = musl_pthread_self(tp);
    matches!(
        read_user_value::<u8>(process, self_ptr + 49),
        Ok(cancel_type) if cancel_type != 0
    )
}

#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
fn sigcancel_pending_for(ext: &UserTaskExt, process: &UserProcess, tp: usize) -> bool {
    ext.pending_signal.load(Ordering::Acquire) == SIGCANCEL_NUM
        && !signal_is_blocked(ext, SIGCANCEL_NUM)
        && !musl_cancel_disabled(process, tp)
}

#[cfg(not(any(target_arch = "riscv64", target_arch = "loongarch64")))]
fn sigcancel_pending_for(_ext: &UserTaskExt, _process: &UserProcess, _tp: usize) -> bool {
    false
}

fn current_sigcancel_pending(process: &UserProcess, tp: usize) -> bool {
    current_task_ext().is_some_and(|ext| sigcancel_pending_for(ext, process, tp))
}

fn arm_current_sigcancel_delivery() {
    if let Some(ext) = current_task_ext() {
        ext.sigcancel_delivery_armed.store(true, Ordering::Release);
    }
}

#[cfg(target_arch = "riscv64")]
fn syscall_instruction_context(tf: &TrapFrame) -> TrapFrame {
    let mut saved = *tf;
    saved.sepc = saved.sepc.saturating_sub(4);
    saved
}

#[cfg(target_arch = "loongarch64")]
fn syscall_instruction_context(tf: &TrapFrame) -> TrapFrame {
    let mut saved = *tf;
    saved.era = saved.era.saturating_sub(4);
    saved
}

fn ensure_user_return_hook_registered() {
    if !USER_RETURN_HOOK_REGISTERED.swap(true, Ordering::AcqRel) {
        register_user_return_handler(user_return_hook);
    }
}

fn user_return_hook(tf: &mut TrapFrame) {
    let Some(ext) = current_task_ext() else {
        return;
    };
    if ext.signal_frame.load(Ordering::Acquire) == 0 {
        if let Some(restored) = ext.pending_sigreturn.lock().take() {
            *tf = restored;
            return;
        }
    }
    #[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
    if ext.signal_frame.load(Ordering::Acquire) == 0 {
        let sig = ext.pending_signal.load(Ordering::Acquire);
        if sig != 0 && !signal_is_blocked(ext, sig) {
            let mut from_cancel_point = false;
            if sig == SIGCANCEL_NUM {
                let disabled = musl_cancel_disabled(ext.process.as_ref(), tf.regs.tp);
                let async_cancel = musl_cancel_async(ext.process.as_ref(), tf.regs.tp);
                let armed = ext.sigcancel_delivery_armed.swap(false, Ordering::AcqRel);
                if disabled {
                    return;
                }
                if !armed && !async_cancel {
                    return;
                }
                from_cancel_point = armed;
            }
            let _ = inject_pending_signal(tf, ext, sig, from_cancel_point);
        }
    }
}

#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
fn user_pc(tf: &TrapFrame) -> usize {
    tf.sepc
}

#[cfg(target_arch = "loongarch64")]
#[allow(dead_code)]
fn user_pc(tf: &TrapFrame) -> usize {
    tf.era
}

fn terminate_current_thread(process: &UserProcess, code: i32) -> ! {
    clear_current_tid_and_wake();
    perform_deferred_self_unmap();
    unregister_user_task(current_tid());
    process.note_thread_exit(code);
    axtask::exit(code)
}

#[cfg(target_arch = "riscv64")]
fn trap_frame_to_riscv_sigcontext(tf: &TrapFrame) -> RiscvSignalSigcontext {
    RiscvSignalSigcontext {
        gregs: [
            tf.sepc,
            tf.regs.ra,
            tf.regs.sp,
            tf.regs.gp,
            tf.regs.tp,
            tf.regs.t0,
            tf.regs.t1,
            tf.regs.t2,
            tf.regs.s0,
            tf.regs.s1,
            tf.regs.a0,
            tf.regs.a1,
            tf.regs.a2,
            tf.regs.a3,
            tf.regs.a4,
            tf.regs.a5,
            tf.regs.a6,
            tf.regs.a7,
            tf.regs.s2,
            tf.regs.s3,
            tf.regs.s4,
            tf.regs.s5,
            tf.regs.s6,
            tf.regs.s7,
            tf.regs.s8,
            tf.regs.s9,
            tf.regs.s10,
            tf.regs.s11,
            tf.regs.t3,
            tf.regs.t4,
            tf.regs.t5,
            tf.regs.t6,
        ],
        fpstate: RiscvSignalFpState {
            bytes: [0; RISCV_SIGNAL_FPSTATE_BYTES],
        },
    }
}

#[cfg(target_arch = "riscv64")]
fn apply_riscv_sigcontext(tf: &mut TrapFrame, sigcontext: &RiscvSignalSigcontext) {
    tf.sepc = sigcontext.gregs[0];
    tf.regs.zero = 0;
    tf.regs.ra = sigcontext.gregs[1];
    tf.regs.sp = sigcontext.gregs[2];
    tf.regs.gp = sigcontext.gregs[3];
    tf.regs.tp = sigcontext.gregs[4];
    tf.regs.t0 = sigcontext.gregs[5];
    tf.regs.t1 = sigcontext.gregs[6];
    tf.regs.t2 = sigcontext.gregs[7];
    tf.regs.s0 = sigcontext.gregs[8];
    tf.regs.s1 = sigcontext.gregs[9];
    tf.regs.a0 = sigcontext.gregs[10];
    tf.regs.a1 = sigcontext.gregs[11];
    tf.regs.a2 = sigcontext.gregs[12];
    tf.regs.a3 = sigcontext.gregs[13];
    tf.regs.a4 = sigcontext.gregs[14];
    tf.regs.a5 = sigcontext.gregs[15];
    tf.regs.a6 = sigcontext.gregs[16];
    tf.regs.a7 = sigcontext.gregs[17];
    tf.regs.s2 = sigcontext.gregs[18];
    tf.regs.s3 = sigcontext.gregs[19];
    tf.regs.s4 = sigcontext.gregs[20];
    tf.regs.s5 = sigcontext.gregs[21];
    tf.regs.s6 = sigcontext.gregs[22];
    tf.regs.s7 = sigcontext.gregs[23];
    tf.regs.s8 = sigcontext.gregs[24];
    tf.regs.s9 = sigcontext.gregs[25];
    tf.regs.s10 = sigcontext.gregs[26];
    tf.regs.s11 = sigcontext.gregs[27];
    tf.regs.t3 = sigcontext.gregs[28];
    tf.regs.t4 = sigcontext.gregs[29];
    tf.regs.t5 = sigcontext.gregs[30];
    tf.regs.t6 = sigcontext.gregs[31];
}

fn make_signal_info(sig: i32, code: i32, tid: i32) -> SignalInfo {
    let mut info = SignalInfo { bytes: [0; 128] };
    info.bytes[0..4].copy_from_slice(&sig.to_ne_bytes());
    info.bytes[4..8].copy_from_slice(&0i32.to_ne_bytes());
    info.bytes[8..12].copy_from_slice(&code.to_ne_bytes());
    info.bytes[16..20].copy_from_slice(&tid.to_ne_bytes());
    info.bytes[20..24].copy_from_slice(&0u32.to_ne_bytes());
    info
}

#[cfg(target_arch = "loongarch64")]
fn trap_frame_to_loongarch_mcontext(tf: &TrapFrame) -> LoongarchSignalMcontext {
    LoongarchSignalMcontext {
        pc: tf.era,
        regs: [
            tf.regs.zero,
            tf.regs.ra,
            tf.regs.tp,
            tf.regs.sp,
            tf.regs.a0,
            tf.regs.a1,
            tf.regs.a2,
            tf.regs.a3,
            tf.regs.a4,
            tf.regs.a5,
            tf.regs.a6,
            tf.regs.a7,
            tf.regs.t0,
            tf.regs.t1,
            tf.regs.t2,
            tf.regs.t3,
            tf.regs.t4,
            tf.regs.t5,
            tf.regs.t6,
            tf.regs.t7,
            tf.regs.t8,
            tf.regs.u0,
            tf.regs.fp,
            tf.regs.s0,
            tf.regs.s1,
            tf.regs.s2,
            tf.regs.s3,
            tf.regs.s4,
            tf.regs.s5,
            tf.regs.s6,
            tf.regs.s7,
            tf.regs.s8,
        ],
        flags: 0,
        pad: 0,
    }
}

#[cfg(target_arch = "loongarch64")]
fn apply_loongarch_mcontext(tf: &mut TrapFrame, mcontext: &LoongarchSignalMcontext) {
    tf.era = mcontext.pc;
    tf.regs.zero = 0;
    tf.regs.ra = mcontext.regs[1];
    tf.regs.tp = mcontext.regs[2];
    tf.regs.sp = mcontext.regs[3];
    tf.regs.a0 = mcontext.regs[4];
    tf.regs.a1 = mcontext.regs[5];
    tf.regs.a2 = mcontext.regs[6];
    tf.regs.a3 = mcontext.regs[7];
    tf.regs.a4 = mcontext.regs[8];
    tf.regs.a5 = mcontext.regs[9];
    tf.regs.a6 = mcontext.regs[10];
    tf.regs.a7 = mcontext.regs[11];
    tf.regs.t0 = mcontext.regs[12];
    tf.regs.t1 = mcontext.regs[13];
    tf.regs.t2 = mcontext.regs[14];
    tf.regs.t3 = mcontext.regs[15];
    tf.regs.t4 = mcontext.regs[16];
    tf.regs.t5 = mcontext.regs[17];
    tf.regs.t6 = mcontext.regs[18];
    tf.regs.t7 = mcontext.regs[19];
    tf.regs.t8 = mcontext.regs[20];
    tf.regs.u0 = mcontext.regs[21];
    tf.regs.fp = mcontext.regs[22];
    tf.regs.s0 = mcontext.regs[23];
    tf.regs.s1 = mcontext.regs[24];
    tf.regs.s2 = mcontext.regs[25];
    tf.regs.s3 = mcontext.regs[26];
    tf.regs.s4 = mcontext.regs[27];
    tf.regs.s5 = mcontext.regs[28];
    tf.regs.s6 = mcontext.regs[29];
    tf.regs.s7 = mcontext.regs[30];
    tf.regs.s8 = mcontext.regs[31];
}

#[cfg(target_arch = "loongarch64")]
fn make_loongarch_siginfo(sig: i32, code: i32, tid: i32) -> LoongarchSignalInfo {
    let mut info = LoongarchSignalInfo { bytes: [0; 128] };
    info.bytes[0..4].copy_from_slice(&sig.to_ne_bytes());
    info.bytes[4..8].copy_from_slice(&0i32.to_ne_bytes());
    info.bytes[8..12].copy_from_slice(&code.to_ne_bytes());
    info.bytes[16..20].copy_from_slice(&tid.to_ne_bytes());
    info.bytes[20..24].copy_from_slice(&0u32.to_ne_bytes());
    info
}

fn ensure_signal_frame_pages(
    process: &UserProcess,
    start: usize,
    len: usize,
) -> Result<(), LinuxError> {
    let end = start.checked_add(len).ok_or(LinuxError::EFAULT)?;
    let page_start = align_down(start, PAGE_SIZE_4K);
    let page_end = align_up(end, PAGE_SIZE_4K);
    let mut aspace = process.aspace.lock();
    for page in (page_start..page_end).step_by(PAGE_SIZE_4K) {
        let page_va = VirtAddr::from(page);
        if aspace.page_table().query(page_va).is_err() {
            let _ = aspace.handle_page_fault(page_va, PageFaultFlags::WRITE);
        }
    }
    aspace
        .protect(
            VirtAddr::from(page_start),
            page_end - page_start,
            user_mapping_flags(true, true, true),
        )
        .map_err(LinuxError::from)
}

#[cfg(target_arch = "riscv64")]
fn inject_pending_signal(
    tf: &mut TrapFrame,
    ext: &UserTaskExt,
    sig: i32,
    from_cancel_point: bool,
) -> Result<(), LinuxError> {
    let action = ext
        .process
        .signal_actions
        .lock()
        .get(&(sig as usize))
        .copied()
        .unwrap_or_else(|| unsafe { core::mem::zeroed() });
    let handler = action
        .sa_handler_kernel
        .map(|func| func as usize)
        .unwrap_or(0);
    if handler <= 1 {
        ext.pending_signal.store(0, Ordering::Release);
        return Ok(());
    }
    let current_mask = ext.signal_mask.load(Ordering::Acquire);
    let frame_size = size_of::<RiscvSignalFrame>();
    let frame_addr = align_down(tf.regs.sp.saturating_sub(frame_size), 16);
    ensure_signal_frame_pages(ext.process.as_ref(), frame_addr, frame_size)?;

    let saved_tf = if sig == SIGCANCEL_NUM && from_cancel_point {
        syscall_instruction_context(tf)
    } else {
        *tf
    };
    let frame = RiscvSignalFrame {
        info: make_signal_info(sig, SI_TKILL_CODE, current_tid()),
        ucontext: RiscvSignalUcontext {
            flags: 0,
            link: 0,
            stack: RiscvSignalStack {
                sp: 0,
                stack_flags: SS_DISABLE,
                stack_pad: 0,
                size: 0,
            },
            sigmask: RiscvKernelSigset {
                sig: [current_mask],
                reserved: [0; RISCV_SIGNAL_SIGSET_RESERVED_BYTES],
            },
            mcontext: trap_frame_to_riscv_sigcontext(&saved_tf),
        },
        trampoline: RISCV_SIGTRAMP_CODE,
    };

    let frame_ret = write_user_value(ext.process.as_ref(), frame_addr, &frame);
    if frame_ret != 0 {
        return Err(LinuxError::EFAULT);
    }

    *ext.pending_sigreturn.lock() = Some(saved_tf);
    ext.signal_frame.store(frame_addr, Ordering::Release);
    ext.pending_signal.store(0, Ordering::Release);
    let mut next_mask = current_mask | action.sa_mask.sig[0];
    if action.sa_flags & SA_NODEFER_FLAG == 0 {
        next_mask |= signal_mask_bit(sig);
    }
    ext.signal_mask.store(next_mask, Ordering::Release);

    tf.regs.sp = frame_addr;
    tf.regs.ra = frame_addr + offset_of!(RiscvSignalFrame, trampoline);
    tf.regs.a0 = sig as usize;
    tf.regs.a1 = frame_addr + offset_of!(RiscvSignalFrame, info);
    tf.regs.a2 = frame_addr + offset_of!(RiscvSignalFrame, ucontext);
    tf.sepc = handler;
    Ok(())
}

#[cfg(target_arch = "loongarch64")]
fn inject_pending_signal(
    tf: &mut TrapFrame,
    ext: &UserTaskExt,
    sig: i32,
    from_cancel_point: bool,
) -> Result<(), LinuxError> {
    let action = ext
        .process
        .signal_actions
        .lock()
        .get(&(sig as usize))
        .copied()
        .unwrap_or_else(|| unsafe { core::mem::zeroed() });
    let handler = action
        .sa_handler_kernel
        .map(|func| func as usize)
        .unwrap_or(0);
    if handler <= 1 {
        ext.pending_signal.store(0, Ordering::Release);
        return Ok(());
    }

    let current_mask = ext.signal_mask.load(Ordering::Acquire);
    let frame_size = size_of::<LoongarchSignalFrame>();
    let frame_addr = align_down(tf.regs.sp.saturating_sub(frame_size), 16);
    ensure_signal_frame_pages(ext.process.as_ref(), frame_addr, frame_size)?;

    let mut sigmask = LoongarchSignalSigset { sig: [0; 16] };
    sigmask.sig[0] = current_mask;
    let saved_tf = if sig == SIGCANCEL_NUM && from_cancel_point {
        syscall_instruction_context(tf)
    } else {
        *tf
    };
    let frame = LoongarchSignalFrame {
        info: make_loongarch_siginfo(sig, SI_TKILL_CODE, current_tid()),
        ucontext: LoongarchSignalUcontext {
            flags: 0,
            link: 0,
            stack: LoongarchSignalStack {
                sp: 0,
                stack_flags: SS_DISABLE,
                stack_pad: 0,
                size: 0,
            },
            sigmask,
            pad: 0,
            mcontext: trap_frame_to_loongarch_mcontext(&saved_tf),
        },
        trampoline: LOONGARCH_SIGTRAMP_CODE,
    };

    let frame_ret = write_user_value(ext.process.as_ref(), frame_addr, &frame);
    if frame_ret != 0 {
        return Err(LinuxError::EFAULT);
    }

    *ext.pending_sigreturn.lock() = Some(saved_tf);
    ext.signal_frame.store(frame_addr, Ordering::Release);
    ext.pending_signal.store(0, Ordering::Release);
    let mut next_mask = current_mask | action.sa_mask.sig[0];
    if action.sa_flags & SA_NODEFER_FLAG == 0 {
        next_mask |= signal_mask_bit(sig);
    }
    ext.signal_mask.store(next_mask, Ordering::Release);

    tf.regs.sp = frame_addr;
    tf.regs.ra = frame_addr + offset_of!(LoongarchSignalFrame, trampoline);
    tf.regs.a0 = sig as usize;
    tf.regs.a1 = frame_addr + offset_of!(LoongarchSignalFrame, info);
    tf.regs.a2 = frame_addr + offset_of!(LoongarchSignalFrame, ucontext);
    tf.era = handler;
    Ok(())
}

#[register_trap_handler(PAGE_FAULT)]
fn user_page_fault(vaddr: VirtAddr, flags: PageFaultFlags, _from_user: bool) -> bool {
    let Some(process) = current_process() else {
        return false;
    };
    if let Some(code) = process.pending_exit_group() {
        user_trace!(
            "user-exit-group-pf: tid={} code={code} fault_vaddr={vaddr:#x} flags={flags:?}",
            current_tid(),
        );
        terminate_current_thread(process.as_ref(), code);
    }
    let should_trace = _from_user
        && flags.contains(PageFaultFlags::WRITE)
        && vaddr.as_usize() >= USER_MMAP_BASE
        && vaddr.as_usize() < USER_STACK_TOP;
    let handled = {
        let mut aspace = process.aspace.lock();
        if should_trace {
            let _query = aspace
                .page_table()
                .query(VirtAddr::from(align_down(vaddr.as_usize(), PAGE_SIZE_4K)));
            user_trace!(
                "user-pf: vaddr={:#x} flags={flags:?} satp={:#x} aspace_root={:#x} query_before={query:?}",
                vaddr,
                axhal::asm::read_user_page_table(),
                aspace.page_table_root(),
            );
        }
        let handled = aspace.handle_page_fault(vaddr, flags);
        if should_trace {
            let _query = aspace
                .page_table()
                .query(VirtAddr::from(align_down(vaddr.as_usize(), PAGE_SIZE_4K)));
            user_trace!("user-pf: handled={handled} query_after={query:?}");
        }
        handled
    };
    if !handled && _from_user {
        if let Some(ext) = current_task_ext() {
            if !signal_is_blocked(ext, general::SIGSEGV as i32) {
                if let Some(entry) = user_thread_entry_by_tid(current_tid()) {
                    if deliver_user_signal(&entry, general::SIGSEGV as i32).is_ok() {
                        return true;
                    }
                }
            }
        }
        terminate_current_thread(process.as_ref(), 128 + 11);
    }
    handled
}

#[register_trap_handler(SYSCALL)]
fn user_syscall(tf: &TrapFrame, syscall_num: usize) -> isize {
    let Some(process) = current_process() else {
        return neg_errno(LinuxError::ENOSYS);
    };
    match syscall_num as u32 {
        general::__NR_exit | general::__NR_exit_group => {}
        _ => {
            if let Some(code) = process.pending_exit_group() {
                user_trace!(
                    "user-exit-group-syscall: tid={} code={code} syscall={} sp={:#x} ra={:#x} pc={:#x}",
                    current_tid(),
                    syscall_num,
                    tf.regs.sp,
                    tf.regs.ra,
                    user_pc(tf),
                );
                terminate_current_thread(process.as_ref(), code);
            }
        }
    };
    let ret = match syscall_num as u32 {
        general::__NR_read => sys_read(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_pread64 => sys_pread64(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3()),
        general::__NR_pwrite64 => {
            sys_pwrite64(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_write => sys_write(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_writev => sys_writev(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_readv => sys_readv(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_getcwd => sys_getcwd(&process, tf.arg0(), tf.arg1()),
        general::__NR_chdir => sys_chdir(&process, tf.arg0()),
        general::__NR_openat => sys_openat(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3()),
        general::__NR_mkdirat => sys_mkdirat(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_unlinkat => sys_unlinkat(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_pipe2 => sys_pipe2(&process, tf.arg0(), tf.arg1()),
        general::__NR_ftruncate => sys_ftruncate(&process, tf.arg0(), tf.arg1()),
        general::__NR_fsync => sys_fsync(&process, tf.arg0()),
        general::__NR_fdatasync => sys_fdatasync(&process, tf.arg0()),
        #[cfg(feature = "net")]
        general::__NR_socket => sys_socket(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        #[cfg(feature = "net")]
        general::__NR_socketpair => {
            sys_socketpair(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        #[cfg(feature = "net")]
        general::__NR_bind => sys_bind(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        #[cfg(feature = "net")]
        general::__NR_listen => sys_listen(&process, tf.arg0(), tf.arg1()),
        #[cfg(feature = "net")]
        general::__NR_accept => sys_accept4(&process, tf.arg0(), tf.arg1(), tf.arg2(), 0),
        #[cfg(feature = "net")]
        general::__NR_accept4 => sys_accept4(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3()),
        #[cfg(feature = "net")]
        general::__NR_connect => sys_connect(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        #[cfg(feature = "net")]
        general::__NR_getsockname => sys_getsockname(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        #[cfg(feature = "net")]
        general::__NR_getpeername => sys_getpeername(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        #[cfg(feature = "net")]
        general::__NR_sendto => sys_sendto(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
            tf.arg5(),
        ),
        #[cfg(feature = "net")]
        general::__NR_recvfrom => sys_recvfrom(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
            tf.arg5(),
        ),
        #[cfg(feature = "net")]
        general::__NR_setsockopt => sys_setsockopt(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
        #[cfg(feature = "net")]
        general::__NR_getsockopt => sys_getsockopt(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
        #[cfg(feature = "net")]
        general::__NR_shutdown => sys_shutdown(&process, tf.arg0(), tf.arg1()),
        general::__NR_readlinkat => {
            sys_readlinkat(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_faccessat => {
            sys_faccessat(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_utimensat => {
            sys_utimensat(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_renameat2 => sys_renameat2(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
        general::__NR_close => sys_close(&process, tf.arg0()),
        general::__NR_newfstatat => {
            sys_newfstatat(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_fstat => sys_fstat(&process, tf.arg0(), tf.arg1()),
        general::__NR_statx => sys_statx(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
        general::__NR_statfs => sys_statfs(&process, tf.arg0(), tf.arg1()),
        general::__NR_fstatfs => sys_fstatfs(&process, tf.arg0(), tf.arg1()),
        general::__NR_getdents64 => sys_getdents64(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_lseek => sys_lseek(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_dup => sys_dup(&process, tf.arg0()),
        general::__NR_dup3 => sys_dup3(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_fcntl => sys_fcntl(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_ppoll => sys_ppoll(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
        general::__NR_pselect6 => sys_pselect6(
            &process,
            tf.arg0() as i32,
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
            tf.arg5(),
        ),
        general::__NR_ioctl => sys_ioctl(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_clock_gettime => sys_clock_gettime(&process, tf.arg0(), tf.arg1()),
        general::__NR_clock_getres => sys_clock_getres(&process, tf.arg0(), tf.arg1()),
        general::__NR_gettimeofday => sys_gettimeofday(&process, tf.arg0(), tf.arg1()),
        general::__NR_getrandom => sys_getrandom(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_setitimer => sys_setitimer(&process, tf.arg0() as i32, tf.arg1(), tf.arg2()),
        general::__NR_umask => sys_umask(&process, tf.arg0() as u32),
        general::__NR_times => sys_times(&process, tf.arg0()),
        general::__NR_getrusage => sys_getrusage(&process, tf.arg0() as i32, tf.arg1()),
        general::__NR_sysinfo => sys_sysinfo(&process, tf.arg0()),
        general::__NR_uname => sys_uname(&process, tf.arg0()),
        general::__NR_nanosleep => sys_nanosleep(&process, tf, tf.arg0(), tf.arg1()),
        general::__NR_clock_nanosleep => {
            sys_clock_nanosleep(&process, tf, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_sched_yield => sys_sched_yield(tf),
        general::__NR_sched_setparam => sys_sched_setparam(&process, tf.arg0() as i32, tf.arg1()),
        general::__NR_sched_getparam => sys_sched_getparam(&process, tf.arg0() as i32, tf.arg1()),
        general::__NR_sched_setscheduler => {
            sys_sched_setscheduler(&process, tf.arg0() as i32, tf.arg1() as i32, tf.arg2())
        }
        general::__NR_sched_getscheduler => sys_sched_getscheduler(&process, tf.arg0() as i32),
        general::__NR_sched_setaffinity => {
            sys_sched_setaffinity(&process, tf.arg0() as i32, tf.arg1(), tf.arg2())
        }
        general::__NR_sched_getaffinity => {
            sys_sched_getaffinity(&process, tf.arg0() as i32, tf.arg1(), tf.arg2())
        }
        general::__NR_syslog => sys_syslog(&process, tf.arg0() as i32, tf.arg1(), tf.arg2()),
        general::__NR_gettid => imp_task::current_tid() as isize,
        general::__NR_setsid => match imp_task::setsid(process.pid()) {
            Ok(pid) => pid as isize,
            Err(err) => neg_errno(err),
        },
        general::__NR_brk => sys_brk(&process, tf.arg0()),
        general::__NR_mmap => sys_mmap(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
            tf.arg5(),
        ),
        general::__NR_mprotect => sys_mprotect(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_munmap => sys_munmap(&process, tf, tf.arg0(), tf.arg1()),
        general::__NR_mbind => sys_mbind(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
        general::__NR_get_mempolicy => sys_get_mempolicy(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
        general::__NR_set_mempolicy => sys_set_mempolicy(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_mlock
        | general::__NR_munlock
        | general::__NR_mlockall
        | general::__NR_munlockall
        | general::__NR_mlock2 => 0,
        general::__NR_set_tid_address => sys_set_tid_address(tf, tf.arg0()),
        general::__NR_set_robust_list => sys_set_robust_list(tf.arg0(), tf.arg1()),
        general::__NR_get_robust_list => {
            sys_get_robust_list(&process, tf.arg0() as i32, tf.arg1(), tf.arg2())
        }
        general::__NR_futex => sys_futex(
            &process,
            tf,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
            tf.arg5(),
        ),
        general::__NR_getuid => 0,
        general::__NR_geteuid => 0,
        general::__NR_getgid => 0,
        general::__NR_getegid => 0,
        general::__NR_setuid => 0,
        general::__NR_setgid => 0,
        general::__NR_kill => sys_kill(&process, tf.arg0() as i32, tf.arg1() as i32),
        general::__NR_tkill => sys_tkill(&process, tf.arg0() as i32, tf.arg1() as i32),
        general::__NR_tgkill => sys_tgkill(
            &process,
            tf.arg0() as i32,
            tf.arg1() as i32,
            tf.arg2() as i32,
        ),
        general::__NR_rt_sigtimedwait => {
            sys_rt_sigtimedwait(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_rt_sigaction => {
            sys_rt_sigaction(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_rt_sigreturn => sys_rt_sigreturn(&process),
        general::__NR_rt_sigprocmask => {
            sys_rt_sigprocmask(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_socket => sys_socket(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_bind => sys_bind(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_getsockname => sys_getsockname(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_setsockopt => sys_setsockopt(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
        general::__NR_sendto => sys_sendto(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
            tf.arg5(),
        ),
        general::__NR_recvfrom => sys_recvfrom(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
            tf.arg5(),
        ),
        general::__NR_listen => sys_listen(&process, tf.arg0(), tf.arg1()),
        general::__NR_connect => sys_connect(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_accept => sys_accept4(&process, tf.arg0(), tf.arg1(), tf.arg2(), 0),
        general::__NR_accept4 => sys_accept4(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3()),
        general::__NR_prlimit64 => sys_prlimit64(
            &process,
            tf.arg0() as i32,
            tf.arg1() as u32,
            tf.arg2(),
            tf.arg3(),
        ),
        general::__NR_getpid => process.pid() as isize,
        general::__NR_getppid => process.ppid() as isize,
        general::__NR_clone => {
            let (tls, ctid) = clone_tls_ctid_args(tf);
            sys_clone(&process, tf, tf.arg0(), tf.arg1(), tf.arg2(), tls, ctid)
        }
        general::__NR_execve => sys_execve(&process, tf, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_wait4 => {
            sys_wait4(&process, tf.arg0() as i32, tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_exit => sys_exit(process.as_ref(), tf, tf.arg0() as i32),
        general::__NR_exit_group => sys_exit_group(process.as_ref(), tf, tf.arg0() as i32),
        _ => neg_errno(LinuxError::ENOSYS),
    };
    ret
}

fn sys_read(process: &UserProcess, fd: usize, buf: usize, count: usize) -> isize {
    with_writable_slice(process, buf, count, |dst| {
        #[cfg(feature = "net")]
        {
            let socket = {
                let table = process.fds.lock();
                match table.entry(fd as i32)? {
                    FdEntry::Socket(socket) => Some(socket.clone()),
                    _ => None,
                }
            };
            if let Some(socket) = socket {
                return socket_retry_blocking(process, &socket, SocketWaitKind::Readable, |sock| {
                    sock.recv(dst)
                });
            }
        }
        let pipe = {
            let table = process.fds.lock();
            match table.entry(fd as i32)? {
                FdEntry::Pipe(pipe) => Some(pipe.clone()),
                _ => None,
            }
        };
        if let Some(pipe) = pipe {
            return pipe.read(dst);
        }
        process.fds.lock().read(fd as i32, dst)
    })
}

fn sys_pread64(process: &UserProcess, fd: usize, buf: usize, count: usize, offset: usize) -> isize {
    with_writable_slice(process, buf, count, |dst| {
        let mut table = process.fds.lock();
        let FdEntry::File(file) = table.entry_mut(fd as i32)? else {
            return Err(LinuxError::EBADF);
        };
        let mut filled = 0usize;
        while filled < dst.len() {
            let read = file
                .file
                .read_at(offset as u64 + filled as u64, &mut dst[filled..])
                .map_err(LinuxError::from)?;
            if read == 0 {
                break;
            }
            filled += read;
        }
        Ok(filled)
    })
}

fn sys_pwrite64(
    process: &UserProcess,
    fd: usize,
    buf: usize,
    count: usize,
    offset: usize,
) -> isize {
    with_readable_slice(process, buf, count, |src| {
        let mut table = process.fds.lock();
        let FdEntry::File(file) = table.entry_mut(fd as i32)? else {
            return Err(LinuxError::EBADF);
        };
        file.file
            .write_at(offset as u64, src)
            .map_err(LinuxError::from)
    })
}

fn sys_write(process: &UserProcess, fd: usize, buf: usize, count: usize) -> isize {
    with_readable_slice(process, buf, count, |src| {
        #[cfg(feature = "net")]
        {
            let socket = {
                let table = process.fds.lock();
                match table.entry(fd as i32)? {
                    FdEntry::Socket(socket) => Some(socket.clone()),
                    _ => None,
                }
            };
            if let Some(socket) = socket {
                return socket_retry_blocking(process, &socket, SocketWaitKind::Writable, |sock| {
                    sock.send(src)
                });
            }
        }
        let pipe = {
            let table = process.fds.lock();
            match table.entry(fd as i32)? {
                FdEntry::Pipe(pipe) => Some(pipe.clone()),
                _ => None,
            }
        };
        if let Some(pipe) = pipe {
            return pipe.write(src);
        }
        process.fds.lock().write(fd as i32, src)
    })
}

fn sys_sched_yield(_tf: &TrapFrame) -> isize {
    imp_task::sys_sched_yield() as isize
}

fn nodemask_len(maxnode: usize) -> usize {
    if maxnode == 0 {
        0
    } else {
        maxnode.div_ceil(usize::BITS as usize) * size_of::<usize>()
    }
}

fn sys_mbind(
    process: &UserProcess,
    start: usize,
    len: usize,
    mode: usize,
    nodemask: usize,
    maxnode: usize,
) -> isize {
    let _ = (start, len, mode);
    let mask_len = nodemask_len(maxnode);
    if nodemask != 0 && mask_len != 0 && user_bytes(process, nodemask, mask_len, false).is_none() {
        return neg_errno(LinuxError::EFAULT);
    }
    0
}

fn sys_get_mempolicy(
    process: &UserProcess,
    mode: usize,
    nodemask: usize,
    maxnode: usize,
    _addr: usize,
    _flags: usize,
) -> isize {
    if mode != 0 {
        let default_mode = 0i32;
        let ret = write_user_value(process, mode, &default_mode);
        if ret != 0 {
            return ret;
        }
    }
    let mask_len = nodemask_len(maxnode);
    if nodemask != 0 && mask_len != 0 {
        let Some(mask) = user_bytes_mut(process, nodemask, mask_len, true) else {
            return neg_errno(LinuxError::EFAULT);
        };
        mask.fill(0);
    }
    0
}

fn sys_set_mempolicy(process: &UserProcess, mode: usize, nodemask: usize, maxnode: usize) -> isize {
    let _ = mode;
    let mask_len = nodemask_len(maxnode);
    if nodemask != 0 && mask_len != 0 && user_bytes(process, nodemask, mask_len, false).is_none() {
        return neg_errno(LinuxError::EFAULT);
    }
    0
}

fn sys_pipe2(process: &UserProcess, pipefd: usize, flags: usize) -> isize {
    let flags = flags as u32;
    if flags & !(general::O_CLOEXEC | general::O_NONBLOCK) != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let (read_end, write_end) = PipeEndpoint::new_pair_with_flags(flags);
    let fd_flags = if flags & general::O_CLOEXEC != 0 {
        FD_CLOEXEC_FLAG
    } else {
        0
    };
    let limit = process.nofile_limit();
    let fds = {
        let mut table = process.fds.lock();
        let read_fd = match table.insert(FdEntry::Pipe(read_end), limit) {
            Ok(fd) => fd,
            Err(err) => return neg_errno(err),
        };
        let write_fd = match table.insert(FdEntry::Pipe(write_end), limit) {
            Ok(fd) => fd,
            Err(err) => {
                let _ = table.close(read_fd);
                return neg_errno(err);
            }
        };
        if fd_flags != 0 {
            let _ = table.set_fd_flags(read_fd, fd_flags);
            let _ = table.set_fd_flags(write_fd, fd_flags);
        }
        [read_fd, write_fd]
    };
    write_user_value(process, pipefd, &fds)
}

#[cfg(feature = "net")]
fn socket_entry(table: &FdTable, fd: i32) -> Result<SocketEntry, LinuxError> {
    match table.entry(fd)? {
        FdEntry::Socket(socket) => Ok(socket.clone()),
        _ => Err(LinuxError::ENOTSOCK),
    }
}

#[cfg(feature = "net")]
fn socket_is_nonblocking(socket: &SocketEntry) -> bool {
    socket.status_flags.load(Ordering::Acquire) & general::O_NONBLOCK != 0
}

#[cfg(feature = "net")]
fn socket_poll_state(socket: &SocketEntry) -> Result<PollState, LinuxError> {
    let state = socket.socket.poll()?;
    if socket.socket_type == ctypes::SOCK_STREAM {
        let err = if socket.socket.peer_addr().is_ok() {
            0
        } else if state.writable {
            LinuxError::ECONNREFUSED as i32
        } else {
            socket.pending_error.load(Ordering::Acquire)
        };
        socket.pending_error.store(err, Ordering::Release);
    }
    Ok(state)
}

#[cfg(feature = "net")]
fn socket_take_error(socket: &SocketEntry) -> i32 {
    socket.pending_error.swap(0, Ordering::AcqRel)
}

#[cfg(feature = "net")]
fn socket_timeout_deadline(socket: &SocketEntry, wait_for: SocketWaitKind) -> Option<Duration> {
    let timeout_us = match wait_for {
        SocketWaitKind::Readable => socket.recv_timeout_us.load(Ordering::Acquire),
        SocketWaitKind::Writable => socket.send_timeout_us.load(Ordering::Acquire),
    };
    (timeout_us != 0).then(|| axhal::time::wall_time() + Duration::from_micros(timeout_us))
}

#[cfg(feature = "net")]
fn read_socket_timeout_us(
    process: &UserProcess,
    optval: usize,
    optlen: usize,
) -> Result<u64, LinuxError> {
    if optlen < size_of::<general::__kernel_old_timeval>() {
        return Err(LinuxError::EINVAL);
    }
    let tv = read_user_value::<general::__kernel_old_timeval>(process, optval)?;
    if tv.tv_sec < 0 || tv.tv_usec < 0 || tv.tv_usec >= 1_000_000 {
        return Err(LinuxError::EINVAL);
    }
    Ok(tv.tv_sec as u64 * 1_000_000 + tv.tv_usec as u64)
}

#[cfg(feature = "net")]
fn socket_timeout_timeval(timeout_us: u64) -> general::__kernel_old_timeval {
    general::__kernel_old_timeval {
        tv_sec: (timeout_us / 1_000_000) as _,
        tv_usec: (timeout_us % 1_000_000) as _,
    }
}

#[cfg(feature = "net")]
#[derive(Clone, Copy)]
enum SocketWaitKind {
    Readable,
    Writable,
}

#[cfg(feature = "net")]
struct SocketNonblockingGuard {
    socket: Arc<UserSocket>,
    status_flags: Arc<AtomicU32>,
    forced: bool,
}

#[cfg(feature = "net")]
impl SocketNonblockingGuard {
    fn new(socket: &SocketEntry) -> Self {
        let forced = !socket_is_nonblocking(socket);
        if forced {
            socket.socket.set_nonblocking(true);
        }
        Self {
            socket: socket.socket.clone(),
            status_flags: socket.status_flags.clone(),
            forced,
        }
    }
}

#[cfg(feature = "net")]
impl Drop for SocketNonblockingGuard {
    fn drop(&mut self) {
        if self.forced {
            let nonblocking = self.status_flags.load(Ordering::Acquire) & general::O_NONBLOCK != 0;
            self.socket.set_nonblocking(nonblocking);
        }
    }
}

#[cfg(feature = "net")]
fn socket_wait_interruptible(process: &UserProcess) -> Result<(), LinuxError> {
    if let Some(code) = process.pending_exit_group() {
        terminate_current_thread(process, code);
    }
    if current_unblocked_pending_signal().is_some() {
        return Err(LinuxError::EINTR);
    }
    axtask::yield_now();
    Ok(())
}

#[cfg(feature = "net")]
fn socket_wait_until(
    process: &UserProcess,
    socket: &SocketEntry,
    wait_for: SocketWaitKind,
    deadline: Option<Duration>,
) -> Result<(), LinuxError> {
    loop {
        let state = socket_poll_state(socket)?;
        let ready = match wait_for {
            SocketWaitKind::Readable => state.readable,
            SocketWaitKind::Writable => state.writable,
        };
        if ready {
            return Ok(());
        }
        if deadline.is_some_and(|ddl| axhal::time::wall_time() >= ddl) {
            return Err(LinuxError::EAGAIN);
        }
        socket_wait_interruptible(process)?;
    }
}

#[cfg(feature = "net")]
fn socket_retry_blocking<T, F>(
    process: &UserProcess,
    socket: &SocketEntry,
    wait_for: SocketWaitKind,
    mut op: F,
) -> Result<T, LinuxError>
where
    F: FnMut(&UserSocket) -> Result<T, LinuxError>,
{
    let nonblocking = socket_is_nonblocking(socket);
    let deadline = socket_timeout_deadline(socket, wait_for);
    let _guard = SocketNonblockingGuard::new(socket);
    loop {
        match op(socket.socket.as_ref()) {
            Err(LinuxError::EAGAIN) if !nonblocking => {
                socket_wait_until(process, socket, wait_for, deadline)?
            }
            res => return res,
        }
    }
}

#[cfg(feature = "net")]
fn socket_connect_interruptible(
    process: &UserProcess,
    socket: &SocketEntry,
    addr: SocketAddr,
) -> Result<(), LinuxError> {
    let nonblocking = socket_is_nonblocking(socket);
    socket.pending_error.store(0, Ordering::Release);
    let _guard = SocketNonblockingGuard::new(socket);
    match socket.socket.connect(addr) {
        Err(LinuxError::EAGAIN) if socket.socket_type == ctypes::SOCK_STREAM => {
            if nonblocking {
                Err(LinuxError::EINPROGRESS)
            } else {
                socket_wait_until(process, socket, SocketWaitKind::Writable, None)?;
                match socket.socket.peer_addr() {
                    Ok(_) => Ok(()),
                    Err(LinuxError::ENOTCONN) => {
                        socket
                            .pending_error
                            .store(LinuxError::ECONNREFUSED as i32, Ordering::Release);
                        Err(LinuxError::ECONNREFUSED)
                    }
                    Err(err) => Err(err),
                }
            }
        }
        Err(err) => {
            if socket.socket_type == ctypes::SOCK_STREAM {
                socket.pending_error.store(err as i32, Ordering::Release);
            }
            Err(err)
        }
        Ok(()) => Ok(()),
    }
}

#[cfg(feature = "net")]
fn sys_socketpair(
    process: &UserProcess,
    domain: usize,
    socktype: usize,
    protocol: usize,
    sv: usize,
) -> isize {
    let domain = domain as u32;
    let socktype = socktype as u32;
    let protocol = protocol as u32;
    if sv == 0 {
        return neg_errno(LinuxError::EFAULT);
    }
    if domain != ctypes::AF_UNIX && domain != ctypes::AF_LOCAL {
        return neg_errno(LinuxError::EAFNOSUPPORT);
    }
    let fd_flags = if socktype & general::O_CLOEXEC != 0 {
        FD_CLOEXEC_FLAG
    } else {
        0
    };
    let status_flags = socktype & general::O_NONBLOCK;
    let socket_type = socktype & !(general::O_CLOEXEC | general::O_NONBLOCK);
    if socket_type != ctypes::SOCK_STREAM || protocol != 0 {
        return neg_errno(LinuxError::EINVAL);
    }

    let (left, right) = LocalStreamSocket::new_pair();
    let left = Arc::new(UserSocket::LocalStream(left));
    let right = Arc::new(UserSocket::LocalStream(right));
    let limit = process.nofile_limit();
    let fds = {
        let mut table = process.fds.lock();
        let left_fd = match table.insert(
            FdEntry::Socket(SocketEntry {
                socket: left,
                status_flags: Arc::new(AtomicU32::new(status_flags & general::O_NONBLOCK)),
                pending_error: Arc::new(AtomicI32::new(0)),
                recv_buf_size: Arc::new(AtomicU32::new(DEFAULT_SOCKET_BUFFER_SIZE)),
                send_buf_size: Arc::new(AtomicU32::new(DEFAULT_SOCKET_BUFFER_SIZE)),
                recv_timeout_us: Arc::new(AtomicU64::new(0)),
                send_timeout_us: Arc::new(AtomicU64::new(0)),
                socket_type,
            }),
            limit,
        ) {
            Ok(fd) => fd,
            Err(err) => return neg_errno(err),
        };
        if let Err(err) = table.set_fd_flags(left_fd, fd_flags) {
            let _ = table.close(left_fd);
            return neg_errno(err);
        }
        if let Ok(FdEntry::Socket(socket)) = table.entry_mut(left_fd) {
            socket
                .socket
                .set_nonblocking(status_flags & general::O_NONBLOCK != 0);
        }
        let right_fd = match table.insert(
            FdEntry::Socket(SocketEntry {
                socket: right,
                status_flags: Arc::new(AtomicU32::new(status_flags & general::O_NONBLOCK)),
                pending_error: Arc::new(AtomicI32::new(0)),
                recv_buf_size: Arc::new(AtomicU32::new(DEFAULT_SOCKET_BUFFER_SIZE)),
                send_buf_size: Arc::new(AtomicU32::new(DEFAULT_SOCKET_BUFFER_SIZE)),
                recv_timeout_us: Arc::new(AtomicU64::new(0)),
                send_timeout_us: Arc::new(AtomicU64::new(0)),
                socket_type,
            }),
            limit,
        ) {
            Ok(fd) => fd,
            Err(err) => {
                let _ = table.close(left_fd);
                return neg_errno(err);
            }
        };
        if let Err(err) = table.set_fd_flags(right_fd, fd_flags) {
            let _ = table.close(right_fd);
            let _ = table.close(left_fd);
            return neg_errno(err);
        }
        if let Ok(FdEntry::Socket(socket)) = table.entry_mut(right_fd) {
            socket
                .socket
                .set_nonblocking(status_flags & general::O_NONBLOCK != 0);
        }
        [left_fd, right_fd]
    };
    let ret = write_user_value(process, sv, &fds);
    if ret != 0 {
        let mut table = process.fds.lock();
        let _ = table.close(fds[1]);
        let _ = table.close(fds[0]);
    }
    ret
}

#[cfg(feature = "net")]
fn install_socket_fd(
    process: &UserProcess,
    socket: Arc<UserSocket>,
    socket_type: u32,
    status_flags: u32,
    fd_flags: u32,
) -> Result<i32, LinuxError> {
    socket.set_nonblocking(status_flags & general::O_NONBLOCK != 0);
    let entry = SocketEntry {
        socket,
        status_flags: Arc::new(AtomicU32::new(status_flags & general::O_NONBLOCK)),
        pending_error: Arc::new(AtomicI32::new(0)),
        recv_buf_size: Arc::new(AtomicU32::new(DEFAULT_SOCKET_BUFFER_SIZE)),
        send_buf_size: Arc::new(AtomicU32::new(DEFAULT_SOCKET_BUFFER_SIZE)),
        recv_timeout_us: Arc::new(AtomicU64::new(0)),
        send_timeout_us: Arc::new(AtomicU64::new(0)),
        socket_type,
    };
    let mut table = process.fds.lock();
    let fd = table.insert(FdEntry::Socket(entry), process.nofile_limit())?;
    table.set_fd_flags(fd, fd_flags)?;
    Ok(fd)
}

#[cfg(feature = "net")]
fn sys_socket(process: &UserProcess, domain: usize, socktype: usize, protocol: usize) -> isize {
    let domain = domain as u32;
    let socktype = socktype as u32;
    let protocol = protocol as u32;
    let fd_flags = if socktype & general::O_CLOEXEC != 0 {
        FD_CLOEXEC_FLAG
    } else {
        0
    };
    let status_flags = socktype & general::O_NONBLOCK;
    let socket_type = socktype & !(general::O_CLOEXEC | general::O_NONBLOCK);
    let socket = match resolve_socket_spec(domain, socket_type, protocol) {
        Ok(crate::imp::net::SocketSpec::Tcp) => {
            Arc::new(UserSocket::Tcp(Mutex::new(TcpSocket::new())))
        }
        Ok(crate::imp::net::SocketSpec::Udp) => {
            Arc::new(UserSocket::Udp(Mutex::new(UdpSocket::new())))
        }
        Err(err) => return neg_errno(err),
    };
    match install_socket_fd(process, socket, socket_type, status_flags, fd_flags) {
        Ok(fd) => fd as isize,
        Err(err) => neg_errno(err),
    }
}

#[cfg(feature = "net")]
fn sys_bind(process: &UserProcess, socket_fd: usize, socket_addr: usize, addrlen: usize) -> isize {
    let addr = match read_user_sockaddr(process, socket_addr, addrlen as ctypes::socklen_t) {
        Ok(addr) => addr,
        Err(err) => return neg_errno(err),
    };
    let socket = {
        let table = process.fds.lock();
        match socket_entry(&table, socket_fd as i32) {
            Ok(socket) => socket,
            Err(err) => return neg_errno(err),
        }
    };
    match socket.socket.bind(addr) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

#[cfg(feature = "net")]
fn sys_connect(
    process: &UserProcess,
    socket_fd: usize,
    socket_addr: usize,
    addrlen: usize,
) -> isize {
    let addr = match read_user_sockaddr(process, socket_addr, addrlen as ctypes::socklen_t) {
        Ok(addr) => addr,
        Err(err) => return neg_errno(err),
    };
    let socket = {
        let table = process.fds.lock();
        match socket_entry(&table, socket_fd as i32) {
            Ok(socket) => socket,
            Err(err) => return neg_errno(err),
        }
    };
    match socket_connect_interruptible(process, &socket, addr) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

#[cfg(feature = "net")]
fn sys_listen(process: &UserProcess, socket_fd: usize, _backlog: usize) -> isize {
    let socket = {
        let table = process.fds.lock();
        match socket_entry(&table, socket_fd as i32) {
            Ok(socket) => socket,
            Err(err) => return neg_errno(err),
        }
    };
    match socket.socket.listen() {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

#[cfg(feature = "net")]
fn sys_accept4(
    process: &UserProcess,
    socket_fd: usize,
    socket_addr: usize,
    socket_len: usize,
    flags: usize,
) -> isize {
    let flags = flags as u32;
    if flags & !(general::O_CLOEXEC | general::O_NONBLOCK) != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let socket = {
        let table = process.fds.lock();
        match socket_entry(&table, socket_fd as i32) {
            Ok(socket) => socket,
            Err(err) => return neg_errno(err),
        }
    };
    let accepted = match socket_retry_blocking(process, &socket, SocketWaitKind::Readable, |sock| {
        sock.accept()
    }) {
        Ok(socket) => socket,
        Err(err) => return neg_errno(err),
    };
    let peer = match accepted.peer_addr() {
        Ok(addr) => addr,
        Err(err) => return neg_errno(LinuxError::from(err)),
    };
    if socket_addr != 0 || socket_len != 0 {
        if let Err(err) = write_user_sockaddr(process, socket_addr, socket_len, peer) {
            return neg_errno(err);
        }
    }
    let accepted = Arc::new(UserSocket::Tcp(Mutex::new(accepted)));
    match install_socket_fd(
        process,
        accepted,
        ctypes::SOCK_STREAM,
        flags & general::O_NONBLOCK,
        if flags & general::O_CLOEXEC != 0 {
            FD_CLOEXEC_FLAG
        } else {
            0
        },
    ) {
        Ok(fd) => fd as isize,
        Err(err) => neg_errno(err),
    }
}

#[cfg(feature = "net")]
fn sys_getsockname(process: &UserProcess, socket_fd: usize, addr: usize, addrlen: usize) -> isize {
    let socket = {
        let table = process.fds.lock();
        match socket_entry(&table, socket_fd as i32) {
            Ok(socket) => socket,
            Err(err) => return neg_errno(err),
        }
    };
    match socket.socket.local_addr() {
        Ok(socket_addr) => match write_user_sockaddr(process, addr, addrlen, socket_addr) {
            Ok(()) => 0,
            Err(err) => neg_errno(err),
        },
        Err(err) => neg_errno(err),
    }
}

#[cfg(feature = "net")]
fn sys_getpeername(process: &UserProcess, socket_fd: usize, addr: usize, addrlen: usize) -> isize {
    let socket = {
        let table = process.fds.lock();
        match socket_entry(&table, socket_fd as i32) {
            Ok(socket) => socket,
            Err(err) => return neg_errno(err),
        }
    };
    match socket.socket.peer_addr() {
        Ok(socket_addr) => match write_user_sockaddr(process, addr, addrlen, socket_addr) {
            Ok(()) => 0,
            Err(err) => neg_errno(err),
        },
        Err(err) => neg_errno(err),
    }
}

#[cfg(feature = "net")]
fn sys_sendto(
    process: &UserProcess,
    socket_fd: usize,
    buf_ptr: usize,
    len: usize,
    _flags: usize,
    socket_addr: usize,
    addrlen: usize,
) -> isize {
    let target = if socket_addr == 0 {
        None
    } else {
        match read_user_sockaddr(process, socket_addr, addrlen as ctypes::socklen_t) {
            Ok(addr) => Some(addr),
            Err(err) => return neg_errno(err),
        }
    };
    with_readable_slice(process, buf_ptr, len, |src| {
        let socket = {
            let table = process.fds.lock();
            socket_entry(&table, socket_fd as i32)?
        };
        match target {
            Some(addr) => {
                socket_retry_blocking(process, &socket, SocketWaitKind::Writable, |sock| {
                    sock.send_to(src, addr)
                })
            }
            None => socket_retry_blocking(process, &socket, SocketWaitKind::Writable, |sock| {
                sock.send(src)
            }),
        }
    })
}

#[cfg(feature = "net")]
fn sys_recvfrom(
    process: &UserProcess,
    socket_fd: usize,
    buf_ptr: usize,
    len: usize,
    _flags: usize,
    socket_addr: usize,
    addrlen: usize,
) -> isize {
    if (socket_addr == 0) != (addrlen == 0) {
        return neg_errno(LinuxError::EFAULT);
    }
    with_writable_slice(process, buf_ptr, len, |dst| {
        let socket = {
            let table = process.fds.lock();
            socket_entry(&table, socket_fd as i32)?
        };
        let (read, peer) =
            socket_retry_blocking(process, &socket, SocketWaitKind::Readable, |sock| {
                sock.recv_from(dst)
            })?;
        if let Some(peer) = peer {
            if socket_addr != 0 {
                write_user_sockaddr(process, socket_addr, addrlen, peer)?;
            }
        }
        Ok(read)
    })
}

#[cfg(feature = "net")]
fn sys_setsockopt(
    process: &UserProcess,
    socket_fd: usize,
    level: usize,
    optname: usize,
    optval: usize,
    optlen: usize,
) -> isize {
    let socket = {
        let table = process.fds.lock();
        match socket_entry(&table, socket_fd as i32) {
            Ok(socket) => socket,
            Err(err) => return neg_errno(err),
        }
    };
    match (level as i32, optname as i32) {
        (SOL_SOCKET_LEVEL, SO_RCVTIMEO_OPT) | (SOL_SOCKET_LEVEL, SO_SNDTIMEO_OPT) => {
            if optval == 0 {
                return neg_errno(LinuxError::EFAULT);
            }
            let timeout_us = match read_socket_timeout_us(process, optval, optlen) {
                Ok(timeout) => timeout,
                Err(err) => return neg_errno(err),
            };
            match optname as i32 {
                SO_RCVTIMEO_OPT => socket.recv_timeout_us.store(timeout_us, Ordering::Release),
                SO_SNDTIMEO_OPT => socket.send_timeout_us.store(timeout_us, Ordering::Release),
                _ => {}
            }
            0
        }
        (SOL_SOCKET_LEVEL, SO_RCVBUF_OPT) | (SOL_SOCKET_LEVEL, SO_SNDBUF_OPT) => {
            if optlen < size_of::<i32>() || optval == 0 {
                return neg_errno(LinuxError::EINVAL);
            }
            let size = match read_user_value::<i32>(process, optval) {
                Ok(size) if size >= 0 => size as u32,
                Ok(_) => return neg_errno(LinuxError::EINVAL),
                Err(err) => return neg_errno(err),
            };
            match optname as i32 {
                SO_RCVBUF_OPT => socket.recv_buf_size.store(size, Ordering::Release),
                SO_SNDBUF_OPT => socket.send_buf_size.store(size, Ordering::Release),
                _ => {}
            }
            0
        }
        (SOL_SOCKET_LEVEL, SO_REUSEADDR_OPT)
        | (SOL_SOCKET_LEVEL, SO_REUSEPORT_OPT)
        | (SOL_SOCKET_LEVEL, SO_KEEPALIVE_OPT)
        | (SOL_SOCKET_LEVEL, SO_BROADCAST_OPT) => {
            if optval == 0 || optlen == 0 {
                return neg_errno(LinuxError::EFAULT);
            }
            0
        }
        (IPPROTO_TCP_LEVEL, TCP_NODELAY_OPT) => {
            if optlen < size_of::<i32>() || optval == 0 {
                return neg_errno(LinuxError::EINVAL);
            }
            let enabled = match read_user_value::<i32>(process, optval) {
                Ok(value) => value != 0,
                Err(err) => return neg_errno(err),
            };
            match socket.socket.set_nodelay(enabled) {
                Ok(()) => 0,
                Err(err) => neg_errno(err),
            }
        }
        _ => neg_errno(LinuxError::EINVAL),
    }
}

#[cfg(feature = "net")]
fn sys_getsockopt(
    process: &UserProcess,
    socket_fd: usize,
    level: usize,
    optname: usize,
    optval: usize,
    optlen: usize,
) -> isize {
    if optval == 0 || optlen == 0 {
        return neg_errno(LinuxError::EFAULT);
    }
    let socket = {
        let table = process.fds.lock();
        match socket_entry(&table, socket_fd as i32) {
            Ok(socket) => socket,
            Err(err) => return neg_errno(err),
        }
    };
    match (level as i32, optname as i32) {
        (SOL_SOCKET_LEVEL, SO_TYPE_OPT) => {
            let len = match read_user_value::<ctypes::socklen_t>(process, optlen) {
                Ok(len) => len,
                Err(err) => return neg_errno(err),
            };
            if len < size_of::<i32>() as ctypes::socklen_t {
                return neg_errno(LinuxError::EINVAL);
            }
            let ty = socket.socket_type as i32;
            if write_user_value(process, optval, &ty) != 0 {
                return neg_errno(LinuxError::EFAULT);
            }
            write_user_value(process, optlen, &(size_of::<i32>() as ctypes::socklen_t))
        }
        (SOL_SOCKET_LEVEL, SO_ERROR_OPT) => {
            let _ = socket_poll_state(&socket);
            let err = socket_take_error(&socket);
            if write_user_value(process, optval, &err) != 0 {
                return neg_errno(LinuxError::EFAULT);
            }
            write_user_value(process, optlen, &(size_of::<i32>() as ctypes::socklen_t))
        }
        (SOL_SOCKET_LEVEL, SO_RCVBUF_OPT) | (SOL_SOCKET_LEVEL, SO_SNDBUF_OPT) => {
            let len = match read_user_value::<ctypes::socklen_t>(process, optlen) {
                Ok(len) => len,
                Err(err) => return neg_errno(err),
            };
            if len < size_of::<i32>() as ctypes::socklen_t {
                return neg_errno(LinuxError::EINVAL);
            }
            let size = match optname as i32 {
                SO_RCVBUF_OPT => socket.recv_buf_size.load(Ordering::Acquire) as i32,
                SO_SNDBUF_OPT => socket.send_buf_size.load(Ordering::Acquire) as i32,
                _ => 0,
            };
            if write_user_value(process, optval, &size) != 0 {
                return neg_errno(LinuxError::EFAULT);
            }
            write_user_value(process, optlen, &(size_of::<i32>() as ctypes::socklen_t))
        }
        (SOL_SOCKET_LEVEL, SO_RCVTIMEO_OPT) | (SOL_SOCKET_LEVEL, SO_SNDTIMEO_OPT) => {
            let timeout_us = match optname as i32 {
                SO_RCVTIMEO_OPT => socket.recv_timeout_us.load(Ordering::Acquire),
                SO_SNDTIMEO_OPT => socket.send_timeout_us.load(Ordering::Acquire),
                _ => 0,
            };
            let tv = socket_timeout_timeval(timeout_us);
            if write_user_value(process, optval, &tv) != 0 {
                return neg_errno(LinuxError::EFAULT);
            }
            write_user_value(
                process,
                optlen,
                &(size_of::<general::__kernel_old_timeval>() as ctypes::socklen_t),
            )
        }
        (IPPROTO_TCP_LEVEL, TCP_NODELAY_OPT) => {
            let enabled = match socket.socket.nodelay() {
                Ok(enabled) => enabled as i32,
                Err(err) => return neg_errno(err),
            };
            if write_user_value(process, optval, &enabled) != 0 {
                return neg_errno(LinuxError::EFAULT);
            }
            write_user_value(process, optlen, &(size_of::<i32>() as ctypes::socklen_t))
        }
        (IPPROTO_TCP_LEVEL, TCP_MAXSEG_OPT) => {
            let len = match read_user_value::<ctypes::socklen_t>(process, optlen) {
                Ok(len) => len,
                Err(err) => return neg_errno(err),
            };
            if len < size_of::<i32>() as ctypes::socklen_t {
                return neg_errno(LinuxError::EINVAL);
            }
            if write_user_value(process, optval, &DEFAULT_TCP_MAXSEG) != 0 {
                return neg_errno(LinuxError::EFAULT);
            }
            write_user_value(process, optlen, &(size_of::<i32>() as ctypes::socklen_t))
        }
        (IPPROTO_TCP_LEVEL, TCP_CONGESTION_OPT) => {
            let len = match read_user_value::<ctypes::socklen_t>(process, optlen) {
                Ok(len) => len as usize,
                Err(err) => return neg_errno(err),
            };
            if len == 0 {
                return neg_errno(LinuxError::EINVAL);
            }
            let Some(dst) = user_bytes_mut(process, optval, len, true) else {
                return neg_errno(LinuxError::EFAULT);
            };
            let copy_len = dst.len().min(DEFAULT_TCP_CONGESTION.len());
            dst[..copy_len].copy_from_slice(&DEFAULT_TCP_CONGESTION[..copy_len]);
            if copy_len < dst.len() {
                dst[copy_len..].fill(0);
            }
            write_user_value(process, optlen, &(copy_len as ctypes::socklen_t))
        }
        (IPPROTO_TCP_LEVEL, TCP_INFO_OPT) => {
            let len = match read_user_value::<ctypes::socklen_t>(process, optlen) {
                Ok(len) => len as usize,
                Err(err) => return neg_errno(err),
            };
            let Some(dst) = user_bytes_mut(process, optval, len, true) else {
                return neg_errno(LinuxError::EFAULT);
            };
            dst.fill(0);
            write_user_value(process, optlen, &(dst.len() as ctypes::socklen_t))
        }
        _ => neg_errno(LinuxError::EINVAL),
    }
}

#[cfg(feature = "net")]
fn sys_shutdown(process: &UserProcess, socket_fd: usize, _how: usize) -> isize {
    let socket = {
        let table = process.fds.lock();
        match socket_entry(&table, socket_fd as i32) {
            Ok(socket) => socket,
            Err(err) => return neg_errno(err),
        }
    };
    match socket.socket.shutdown() {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_ppoll(
    process: &UserProcess,
    fds: usize,
    nfds: usize,
    timeout: usize,
    _sigmask: usize,
    _sigsetsize: usize,
) -> isize {
    let timeout = if timeout == 0 {
        None
    } else {
        match read_user_value::<general::timespec>(process, timeout) {
            Ok(ts) => match imp_time::timespec_to_duration(ts) {
                Ok(timeout) => Some(timeout),
                Err(err) => return neg_errno(err),
            },
            Err(err) => return neg_errno(err),
        }
    };
    with_user_pollfds(process, fds, nfds, |events| {
        poll_ready_events(process, events, timeout)
    })
}

fn with_user_pollfds(
    process: &UserProcess,
    ptr: usize,
    nfds: usize,
    f: impl FnOnce(&mut [general::pollfd]) -> Result<usize, LinuxError>,
) -> isize {
    let len = match nfds.checked_mul(size_of::<general::pollfd>()) {
        Some(len) => len,
        None => return neg_errno(LinuxError::EINVAL),
    };
    let Some(bytes) = user_bytes_mut(process, ptr, len, true) else {
        return neg_errno(LinuxError::EFAULT);
    };

    let mut pollfds = Vec::with_capacity(nfds);
    for index in 0..nfds {
        let src = unsafe { bytes.as_ptr().add(index * size_of::<general::pollfd>()) };
        pollfds.push(unsafe { ptr::read_unaligned(src.cast::<general::pollfd>()) });
    }

    match f(&mut pollfds) {
        Ok(ready) => {
            for (index, pollfd) in pollfds.iter().enumerate() {
                let dst = unsafe { bytes.as_mut_ptr().add(index * size_of::<general::pollfd>()) };
                unsafe { ptr::write_unaligned(dst.cast::<general::pollfd>(), *pollfd) };
            }
            ready as isize
        }
        Err(err) => neg_errno(err),
    }
}

fn poll_ready_events(
    process: &UserProcess,
    pollfds: &mut [general::pollfd],
    timeout: Option<Duration>,
) -> Result<usize, LinuxError> {
    let mut events = pollfds
        .iter()
        .map(|pollfd| PollEvent {
            fd: pollfd.fd,
            events: pollfd.events,
            revents: 0,
        })
        .collect::<Vec<_>>();
    let ready = poll_events(&mut events, timeout, |fd| process.fds.lock().poll_state(fd))?;
    for (pollfd, event) in pollfds.iter_mut().zip(events.iter()) {
        pollfd.revents = event.revents;
    }
    Ok(ready)
}

fn sys_pselect6(
    process: &UserProcess,
    nfds: i32,
    readfds: usize,
    writefds: usize,
    exceptfds: usize,
    timeout: usize,
    _sigmask: usize,
) -> isize {
    if nfds < 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let nfds = (nfds as usize).min(FD_SETSIZE);
    let read_bits = match read_fd_set(process, readfds) {
        Ok(bits) => bits,
        Err(err) => return neg_errno(err),
    };
    let write_bits = match read_fd_set(process, writefds) {
        Ok(bits) => bits,
        Err(err) => return neg_errno(err),
    };
    let except_bits = match read_fd_set(process, exceptfds) {
        Ok(bits) => bits,
        Err(err) => return neg_errno(err),
    };
    let deadline = match read_pselect_deadline(process, timeout) {
        Ok(deadline) => deadline,
        Err(err) => return neg_errno(err),
    };
    loop {
        #[cfg(feature = "net")]
        axnet::poll_interfaces();
        let mut ready_read = [0usize; FD_SET_WORDS];
        let mut ready_write = [0usize; FD_SET_WORDS];
        let mut ready_except = [0usize; FD_SET_WORDS];
        let ready = {
            let table = process.fds.lock();
            let mut count = 0usize;
            count += poll_fd_set(&table, nfds, &read_bits, &mut ready_read, SelectMode::Read);
            count += poll_fd_set(
                &table,
                nfds,
                &write_bits,
                &mut ready_write,
                SelectMode::Write,
            );
            count += poll_fd_set(
                &table,
                nfds,
                &except_bits,
                &mut ready_except,
                SelectMode::Except,
            );
            count
        };
        if ready > 0 {
            let ret = write_fd_set(process, readfds, &ready_read);
            if ret != 0 {
                return ret;
            }
            let ret = write_fd_set(process, writefds, &ready_write);
            if ret != 0 {
                return ret;
            }
            let ret = write_fd_set(process, exceptfds, &ready_except);
            if ret != 0 {
                return ret;
            }
            return ready as isize;
        }
        if deadline.is_some_and(|ddl| axhal::time::wall_time() >= ddl) {
            axtask::yield_now();
            let ret = write_fd_set(process, readfds, &[0; FD_SET_WORDS]);
            if ret != 0 {
                return ret;
            }
            let ret = write_fd_set(process, writefds, &[0; FD_SET_WORDS]);
            if ret != 0 {
                return ret;
            }
            let ret = write_fd_set(process, exceptfds, &[0; FD_SET_WORDS]);
            if ret != 0 {
                return ret;
            }
            return 0;
        }
        axtask::yield_now();
    }
}

fn sys_writev(process: &UserProcess, fd: usize, iov: usize, iovcnt: usize) -> isize {
    if iovcnt > 1024 {
        return neg_errno(LinuxError::EINVAL);
    }
    let Some(iov_bytes) = user_bytes(process, iov, iovcnt * size_of::<general::iovec>(), false)
    else {
        return neg_errno(LinuxError::EFAULT);
    };
    let pipe = {
        let table = process.fds.lock();
        match table.entry(fd as i32) {
            Ok(FdEntry::Pipe(pipe)) => Some(pipe.clone()),
            Ok(_) => None,
            Err(err) => return neg_errno(err),
        }
    };
    let mut written = 0isize;
    for chunk in iov_bytes.chunks_exact(size_of::<general::iovec>()) {
        let entry = unsafe { ptr::read_unaligned(chunk.as_ptr() as *const general::iovec) };
        let len = entry.iov_len as usize;
        if len == 0 {
            continue;
        }
        let Some(src) = user_bytes(process, entry.iov_base as usize, len, false) else {
            return neg_errno(LinuxError::EFAULT);
        };
        let n = if let Some(pipe) = pipe.as_ref() {
            match pipe.write(src) {
                Ok(v) => v,
                Err(err) => return neg_errno(err),
            }
        } else {
            match process.fds.lock().write(fd as i32, src) {
                Ok(v) => v,
                Err(err) => return neg_errno(err),
            }
        };
        written += n as isize;
        if n < len {
            break;
        }
    }
    written
}

fn sys_readv(process: &UserProcess, fd: usize, iov: usize, iovcnt: usize) -> isize {
    if iovcnt > 1024 {
        return neg_errno(LinuxError::EINVAL);
    }
    let Some(iov_bytes) = user_bytes(process, iov, iovcnt * size_of::<general::iovec>(), false)
    else {
        return neg_errno(LinuxError::EFAULT);
    };
    let pipe = {
        let table = process.fds.lock();
        match table.entry(fd as i32) {
            Ok(FdEntry::Pipe(pipe)) => Some(pipe.clone()),
            Ok(_) => None,
            Err(err) => return neg_errno(err),
        }
    };
    let mut total = 0isize;
    for chunk in iov_bytes.chunks_exact(size_of::<general::iovec>()) {
        let entry = unsafe { ptr::read_unaligned(chunk.as_ptr() as *const general::iovec) };
        let len = entry.iov_len as usize;
        if len == 0 {
            continue;
        }
        let Some(dst) = user_bytes_mut(process, entry.iov_base as usize, len, true) else {
            return neg_errno(LinuxError::EFAULT);
        };
        let n = if let Some(pipe) = pipe.as_ref() {
            match pipe.read(dst) {
                Ok(v) => v,
                Err(err) => return neg_errno(err),
            }
        } else {
            match process.fds.lock().read(fd as i32, dst) {
                Ok(v) => v,
                Err(err) => return neg_errno(err),
            }
        };
        total += n as isize;
        if n < len {
            break;
        }
    }
    total
}

fn sys_getcwd(process: &UserProcess, buf: usize, size: usize) -> isize {
    let cwd = process.cwd();
    let mut bytes = cwd.into_bytes();
    bytes.push(0);
    if bytes.len() > size {
        return neg_errno(LinuxError::ERANGE);
    }
    let Some(dst) = user_bytes_mut(process, buf, bytes.len(), true) else {
        return neg_errno(LinuxError::EFAULT);
    };
    dst.copy_from_slice(&bytes);
    bytes.len() as isize
}

fn sys_chdir(process: &UserProcess, pathname: usize) -> isize {
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    let cwd = process.cwd();
    let abs_path = match resolve_host_path(cwd, path.as_str()) {
        Ok(path) => path,
        Err(_) => return neg_errno(LinuxError::EINVAL),
    };
    if open_dir_entry(abs_path.as_str(), FileTimes::default()).is_err() {
        return neg_errno(LinuxError::ENOENT);
    }
    process.set_cwd(abs_path);
    0
}

fn sys_execve(
    process: &UserProcess,
    _tf: &TrapFrame,
    pathname: usize,
    argv: usize,
    _envp: usize,
) -> isize {
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    let argv = match read_execve_argv(process, argv, path.as_str()) {
        Ok(argv) => argv,
        Err(err) => return neg_errno(err),
    };
    let cwd = process.cwd();
    let (entry, stack_ptr, argc) = match exec_program(process, cwd.as_str(), &argv) {
        Ok(image) => image,
        Err(_) => return neg_errno(LinuxError::ENOEXEC),
    };
    let context = make_uspace_context(entry, stack_ptr, argc);
    let kstack_top = axtask::current()
        .kernel_stack_top()
        .expect("user task must have a kernel stack");
    unsafe { context.enter_uspace(kstack_top) }
}

fn sys_clone(
    process: &Arc<UserProcess>,
    tf: &TrapFrame,
    flags: usize,
    child_stack: usize,
    ptid: usize,
    arg3: usize,
    arg4: usize,
) -> isize {
    #[cfg(target_arch = "loongarch64")]
    let (ctid, tls) = (arg3, arg4);
    #[cfg(not(target_arch = "loongarch64"))]
    let (tls, ctid) = (arg3, arg4);
    let exit_signal = flags & 0xff;
    let clone_flags = flags & !0xff;
    user_trace!(
        "thrclone: tid={} pid={} flags={flags:#x} clone_flags={clone_flags:#x} exit_signal={exit_signal} stack={child_stack:#x} ptid={ptid:#x} tls={tls:#x} ctid={ctid:#x} pc={:#x} sp={:#x} tp={:#x}",
        current_tid(),
        process.pid(),
        user_pc(tf),
        tf.regs.sp,
        tf.regs.tp,
    );
    let inherited_signal_mask = current_task_ext()
        .map(|ext| ext.signal_mask.load(Ordering::Acquire))
        .unwrap_or(0);
    let vfork_flags = general::CLONE_VM as usize | general::CLONE_VFORK as usize;
    let process_allowed_flags = vfork_flags
        | general::CLONE_SETTLS as usize
        | general::CLONE_PARENT_SETTID as usize
        | general::CLONE_CHILD_SETTID as usize
        | general::CLONE_CHILD_CLEARTID as usize;
    let fork_like_flags = clone_flags & !process_allowed_flags == 0
        && (clone_flags & general::CLONE_VM as usize == 0
            || clone_flags & vfork_flags == vfork_flags);
    if fork_like_flags {
        if !matches!(exit_signal, 0) && exit_signal != SIGCHLD_NUM as usize {
            return neg_errno(LinuxError::ENOSYS);
        }
        if clone_flags & general::CLONE_PARENT_SETTID as usize != 0 && ptid == 0 {
            return neg_errno(LinuxError::EFAULT);
        }
        if clone_flags
            & (general::CLONE_CHILD_SETTID as usize | general::CLONE_CHILD_CLEARTID as usize)
            != 0
            && ctid == 0
        {
            return neg_errno(LinuxError::EFAULT);
        }

        let child_process = match process.fork() {
            Ok(process) => process,
            Err(err) => return neg_errno(err),
        };
        let mut child_tf = child_trap_frame(tf, child_stack);
        if clone_flags & general::CLONE_SETTLS as usize != 0 {
            child_tf.regs.tp = tls;
        }
        #[cfg(target_arch = "riscv64")]
        fixup_riscv_clone_child_return(process.as_ref(), &mut child_tf);
        let child_context = UspaceContext::from(&child_tf);
        let task_process = child_process.clone();
        let mut task = TaskInner::new(
            move || user_task_entry(task_process, child_context),
            "user:fork".into(),
            64 * 1024,
        );
        let pid = task.id().as_u64() as i32;
        child_process.set_pid(pid);
        if clone_flags & general::CLONE_PARENT_SETTID as usize != 0 {
            let ret = write_user_value(process.as_ref(), ptid, &pid);
            if ret != 0 {
                child_process.teardown();
                return ret;
            }
        }
        if clone_flags & general::CLONE_CHILD_SETTID as usize != 0 {
            let ret = write_user_value(child_process.as_ref(), ctid, &pid);
            if ret != 0 {
                child_process.teardown();
                return ret;
            }
        }
        let child_clear_tid = if clone_flags & general::CLONE_CHILD_CLEARTID as usize != 0 {
            ctid
        } else {
            0
        };
        let root = child_process.aspace.lock().page_table_root();
        task.ctx_mut().set_page_table_root(root);
        task.init_task_ext(UserTaskExt {
            process: child_process.clone(),
            clear_child_tid: AtomicUsize::new(child_clear_tid),
            pending_signal: AtomicI32::new(0),
            signal_mask: AtomicU64::new(inherited_signal_mask),
            signal_wait: WaitQueue::new(),
            futex_wait: AtomicUsize::new(0),
            futex_token: Mutex::new(None),
            robust_list_head: AtomicUsize::new(0),
            robust_list_len: AtomicUsize::new(0),
            deferred_unmap_start: AtomicUsize::new(0),
            deferred_unmap_len: AtomicUsize::new(0),
            signal_frame: AtomicUsize::new(0),
            sigcancel_delivery_armed: AtomicBool::new(false),
            pending_sigreturn: Mutex::new(None),
        });
        let task = axtask::spawn_task(task);
        register_user_task(task.clone(), child_process.clone());
        process.add_child(task, child_process);
        return pid as isize;
    }

    const THREAD_REQUIRED_FLAGS: usize = general::CLONE_VM as usize
        | general::CLONE_FS as usize
        | general::CLONE_FILES as usize
        | general::CLONE_SIGHAND as usize
        | general::CLONE_SYSVSEM as usize
        | general::CLONE_THREAD as usize;
    const THREAD_ALLOWED_FLAGS: usize = THREAD_REQUIRED_FLAGS
        | general::CLONE_SETTLS as usize
        | general::CLONE_PARENT_SETTID as usize
        | general::CLONE_CHILD_CLEARTID as usize
        | general::CLONE_CHILD_SETTID as usize
        | general::CLONE_DETACHED as usize
        | general::CLONE_UNTRACED as usize;

    if exit_signal != 0
        || clone_flags & THREAD_REQUIRED_FLAGS != THREAD_REQUIRED_FLAGS
        || clone_flags & !THREAD_ALLOWED_FLAGS != 0
        || child_stack == 0
    {
        return neg_errno(LinuxError::ENOSYS);
    }

    if clone_flags & general::CLONE_PARENT_SETTID as usize != 0 && ptid == 0 {
        return neg_errno(LinuxError::EFAULT);
    }
    if clone_flags & (general::CLONE_CHILD_SETTID as usize | general::CLONE_CHILD_CLEARTID as usize)
        != 0
        && ctid == 0
    {
        return neg_errno(LinuxError::EFAULT);
    }

    let mut child_tf = child_trap_frame(tf, child_stack);
    if clone_flags & general::CLONE_SETTLS as usize != 0 {
        child_tf.regs.tp = tls;
    }
    #[cfg(target_arch = "riscv64")]
    fixup_riscv_clone_child_return(process.as_ref(), &mut child_tf);
    let child_context = UspaceContext::from(&child_tf);
    let child_set_tid = if clone_flags & general::CLONE_CHILD_SETTID as usize != 0 {
        ctid
    } else {
        0
    };
    let child_clear_tid = if clone_flags & general::CLONE_CHILD_CLEARTID as usize != 0 {
        ctid
    } else {
        0
    };
    let task_process = process.clone();
    let mut task = TaskInner::new(
        move || user_thread_entry(task_process, child_context, child_set_tid),
        "user:thread".into(),
        64 * 1024,
    );
    let tid = task.id().as_u64() as i32;
    let root = process.aspace.lock().page_table_root();
    task.ctx_mut().set_page_table_root(root);
    task.init_task_ext(UserTaskExt {
        process: process.clone(),
        clear_child_tid: AtomicUsize::new(child_clear_tid),
        pending_signal: AtomicI32::new(0),
        signal_mask: AtomicU64::new(inherited_signal_mask),
        signal_wait: WaitQueue::new(),
        futex_wait: AtomicUsize::new(0),
        futex_token: Mutex::new(None),
        robust_list_head: AtomicUsize::new(0),
        robust_list_len: AtomicUsize::new(0),
        deferred_unmap_start: AtomicUsize::new(0),
        deferred_unmap_len: AtomicUsize::new(0),
        signal_frame: AtomicUsize::new(0),
        sigcancel_delivery_armed: AtomicBool::new(false),
        pending_sigreturn: Mutex::new(None),
    });

    if clone_flags & general::CLONE_PARENT_SETTID as usize != 0 {
        let ret = write_user_value(process.as_ref(), ptid, &tid);
        if ret != 0 {
            return ret;
        }
    }
    process.add_thread();
    let spawned = axtask::spawn_task(task);
    register_user_task(spawned, process.clone());
    tid as isize
}

fn sys_wait4(
    process: &UserProcess,
    pid: i32,
    status: usize,
    options: usize,
    _rusage: usize,
) -> isize {
    const SUPPORTED_WAIT_OPTIONS: u32 = general::WNOHANG
        | general::WUNTRACED
        | general::WCONTINUED
        | general::__WNOTHREAD
        | general::__WALL
        | general::__WCLONE;

    let options = options as u32;
    if options & !SUPPORTED_WAIT_OPTIONS != 0 {
        return neg_errno(LinuxError::EINVAL);
    }

    let nohang = options & general::WNOHANG != 0;
    let Some((child_pid, exit_code)) = (match process.wait_child(pid, nohang) {
        Ok(result) => result,
        Err(err) => return neg_errno(err),
    }) else {
        return 0;
    };
    user_trace!("user-wait4: requested pid={pid}, child={child_pid}, exit={exit_code}");
    if status != 0 {
        let wait_status = (exit_code & 0xff) << 8;
        let ret = write_user_value(process, status, &wait_status);
        if ret != 0 {
            return ret;
        }
    }
    child_pid as isize
}

fn sys_openat(
    process: &UserProcess,
    dirfd: usize,
    pathname: usize,
    flags: usize,
    _mode: usize,
) -> isize {
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    match process
        .fds
        .lock()
        .open(process, dirfd as i32, path.as_str(), flags as u32)
    {
        Ok(fd) => fd as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_mkdirat(process: &UserProcess, dirfd: usize, pathname: usize, _mode: usize) -> isize {
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    match process
        .fds
        .lock()
        .mkdirat(process, dirfd as i32, path.as_str())
    {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_unlinkat(process: &UserProcess, dirfd: usize, pathname: usize, flags: usize) -> isize {
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    match process
        .fds
        .lock()
        .unlinkat(process, dirfd as i32, path.as_str(), flags as u32)
    {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_faccessat(
    process: &UserProcess,
    dirfd: usize,
    pathname: usize,
    _mode: usize,
    _flags: usize,
) -> isize {
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    match process
        .fds
        .lock()
        .stat_path(process, dirfd as i32, path.as_str())
    {
        Ok(_) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_readlinkat(
    process: &UserProcess,
    dirfd: usize,
    pathname: usize,
    buf: usize,
    bufsiz: usize,
) -> isize {
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    let target = {
        let table = process.fds.lock();
        let abs_path = match resolve_dirfd_path(process, &table, dirfd as i32, path.as_str()) {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        };
        if let Some(target) = proc_magiclink_target(process, &table, abs_path.as_str()) {
            target
        } else {
            match axfs::api::metadata(abs_path.as_str()) {
                Ok(meta) if meta.file_type() == axfs::api::FileType::SymLink => {
                    match axfs::api::read_to_string(abs_path.as_str()) {
                        Ok(target) => target,
                        Err(err) => return neg_errno(LinuxError::from(err)),
                    }
                }
                Ok(_) => return neg_errno(LinuxError::EINVAL),
                Err(err) => return neg_errno(LinuxError::from(err)),
            }
        }
    };
    with_writable_slice(process, buf, bufsiz, |dst| {
        let src = target.as_bytes();
        let len = cmp::min(dst.len(), src.len());
        dst[..len].copy_from_slice(&src[..len]);
        Ok(len)
    })
}

fn sys_ftruncate(process: &UserProcess, fd: usize, length: usize) -> isize {
    match process.fds.lock().truncate(fd as i32, length as u64) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_fsync(process: &UserProcess, fd: usize) -> isize {
    match process.fds.lock().sync(fd as i32) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_fdatasync(process: &UserProcess, fd: usize) -> isize {
    sys_fsync(process, fd)
}

fn sys_utimensat(
    process: &UserProcess,
    dirfd: usize,
    pathname: usize,
    times: usize,
    flags: usize,
) -> isize {
    let updates = match read_utime_updates(process, times) {
        Ok(updates) => updates,
        Err(err) => return neg_errno(err),
    };
    if pathname == 0 {
        return match process
            .fds
            .lock()
            .update_fd_times(dirfd as i32, updates.0, updates.1)
        {
            Ok(()) => 0,
            Err(err) => neg_errno(err),
        };
    }
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    if path.is_empty() && flags as u32 & general::AT_EMPTY_PATH != 0 {
        return match process
            .fds
            .lock()
            .update_fd_times(dirfd as i32, updates.0, updates.1)
        {
            Ok(()) => 0,
            Err(err) => neg_errno(err),
        };
    }
    let abs_path = {
        let table = process.fds.lock();
        match resolve_dirfd_path(process, &table, dirfd as i32, path.as_str()) {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        };
        let table = process.fds.lock();
        let abs_path = match resolve_dirfd_path(process, &table, dirfd as i32, path.as_str()) {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        };
        Some(abs_path)
    };

    if let Some(abs_path) = abs_path.as_ref() {
        if let Err(err) = axfs::api::metadata(abs_path.as_str()) {
            return neg_errno(LinuxError::from(err));
        }
    }

    let now = {
        let now = axhal::time::wall_time();
        general::timespec {
            tv_sec: now.as_secs() as _,
            tv_nsec: now.subsec_nanos() as _,
        }
    };
    match axfs::api::metadata(abs_path.as_str()) {
        Ok(_) => match process
            .fds
            .lock()
            .update_path_times(abs_path.as_str(), updates.0, updates.1)
        {
            Ok(()) => 0,
            Err(err) => neg_errno(err),
        },
        Err(err) => neg_errno(LinuxError::from(err)),
    }
}

fn sys_renameat2(
    process: &UserProcess,
    olddirfd: usize,
    oldpath: usize,
    newdirfd: usize,
    newpath: usize,
    flags: usize,
) -> isize {
    if flags != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let old_path = match read_cstr(process, oldpath) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    let new_path = match read_cstr(process, newpath) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    let (old_abs_path, new_abs_path) = {
        let table = process.fds.lock();
        let old_abs = match resolve_dirfd_path(process, &table, olddirfd as i32, old_path.as_str())
        {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        };
        let new_abs = match resolve_dirfd_path(process, &table, newdirfd as i32, new_path.as_str())
        {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        };
        (old_abs, new_abs)
    };
    match axfs::api::rename(old_abs_path.as_str(), new_abs_path.as_str()) {
        Ok(()) => 0,
        Err(err) => neg_errno(LinuxError::from(err)),
    }
}

fn sys_close(process: &UserProcess, fd: usize) -> isize {
    match process.fds.lock().close(fd as i32) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_newfstatat(
    process: &UserProcess,
    dirfd: usize,
    pathname: usize,
    statbuf: usize,
    flags: usize,
) -> isize {
    let empty_path = flags as u32 & general::AT_EMPTY_PATH != 0;
    let path = if pathname == 0 && empty_path {
        String::new()
    } else {
        match read_cstr(process, pathname) {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        }
    };
    if path.is_empty() && empty_path {
        let st = {
            let mut table = process.fds.lock();
            if dirfd as i32 == general::AT_FDCWD {
                match table.stat_path(process, general::AT_FDCWD, ".") {
                    Ok(st) => st,
                    Err(err) => return neg_errno(err),
                }
            } else {
                match table.stat(dirfd as i32) {
                    Ok(st) => st,
                    Err(err) => return neg_errno(err),
                }
            }
        };
        return write_user_value(process, statbuf, &st);
    };
    let st = match process
        .fds
        .lock()
        .stat_path(process, dirfd as i32, path.as_str())
    {
        Ok(st) => st,
        Err(err) => return neg_errno(err),
    };
    write_user_value(process, statbuf, &st)
}

fn sys_fstat(process: &UserProcess, fd: usize, statbuf: usize) -> isize {
    let st = match process.fds.lock().stat(fd as i32) {
        Ok(st) => st,
        Err(err) => return neg_errno(err),
    };
    write_user_value(process, statbuf, &st)
}

fn sys_statx(
    process: &UserProcess,
    dirfd: usize,
    pathname: usize,
    flags: usize,
    _mask: usize,
    statxbuf: usize,
) -> isize {
    let flags = flags as u32;
    let st = if pathname == 0 || flags & general::AT_EMPTY_PATH != 0 {
        match process.fds.lock().stat(dirfd as i32) {
            Ok(st) => st,
            Err(err) => return neg_errno(err),
        }
    } else {
        let path = match read_cstr(process, pathname) {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        };
        if path.is_empty() {
            match process.fds.lock().stat(dirfd as i32) {
                Ok(st) => st,
                Err(err) => return neg_errno(err),
            }
        } else {
            match process
                .fds
                .lock()
                .stat_path(process, dirfd as i32, path.as_str())
            {
                Ok(st) => st,
                Err(err) => return neg_errno(err),
            }
        }
    };
    let stx = stat_to_statx(&st);
    write_user_value(process, statxbuf, &stx)
}

fn sys_statfs(process: &UserProcess, pathname: usize, buf: usize) -> isize {
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    if path.is_empty() {
        return neg_errno(LinuxError::ENOENT);
    }
    let st = default_statfs();
    write_user_value(process, buf, &st)
}

fn sys_fstatfs(process: &UserProcess, fd: usize, buf: usize) -> isize {
    if process.fds.lock().entry(fd as i32).is_err() {
        return neg_errno(LinuxError::EBADF);
    }
    let st = default_statfs();
    write_user_value(process, buf, &st)
}

fn sys_getdents64(process: &UserProcess, fd: usize, dirp: usize, count: usize) -> isize {
    let Some(dst) = user_bytes_mut(process, dirp, count, true) else {
        return neg_errno(LinuxError::EFAULT);
    };
    match process.fds.lock().getdents64(fd as i32, dst) {
        Ok(n) => n as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_lseek(process: &UserProcess, fd: usize, offset: usize, whence: usize) -> isize {
    match process
        .fds
        .lock()
        .lseek(fd as i32, offset as isize as i64, whence as u32)
    {
        Ok(v) => v as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_dup(process: &UserProcess, fd: usize) -> isize {
    match process.fds.lock().dup(fd as i32, process.nofile_limit()) {
        Ok(new_fd) => new_fd as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_dup3(process: &UserProcess, oldfd: usize, newfd: usize, flags: usize) -> isize {
    match process.fds.lock().dup3(
        oldfd as i32,
        newfd as i32,
        flags as u32,
        process.nofile_limit(),
    ) {
        Ok(fd) => fd as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_fcntl(process: &UserProcess, fd: usize, cmd: usize, arg: usize) -> isize {
    match process
        .fds
        .lock()
        .fcntl(fd as i32, cmd as u32, arg, process.nofile_limit())
    {
        Ok(v) => v as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_ioctl(process: &UserProcess, fd: usize, req: usize, arg: usize) -> isize {
    if req as u32 == ioctl::TIOCGWINSZ {
        let winsize = imp_system::current_terminal_size();
        let winsize = general::winsize {
            ws_row: winsize.rows,
            ws_col: winsize.cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        if process.fds.lock().is_stdio(fd as i32) {
            return write_user_value(process, arg, &winsize);
        }
    }
    match process
        .fds
        .lock()
        .ioctl(process, fd as i32, req as u32, arg)
    {
        Ok(v) => v as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_clock_gettime(process: &UserProcess, clk_id: usize, tp: usize) -> isize {
    let ts = match imp_time::clock_gettime_value(clk_id as u32) {
        Ok(ts) => ts,
        Err(err) => return neg_errno(err),
    };
    write_user_value(process, tp, &ts)
}

fn sys_clock_getres(process: &UserProcess, clk_id: usize, tp: usize) -> isize {
    let ts = match imp_time::clock_getres_value(clk_id as u32) {
        Ok(ts) => ts,
        Err(err) => return neg_errno(err),
    };
    if tp == 0 {
        return 0;
    }
    write_user_value(process, tp, &ts)
}

fn sys_gettimeofday(process: &UserProcess, tv: usize, tz: usize) -> isize {
    let (timeval, timezone) = imp_time::gettimeofday_values();
    if tv != 0 {
        let ret = write_user_value(process, tv, &timeval);
        if ret != 0 {
            return ret;
        }
    }
    if tz != 0 {
        let ret = write_user_value(process, tz, &timezone);
        if ret != 0 {
            return ret;
        }
    }
    0
}

fn sys_getrandom(process: &UserProcess, buf: usize, buflen: usize, flags: usize) -> isize {
    let flags = flags as u32;
    with_writable_slice(process, buf, buflen, |dst| {
        imp_system::getrandom(dst, flags)
    })
}

fn sys_setitimer(process: &UserProcess, which: i32, new_value: usize, old_value: usize) -> isize {
    if which != general::ITIMER_REAL as i32 {
        return neg_errno(LinuxError::EINVAL);
    }
    let value = if new_value == 0 {
        general::itimerval {
            it_interval: general::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            it_value: general::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
        }
    } else {
        match read_user_value::<general::itimerval>(process, new_value) {
            Ok(value) => value,
            Err(err) => return neg_errno(err),
        }
    };
    let pid = process.pid();
    let Some(process_ref) = current_process() else {
        return neg_errno(LinuxError::EINVAL);
    };
    let old = match imp_time::set_real_interval_timer(pid, value, move || {
        let _ = sys_kill(process_ref.as_ref(), pid, general::SIGALRM as i32);
    }) {
        Ok(old) => old,
        Err(err) => return neg_errno(err),
    };
    if old_value != 0 {
        let ret = write_user_value(process, old_value, &old);
        if ret != 0 {
            return ret;
        }
    }
    0
}

fn sys_umask(process: &UserProcess, mask: u32) -> isize {
    crate::imp::fs::set_process_umask(process.pid(), mask) as isize
}

fn sys_times(process: &UserProcess, buf: usize) -> isize {
    let ticks = imp_system::monotonic_ticks();
    let tms = Tms {
        tms_utime: ticks as c_long,
        tms_stime: 0,
        tms_cutime: 0,
        tms_cstime: 0,
    };
    let ret = write_user_value(process, buf, &tms);
    if ret != 0 {
        return ret;
    }
    ticks as isize
}

fn is_same_sched_target(process: &UserProcess, pid: i32) -> bool {
    imp_task::is_same_sched_target(process.pid(), pid)
}

fn sys_sched_setparam(process: &UserProcess, pid: i32, param: usize) -> isize {
    if !is_same_sched_target(process, pid) {
        return neg_errno(LinuxError::ESRCH);
    }
    if param == 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    match read_user_value::<UserSchedParam>(process, param) {
        Ok(value) => match imp_task::validate_sched_param(value.sched_priority) {
            Ok(()) => 0,
            Err(err) => neg_errno(err),
        },
        Err(err) => neg_errno(err),
    }
}

fn sys_sched_getparam(process: &UserProcess, pid: i32, param: usize) -> isize {
    if !is_same_sched_target(process, pid) {
        return neg_errno(LinuxError::ESRCH);
    }
    if param == 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let value = UserSchedParam { sched_priority: 0 };
    write_user_value(process, param, &value)
}

fn sys_sched_setscheduler(process: &UserProcess, pid: i32, policy: i32, param: usize) -> isize {
    if !is_same_sched_target(process, pid) {
        return neg_errno(LinuxError::ESRCH);
    }
    if param == 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let param = match read_user_value::<UserSchedParam>(process, param) {
        Ok(param) => param,
        Err(err) => return neg_errno(err),
    };
    match imp_task::validate_scheduler(policy as u32, param.sched_priority) {
        Ok(value) => value as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_sched_getscheduler(process: &UserProcess, pid: i32) -> isize {
    if !is_same_sched_target(process, pid) {
        return neg_errno(LinuxError::ESRCH);
    }
    imp_task::current_scheduler() as isize
}

fn sys_sched_setaffinity(process: &UserProcess, pid: i32, cpusetsize: usize, mask: usize) -> isize {
    if !is_same_sched_target(process, pid) {
        return neg_errno(LinuxError::ESRCH);
    }
    if cpusetsize == 0 || mask == 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    with_readable_slice(process, mask, cpusetsize, |src| {
        imp_task::set_current_affinity_from_bytes(src)?;
        Ok(0)
    })
}

fn sys_sched_getaffinity(process: &UserProcess, pid: i32, cpusetsize: usize, mask: usize) -> isize {
    if !is_same_sched_target(process, pid) {
        return neg_errno(LinuxError::ESRCH);
    }
    if cpusetsize == 0 || mask == 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    with_writable_slice(process, mask, cpusetsize, |dst| {
        imp_task::current_affinity_to_bytes(dst)
    })
}

fn sys_syslog(process: &UserProcess, log_type: i32, buf: usize, len: usize) -> isize {
    match log_type {
        3 | 4 => {
            if len == 0 {
                return 0;
            }
            if buf == 0 {
                return neg_errno(LinuxError::EINVAL);
            }
            with_writable_slice(process, buf, len, |dst| {
                imp_system::syslog(log_type, Some(dst))
            })
        }
        _ => match imp_system::syslog(log_type, None) {
            Ok(value) => value as isize,
            Err(err) => neg_errno(err),
        },
    }
}

fn sys_getrusage(process: &UserProcess, who: i32, usage: usize) -> isize {
    let value = match imp_system::getrusage(who) {
        Ok(value) => value,
        Err(err) => return neg_errno(err),
    };
    write_user_value(process, usage, &value)
}

fn sys_sysinfo(process: &UserProcess, info: usize) -> isize {
    let sysinfo = imp_system::current_sysinfo();
    with_writable_slice(process, info, size_of_val(&sysinfo), |dst| {
        unsafe {
            ptr::write_unaligned(dst.as_mut_ptr() as *mut _, sysinfo);
        }
        Ok(0)
    })
}

fn sys_uname(process: &UserProcess, buf: usize) -> isize {
    let uts = imp_system::current_utsname();
    write_user_value(process, buf, &uts)
}

fn sleep_interruptible(
    process: &UserProcess,
    tp: usize,
    duration: core::time::Duration,
) -> Result<(), core::time::Duration> {
    let deadline = axhal::time::wall_time() + duration;
    let quantum = core::time::Duration::from_millis(1);
    loop {
        if current_sigcancel_pending(process, tp) {
            return Err(deadline.saturating_sub(axhal::time::wall_time()));
        }
        let now = axhal::time::wall_time();
        if now >= deadline {
            return Ok(());
        }
        axtask::sleep(cmp::min(deadline - now, quantum));
    }
}

fn write_remaining_sleep(
    process: &UserProcess,
    rem: usize,
    remaining: core::time::Duration,
) -> isize {
    if rem == 0 {
        return 0;
    }
    let ts = general::timespec {
        tv_sec: remaining.as_secs() as _,
        tv_nsec: remaining.subsec_nanos() as _,
    };
    write_user_value(process, rem, &ts)
}

fn sys_nanosleep(process: &UserProcess, tf: &TrapFrame, req: usize, rem: usize) -> isize {
    let duration = match read_timespec_duration(process, req) {
        Ok(duration) => duration,
        Err(err) => return neg_errno(err),
    };
    if let Err(remaining) = sleep_interruptible(process, tf.regs.tp, duration) {
        let ret = write_remaining_sleep(process, rem, remaining);
        if ret == 0 {
            arm_current_sigcancel_delivery();
            return neg_errno(LinuxError::EINTR);
        }
        return ret;
    }
    write_remaining_sleep(process, rem, core::time::Duration::ZERO)
}

fn sys_clock_nanosleep(
    process: &UserProcess,
    tf: &TrapFrame,
    clockid: usize,
    flags: usize,
    req: usize,
    rem: usize,
) -> isize {
    let req = match read_user_value::<general::timespec>(process, req) {
        Ok(req) => req,
        Err(err) => return neg_errno(err),
    };
    if let Err(err) = imp_time::clock_nanosleep(clockid as u32, flags as u32, req) {
        return neg_errno(err);
    }
    if rem != 0 {
        let zero = general::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        if let Some(delta) = duration.checked_sub(now) {
            if sleep_interruptible(process, tf.regs.tp, delta).is_err() {
                arm_current_sigcancel_delivery();
                return neg_errno(LinuxError::EINTR);
            }
        }
    }
    sys_nanosleep(process, tf, req, rem)
}

fn read_timespec_duration(
    process: &UserProcess,
    ptr: usize,
) -> Result<core::time::Duration, LinuxError> {
    let Some(bytes) = user_bytes(process, ptr, size_of::<general::timespec>(), false) else {
        return Err(LinuxError::EFAULT);
    };
    let ts = unsafe { ptr::read_unaligned(bytes.as_ptr() as *const general::timespec) };
    if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000 {
        return Err(LinuxError::EINVAL);
    }
    Ok(core::time::Duration::new(
        ts.tv_sec as u64,
        ts.tv_nsec as u32,
    ))
}

fn read_abs_timespec_timeout(
    process: &UserProcess,
    ptr: usize,
    clockid: u32,
) -> Result<core::time::Duration, LinuxError> {
    let Some(bytes) = user_bytes(process, ptr, size_of::<general::timespec>(), false) else {
        return Err(LinuxError::EFAULT);
    };
    let ts = unsafe { ptr::read_unaligned(bytes.as_ptr() as *const general::timespec) };
    if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000 {
        return Err(LinuxError::EINVAL);
    }
    let target = core::time::Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32);
    let now = clock_now_duration(clockid)?;
    Ok(target.saturating_sub(now))
}

fn clock_now_duration(clockid: u32) -> Result<core::time::Duration, LinuxError> {
    match clockid {
        general::CLOCK_REALTIME | general::CLOCK_REALTIME_COARSE | general::CLOCK_TAI => {
            Ok(axhal::time::wall_time())
        }
        general::CLOCK_MONOTONIC
        | general::CLOCK_MONOTONIC_RAW
        | general::CLOCK_MONOTONIC_COARSE
        | general::CLOCK_BOOTTIME
        | general::CLOCK_PROCESS_CPUTIME_ID
        | general::CLOCK_THREAD_CPUTIME_ID => Ok(axhal::time::monotonic_time()),
        general::CLOCK_REALTIME_ALARM | general::CLOCK_BOOTTIME_ALARM => Err(LinuxError::EINVAL),
        _ => Err(LinuxError::EINVAL),
    }
}

fn validate_clock_id(clockid: u32) -> Result<(), LinuxError> {
    clock_now_duration(clockid).map(|_| ())
}

#[derive(Clone, Copy)]
enum SelectMode {
    Read,
    Write,
    Except,
}

fn read_pselect_deadline(
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

fn read_fd_set(process: &UserProcess, ptr: usize) -> Result<[usize; FD_SET_WORDS], LinuxError> {
    if ptr == 0 {
        return Ok([0; FD_SET_WORDS]);
    }
    Ok(read_user_value::<UserFdSet>(process, ptr)?.fds_bits)
}

fn write_fd_set(process: &UserProcess, ptr: usize, bits: &[usize; FD_SET_WORDS]) -> isize {
    if ptr == 0 {
        return 0;
    }
    write_user_value(process, ptr, &UserFdSet { fds_bits: *bits })
}

fn poll_fd_set(
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

fn sys_brk(process: &UserProcess, addr: usize) -> isize {
    let mut brk = process.brk.lock();
    if addr == 0 {
        return brk.end as isize;
    }
    if addr < brk.start || addr > brk.limit {
        return brk.end as isize;
    }
    brk.end = addr;
    brk.end as isize
}

fn sys_mmap(
    process: &UserProcess,
    addr: usize,
    len: usize,
    prot: usize,
    flags: usize,
    fd: usize,
    offset: usize,
) -> isize {
    let size = align_up(len.max(1), PAGE_SIZE_4K);
    let map_fixed = flags as u32 & general::MAP_FIXED != 0;
    let request_addr = if addr == 0 {
        None
    } else {
        Some(align_down(addr, PAGE_SIZE_4K))
    };
    let map_flags = mmap_prot_to_flags(prot as u32);
    let target = {
        let mut brk = process.brk.lock();
        let start = request_addr.unwrap_or_else(|| {
            let start = align_up(brk.next_mmap, PAGE_SIZE_4K);
            brk.next_mmap = start + size + PAGE_SIZE_4K;
            start
        });
        if start < USER_MMAP_BASE || start + size >= USER_STACK_TOP - USER_STACK_SIZE {
            return neg_errno(LinuxError::ENOMEM);
        }
        start
    };
    if flags as u32 & general::MAP_ANONYMOUS != 0 && size <= 0x40000 {
        user_trace!("user-mmap: target={target:#x} len={size:#x} prot={prot:#x} flags={flags:#x}");
    }
    let populate = flags as u32 & general::MAP_ANONYMOUS == 0;
    {
        let mut aspace = process.aspace.lock();
        if map_fixed {
            let _ = aspace.unmap(VirtAddr::from(target), size);
        }
        if let Err(err) = aspace.map_alloc(VirtAddr::from(target), size, map_flags, populate) {
            return neg_errno(LinuxError::from(err));
        }
    }

    if flags as u32 & general::MAP_ANONYMOUS == 0 {
        let file_bytes = {
            let mut table = process.fds.lock();
            match table.read_file_at(fd as i32, offset as u64, len) {
                Ok(bytes) => bytes,
                Err(err) => return neg_errno(err),
            }
        };
        if let Err(err) = process
            .aspace
            .lock()
            .write(VirtAddr::from(target), &file_bytes)
        {
            return neg_errno(LinuxError::from(err));
        }
    }
    target as isize
}

fn sys_munmap(process: &UserProcess, tf: &TrapFrame, addr: usize, len: usize) -> isize {
    if len == 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let start = align_down(addr, PAGE_SIZE_4K);
    let end = align_up(addr.saturating_add(len), PAGE_SIZE_4K);
    if end <= start {
        return neg_errno(LinuxError::EINVAL);
    }
    let self_stack_unmap = (start..end).contains(&tf.regs.sp);
    if start >= USER_MMAP_BASE && end - start <= 0x40000 {
        let _query = process
            .aspace
            .lock()
            .page_table()
            .query(VirtAddr::from(start));
        user_trace!(
            "user-munmap: tid={} start={start:#x} end={end:#x} sp={:#x} tp={:#x} ra={:#x} pc={:#x} query_before={query:?}",
            current_tid(),
            tf.regs.sp,
            tf.regs.tp,
            tf.regs.ra,
            user_pc(tf),
        );
    }
    if self_stack_unmap {
        if let Some(ext) = current_task_ext() {
            user_trace!(
                "thrmunmap: defer tid={} start={start:#x} end={end:#x} sp={:#x} tp={:#x}",
                current_tid(),
                tf.regs.sp,
                tf.regs.tp,
            );
            ext.deferred_unmap_start.store(start, Ordering::Release);
            ext.deferred_unmap_len.store(end - start, Ordering::Release);
            return 0;
        }
    }
    let unmap_result = process
        .aspace
        .lock()
        .unmap(VirtAddr::from(start), end - start);
    match unmap_result {
        Ok(()) => 0,
        Err(err) => neg_errno(LinuxError::from(err)),
    }
}

fn sys_mprotect(_process: &UserProcess, _addr: usize, _len: usize, _prot: usize) -> isize {
    if _len == 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let start = align_down(_addr, PAGE_SIZE_4K);
    let end = align_up(_addr.saturating_add(_len), PAGE_SIZE_4K);
    if end <= start {
        return neg_errno(LinuxError::EINVAL);
    }
    if _len <= 0x40000 {
        user_trace!("user-mprotect: start={start:#x} end={end:#x} prot={_prot:#x}");
    }
    let prot_flags = mmap_prot_to_flags(_prot as u32);
    let mut aspace = _process.aspace.lock();
    match aspace.protect(VirtAddr::from(start), end - start, prot_flags) {
        Ok(()) => {
            // Thread stacks are typically created as PROT_NONE mappings and then
            // flipped to writable with mprotect(). Pre-fault only the stack-top
            // pages so the first user-space writes succeed without turning the
            // whole stack into eagerly allocated memory.
            if _prot as u32 & general::PROT_WRITE != 0 && end - start <= 0x40000 {
                let prefault_start = end.saturating_sub(PAGE_SIZE_4K * 2).max(start);
                for page in
                    PageIter4K::new(VirtAddr::from(prefault_start), VirtAddr::from(end)).unwrap()
                {
                    let _ = aspace.handle_page_fault(page, PageFaultFlags::WRITE);
                }
            }
            0
        }
        Err(err) => neg_errno(LinuxError::from(err)),
    }
}

fn sys_shmget(process: &UserProcess, key: i32, size: usize, shmflg: i32) -> isize {
    let create = shmflg & IPC_CREAT_FLAG != 0;
    let excl = shmflg & IPC_EXCL_FLAG != 0;

    let existing = {
        let state = shared_mem_state().lock();
        if key == IPC_PRIVATE_KEY {
            None
        } else {
            state.key_map.get(&key).copied().and_then(|shmid| {
                state
                    .segments
                    .get(&shmid)
                    .map(|segment| (shmid, segment.size))
            })
        }
    };
    if let Some((shmid, existing_size)) = existing {
        if create && excl {
            return neg_errno(LinuxError::EEXIST);
        }
        if size != 0 && size > existing_size {
            return neg_errno(LinuxError::EINVAL);
        }
        return shmid as isize;
    }
    if !create && key != IPC_PRIVATE_KEY {
        return neg_errno(LinuxError::ENOENT);
    }
    if size == 0 {
        return neg_errno(LinuxError::EINVAL);
    }

    let requested_size = size;
    let (start_vaddr, num_pages) = match alloc_shared_pages(requested_size) {
        Ok(pages) => pages,
        Err(err) => return neg_errno(err),
    };
    let map_size = num_pages * PAGE_SIZE_4K;
    let shmid = {
        let mut state = shared_mem_state().lock();
        let shmid = state.next_id;
        state.next_id += 1;
        let seq = state.next_seq;
        state.next_seq += 1;
        state.segments.insert(
            shmid,
            SharedMemSegment {
                key,
                mode: (shmflg as u32) & 0o777,
                size: requested_size,
                map_size,
                start_vaddr,
                num_pages,
                cpid: process.pid(),
                lpid: 0,
                nattch: 0,
                atime: 0,
                dtime: 0,
                ctime: ipc_now_secs(),
                seq,
                marked_destroy: false,
            },
        );
        if key != IPC_PRIVATE_KEY {
            state.key_map.insert(key, shmid);
        }
        shmid
    };
    shmid as isize
}

fn sys_shmctl(process: &UserProcess, shmid: i32, cmd: i32, buf: usize) -> isize {
    match cmd {
        IPC_STAT_CMD => {
            let ds = {
                let state = shared_mem_state().lock();
                let segment = match state.segments.get(&shmid) {
                    Some(segment) => segment,
                    None => return neg_errno(LinuxError::EINVAL),
                };
                shared_segment_to_ds(segment)
            };
            write_user_value(process, buf, &ds)
        }
        IPC_RMID_CMD => {
            let free_pages = {
                let mut state = shared_mem_state().lock();
                let key = match state.segments.get_mut(&shmid) {
                    Some(segment) => {
                        segment.marked_destroy = true;
                        segment.key
                    }
                    None => return neg_errno(LinuxError::EINVAL),
                };
                state.key_map.remove(&key);
                collect_destroyed_shared_segment(&mut state, shmid)
            };
            if let Some((start_vaddr, num_pages)) = free_pages {
                free_shared_pages(start_vaddr, num_pages);
            }
            0
        }
        IPC_SET_CMD => 0,
        _ => neg_errno(LinuxError::EINVAL),
    }
}

fn sys_shmat(process: &UserProcess, shmid: i32, shmaddr: usize, shmflg: i32) -> isize {
    match register_shmat_mapping(process, shmid, shmaddr, shmflg) {
        Ok(addr) => addr as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_shmdt(process: &UserProcess, shmaddr: usize) -> isize {
    match detach_shmat_mapping(process, shmaddr) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_set_tid_address(_tf: &TrapFrame, _tidptr: usize) -> isize {
    if let Some(ext) = current_task_ext() {
        ext.clear_child_tid.store(_tidptr, Ordering::Release);
    }
    user_trace!(
        "user-set-tid: tid={} tidptr={_tidptr:#x} sp={:#x} tp={:#x} ra={:#x} pc={:#x}",
        current_tid(),
        tf.regs.sp,
        tf.regs.tp,
        tf.regs.ra,
        user_pc(tf),
    );
    axtask::current().id().as_u64() as isize
}

fn sys_set_robust_list(head: usize, len: usize) -> isize {
    let Some(ext) = current_task_ext() else {
        return neg_errno(LinuxError::EINVAL);
    };
    ext.robust_list_head.store(head, Ordering::Release);
    ext.robust_list_len.store(len, Ordering::Release);
    0
}

fn sys_get_robust_list(process: &UserProcess, pid: i32, head_ptr: usize, len_ptr: usize) -> isize {
    let tid = if pid == 0 { current_tid() } else { pid };
    let Some(entry) = user_thread_entry_by_tid(tid) else {
        return neg_errno(LinuxError::ESRCH);
    };
    if entry.process.pid() != process.pid() {
        return neg_errno(LinuxError::EPERM);
    }
    let Some(ext) = task_ext(&entry.task) else {
        return neg_errno(LinuxError::ESRCH);
    };
    let head = ext.robust_list_head.load(Ordering::Acquire);
    let len = ext.robust_list_len.load(Ordering::Acquire);
    let ret = write_user_value(process, head_ptr, &head);
    if ret != 0 {
        return ret;
    }
    write_user_value(process, len_ptr, &len)
}

fn sys_futex(
    process: &UserProcess,
    tf: &TrapFrame,
    uaddr: usize,
    futex_op: usize,
    val: usize,
    timeout: usize,
    _uaddr2: usize,
    _val3: usize,
) -> isize {
    if uaddr == 0 || uaddr % size_of::<u32>() != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let op = futex_op as u32;
    let cmd = op & general::FUTEX_CMD_MASK as u32;
    if uaddr < USER_MMAP_BASE || (uaddr >= USER_MMAP_BASE && val <= 8) {
        user_trace!(
            "user-futex: tid={} cmd={cmd:#x} op={op:#x} uaddr={uaddr:#x} val={val:#x} timeout={timeout:#x} sp={:#x} tp={:#x} ra={:#x} pc={:#x}",
            current_tid(),
            tf.regs.sp,
            tf.regs.tp,
            tf.regs.ra,
            user_pc(tf),
        );
    }
    match cmd {
        general::FUTEX_WAIT | FUTEX_WAIT_BITSET_CMD => {
            if cmd == FUTEX_WAIT_BITSET_CMD && _val3 == 0 {
                return neg_errno(LinuxError::EINVAL);
            }
            let current = match read_user_value::<u32>(process, uaddr) {
                Ok(value) => value,
                Err(err) => return neg_errno(err),
            };
            if current != val as u32 {
                return neg_errno(LinuxError::EAGAIN);
            }
            let timeout_duration = if timeout == 0 {
                None
            } else if cmd == FUTEX_WAIT_BITSET_CMD {
                let clockid = if op & FUTEX_CLOCK_REALTIME_FLAG != 0 {
                    general::CLOCK_REALTIME
                } else {
                    general::CLOCK_MONOTONIC
                };
                match read_abs_timespec_timeout(process, timeout, clockid) {
                    Ok(duration) => Some(duration),
                    Err(err) => return neg_errno(err),
                }
            } else {
                match read_timespec_duration(process, timeout) {
                    Ok(duration) => Some(duration),
                    Err(err) => return neg_errno(err),
                }
            };
            let state = futex_state(uaddr);
            let seq = state.seq.load(Ordering::Acquire);
            if let Some(ext) = current_task_ext() {
                ext.futex_wait.store(uaddr, Ordering::Release);
                set_futex_wait_token(ext, state.seq.clone());
            }
            let wait_cond = || {
                current_task_ext().is_some_and(futex_wait_token_changed)
                    || read_user_value::<u32>(process, uaddr)
                        .map_or(true, |value| value != val as u32)
                    || current_sigcancel_pending(process, tf.regs.tp)
            };
            if let Some(dur) = timeout_duration {
                if state.queue.wait_timeout_until(dur, wait_cond) {
                    clear_current_futex_wait();
                    return neg_errno(LinuxError::ETIMEDOUT);
                }
                clear_current_futex_wait();
                let changed = state.seq.load(Ordering::Acquire) != seq
                    || read_user_value::<u32>(process, uaddr)
                        .map_or(true, |value| value != val as u32);
                if !changed && current_sigcancel_pending(process, tf.regs.tp) {
                    arm_current_sigcancel_delivery();
                    return neg_errno(LinuxError::EINTR);
                }
                return 0;
            }
            state.queue.wait_until(wait_cond);
            clear_current_futex_wait();
            let changed = state.seq.load(Ordering::Acquire) != seq
                || read_user_value::<u32>(process, uaddr).map_or(true, |value| value != val as u32);
            if !changed && current_sigcancel_pending(process, tf.regs.tp) {
                arm_current_sigcancel_delivery();
                return neg_errno(LinuxError::EINTR);
            }
            0
        }
        general::FUTEX_WAKE | FUTEX_WAKE_BITSET_CMD => {
            if cmd == FUTEX_WAKE_BITSET_CMD && _val3 == 0 {
                return neg_errno(LinuxError::EINVAL);
            }
            let woken = futex_wake_addr(uaddr, val);
            if woken != 0 {
                axtask::yield_now();
            }
            woken as isize
        }
        FUTEX_REQUEUE_CMD | FUTEX_CMP_REQUEUE_CMD => {
            if _uaddr2 == 0 {
                return neg_errno(LinuxError::EFAULT);
            }
            if cmd == FUTEX_CMP_REQUEUE_CMD {
                let current = match read_user_value::<u32>(process, uaddr) {
                    Ok(value) => value,
                    Err(err) => return neg_errno(err),
                };
                if current != _val3 as u32 {
                    return neg_errno(LinuxError::EAGAIN);
                }
            }
            let source = futex_state(uaddr);
            let target = futex_state(_uaddr2);
            source.seq.fetch_add(1, Ordering::Release);
            let (woken, requeued) =
                source
                    .queue
                    .notify_and_requeue_with(val, timeout, &target.queue, true, |task| {
                        if let Some(ext) = task_ext(task) {
                            ext.futex_wait.store(_uaddr2, Ordering::Release);
                        }
                    });
            if woken != 0 {
                axtask::yield_now();
            }
            woken.saturating_add(requeued) as isize
        }
        FUTEX_WAKE_OP_CMD => {
            let woken = match futex_wake_op(process, uaddr, val, _uaddr2, timeout, _val3 as u32) {
                Ok(woken) => woken,
                Err(err) => return neg_errno(err),
            };
            if woken != 0 {
                axtask::yield_now();
            }
            woken as isize
        }
        _ => neg_errno(LinuxError::ENOSYS),
    }
}

fn sys_rt_sigaction(
    process: &UserProcess,
    signum: usize,
    act: usize,
    oldact: usize,
    _sigsetsize: usize,
) -> isize {
    if signum == 0 || signum >= 65 {
        return neg_errno(LinuxError::EINVAL);
    }

    let new_action = if act != 0 {
        match read_user_value::<general::kernel_sigaction>(process, act) {
            Ok(value) => Some(value),
            Err(err) => return neg_errno(err),
        }
    } else {
        None
    };

    if oldact != 0 {
        let old = process
            .signal_actions
            .lock()
            .get(&signum)
            .copied()
            .unwrap_or_else(|| unsafe { core::mem::zeroed() });
        let ret = write_user_value(process, oldact, &old);
        if ret != 0 {
            return ret;
        }
    }

    if let Some(new_action) = new_action {
        process.signal_actions.lock().insert(signum, new_action);
    }

    0
}

fn sys_rt_sigreturn(process: &UserProcess) -> isize {
    #[cfg(target_arch = "riscv64")]
    {
        let Some(ext) = current_task_ext() else {
            return neg_errno(LinuxError::EINVAL);
        };
        let frame_addr = ext.signal_frame.load(Ordering::Acquire);
        if frame_addr == 0 {
            return neg_errno(LinuxError::EINVAL);
        }
        let frame = match read_user_value::<RiscvSignalFrame>(process, frame_addr) {
            Ok(frame) => frame,
            Err(err) => return neg_errno(err),
        };
        let Some(mut restored) = ext.pending_sigreturn.lock().take() else {
            return neg_errno(LinuxError::EINVAL);
        };
        apply_riscv_sigcontext(&mut restored, &frame.ucontext.mcontext);
        let sig = i32::from_ne_bytes(frame.info.bytes[0..4].try_into().unwrap_or([0; 4]));
        let requeue_sigcancel =
            sig == SIGCANCEL_NUM && musl_cancel_pending(process, restored.regs.tp);
        ext.signal_mask
            .store(frame.ucontext.sigmask.sig[0], Ordering::Release);
        ext.signal_frame.store(0, Ordering::Release);
        if requeue_sigcancel {
            ext.pending_signal.store(SIGCANCEL_NUM, Ordering::Release);
        }
        *ext.pending_sigreturn.lock() = Some(restored);
        0
    }
    #[cfg(target_arch = "loongarch64")]
    {
        let Some(ext) = current_task_ext() else {
            return neg_errno(LinuxError::EINVAL);
        };
        let frame_addr = ext.signal_frame.load(Ordering::Acquire);
        if frame_addr == 0 {
            return neg_errno(LinuxError::EINVAL);
        }
        let frame = match read_user_value::<LoongarchSignalFrame>(process, frame_addr) {
            Ok(frame) => frame,
            Err(err) => return neg_errno(err),
        };
        let Some(mut restored) = ext.pending_sigreturn.lock().take() else {
            return neg_errno(LinuxError::EINVAL);
        };
        apply_loongarch_mcontext(&mut restored, &frame.ucontext.mcontext);
        let sig = i32::from_ne_bytes(frame.info.bytes[0..4].try_into().unwrap_or([0; 4]));
        let requeue_sigcancel =
            sig == SIGCANCEL_NUM && musl_cancel_pending(process, restored.regs.tp);
        ext.signal_mask
            .store(frame.ucontext.sigmask.sig[0], Ordering::Release);
        ext.signal_frame.store(0, Ordering::Release);
        if requeue_sigcancel {
            ext.pending_signal.store(SIGCANCEL_NUM, Ordering::Release);
        }
        *ext.pending_sigreturn.lock() = Some(restored);
        0
    }
    #[cfg(not(any(target_arch = "riscv64", target_arch = "loongarch64")))]
    {
        let _ = process;
        neg_errno(LinuxError::ENOSYS)
    }
}

fn sys_rt_sigprocmask(
    process: &UserProcess,
    how: usize,
    set: usize,
    oldset: usize,
    sigsetsize: usize,
) -> isize {
    let Some(ext) = current_task_ext() else {
        return neg_errno(LinuxError::EINVAL);
    };
    if sigsetsize != 0 && sigsetsize < KERNEL_SIGSET_BYTES {
        return neg_errno(LinuxError::EINVAL);
    }
    let current_mask = ext.signal_mask.load(Ordering::Acquire);
    if oldset != 0 {
        let Some(dst) = user_bytes_mut(process, oldset, sigsetsize, true) else {
            return neg_errno(LinuxError::EFAULT);
        };
        dst.fill(0);
        if sigsetsize >= KERNEL_SIGSET_BYTES {
            dst[..KERNEL_SIGSET_BYTES].copy_from_slice(&current_mask.to_ne_bytes());
        }
    }
    if set != 0 {
        let Some(src) = user_bytes(process, set, KERNEL_SIGSET_BYTES, false) else {
            return neg_errno(LinuxError::EFAULT);
        };
        let mut set_bytes = [0u8; KERNEL_SIGSET_BYTES];
        set_bytes.copy_from_slice(src);
        let set_mask = u64::from_ne_bytes(set_bytes);
        let next_mask = match how {
            SIG_BLOCK_HOW => current_mask | set_mask,
            SIG_UNBLOCK_HOW => current_mask & !set_mask,
            SIG_SETMASK_HOW => set_mask,
            _ => return neg_errno(LinuxError::EINVAL),
        };
        ext.signal_mask.store(next_mask, Ordering::Release);
    }
    0
}

fn sys_rt_sigtimedwait(
    process: &UserProcess,
    set: usize,
    info: usize,
    timeout: usize,
    sigsetsize: usize,
) -> isize {
    let Some(ext) = current_task_ext() else {
        return neg_errno(LinuxError::EINVAL);
    };
    if sigsetsize != 0 && sigsetsize < KERNEL_SIGSET_BYTES {
        return neg_errno(LinuxError::EINVAL);
    }
    let Some(src) = user_bytes(process, set, KERNEL_SIGSET_BYTES, false) else {
        return neg_errno(LinuxError::EFAULT);
    };
    let mut set_bytes = [0u8; KERNEL_SIGSET_BYTES];
    set_bytes.copy_from_slice(src);
    let set_mask = u64::from_ne_bytes(set_bytes);
    let timeout = match read_futex_relative_timeout(process, timeout) {
        Ok(timeout) => timeout,
        Err(err) => return neg_errno(err),
    };

    let wait_cond = || pending_signal_in_set(ext, set_mask).is_some();
    if wait_cond() {
    } else if let Some(timeout) = timeout {
        let dur = match timeout {
            FutexTimeout::Relative(dur) | FutexTimeout::Absolute(dur) => dur,
        };
        if ext.signal_wait.wait_timeout_until(dur, wait_cond) {
            return neg_errno(LinuxError::EAGAIN);
        }
    } else {
        ext.signal_wait.wait_until(wait_cond);
    }

    let Some(sig) = take_pending_signal_in_set(ext, set_mask) else {
        return neg_errno(LinuxError::EAGAIN);
    };
    if info != 0 {
        let siginfo = make_signal_info(sig, SI_TKILL_CODE, current_tid());
        let ret = write_user_value(process, info, &siginfo);
        if ret != 0 {
            return ret;
        }
    }
    sig as isize
}

fn validate_signal_target(sig: i32) -> Result<(), LinuxError> {
    if sig < 0 || sig > 64 {
        return Err(LinuxError::EINVAL);
    }
    Ok(())
}

fn sys_kill(process: &UserProcess, pid: i32, sig: i32) -> isize {
    if let Err(err) = validate_signal_target(sig) {
        return neg_errno(err);
    }
    let deliver_process_signal = |target_pid: i32| -> isize {
        let entries = user_thread_entries_by_pid(target_pid);
        if entries.is_empty() {
            return neg_errno(LinuxError::ESRCH);
        }
        if sig == 0 {
            return 0;
        }
        if sig == general::SIGKILL as i32 {
            entries[0].process.request_exit_group(128 + sig);
            for entry in &entries {
                if let Err(err) = deliver_user_signal(entry, sig) {
                    return neg_errno(err);
                }
            }
            return 0;
        }
        deliver_user_signal(&entries[0], sig)
            .map(|()| 0)
            .unwrap_or_else(neg_errno)
    };
    if pid == current_tid() {
        if sig == 0 {
            return 0;
        }
        let Some(entry) = user_thread_entry_by_tid(pid) else {
            return neg_errno(LinuxError::ESRCH);
        };
        return deliver_user_signal(&entry, sig)
            .map(|()| 0)
            .unwrap_or_else(neg_errno);
    }
    if pid == 0 || pid == process.pid() {
        return deliver_process_signal(process.pid());
    }
    if pid > 0 {
        return deliver_process_signal(pid);
    }
    neg_errno(LinuxError::ESRCH)
}

fn sys_tkill(process: &UserProcess, tid: i32, sig: i32) -> isize {
    if tid <= 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    if let Err(err) = validate_signal_target(sig) {
        return neg_errno(err);
    }
    let entry = match user_thread_entry_by_tid(tid) {
        Some(entry) => entry,
        None => return neg_errno(LinuxError::ESRCH),
    };
    if entry.process.pid() != process.pid() {
        return neg_errno(LinuxError::ESRCH);
    }
    if let Err(err) = deliver_user_signal(&entry, sig) {
        return neg_errno(err);
    }
    0
}

fn sys_tgkill(process: &UserProcess, tgid: i32, tid: i32, sig: i32) -> isize {
    if tgid <= 0 || tid <= 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let entry = match user_thread_entry_by_tid(tid) {
        Some(entry) => entry,
        None => return neg_errno(LinuxError::ESRCH),
    };
    if entry.process.pid() != process.pid() || entry.process.pid() != tgid {
        return neg_errno(LinuxError::ESRCH);
    }
    if let Err(err) = deliver_user_signal(&entry, sig) {
        return neg_errno(err);
    }
    0
}

fn read_sockaddr_in(
    process: &UserProcess,
    addr: usize,
    addrlen: usize,
) -> Result<SockAddrIn, LinuxError> {
    if addr == 0 || addrlen < size_of::<SockAddrIn>() {
        return Err(LinuxError::EINVAL);
    }
    let sockaddr = read_user_value::<SockAddrIn>(process, addr)?;
    if sockaddr.sin_family != AF_INET_NUM as u16 {
        return Err(LinuxError::EAFNOSUPPORT);
    }
    Ok(sockaddr)
}

fn write_sockaddr_in(
    process: &UserProcess,
    addr: usize,
    addrlen: usize,
    sockaddr: SockAddrIn,
) -> Result<(), LinuxError> {
    if addr != 0 {
        let ret = write_user_value(process, addr, &sockaddr);
        if ret != 0 {
            return Err(LinuxError::EFAULT);
        }
    }
    if addrlen != 0 {
        let len = size_of::<SockAddrIn>() as u32;
        let ret = write_user_value(process, addrlen, &len);
        if ret != 0 {
            return Err(LinuxError::EFAULT);
        }
    }
    Ok(())
}

fn sys_socket(process: &UserProcess, domain: usize, ty: usize, protocol: usize) -> isize {
    match process
        .fds
        .lock()
        .socket(domain as u32, ty as u32, protocol as u32)
    {
        Ok(fd) => fd as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_bind(process: &UserProcess, fd: usize, addr: usize, addrlen: usize) -> isize {
    let sockaddr = match read_sockaddr_in(process, addr, addrlen) {
        Ok(sockaddr) => sockaddr,
        Err(err) => return neg_errno(err),
    };
    match process.fds.lock().bind_socket(fd as i32, sockaddr) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_getsockname(process: &UserProcess, fd: usize, addr: usize, addrlen: usize) -> isize {
    let sockaddr = match process.fds.lock().socket_name(fd as i32) {
        Ok(sockaddr) => sockaddr,
        Err(err) => return neg_errno(err),
    };
    match write_sockaddr_in(process, addr, addrlen, sockaddr) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_setsockopt(
    process: &UserProcess,
    fd: usize,
    level: usize,
    optname: usize,
    _optval: usize,
    _optlen: usize,
) -> isize {
    if process.fds.lock().entry(fd as i32).is_err() {
        return neg_errno(LinuxError::EBADF);
    }
    if level as u32 == SOL_SOCKET_NUM && optname as u32 == SO_RCVTIMEO_NUM {
        return 0;
    }
    0
}

fn sys_sendto(
    process: &UserProcess,
    fd: usize,
    buf: usize,
    len: usize,
    _flags: usize,
    addr: usize,
    addrlen: usize,
) -> isize {
    let sockaddr = match read_sockaddr_in(process, addr, addrlen) {
        Ok(sockaddr) => sockaddr,
        Err(err) => return neg_errno(err),
    };
    let Some(src) = user_bytes(process, buf, len, false) else {
        return neg_errno(LinuxError::EFAULT);
    };
    match process.fds.lock().sendto_socket(fd as i32, src, sockaddr) {
        Ok(n) => n as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_recvfrom(
    process: &UserProcess,
    fd: usize,
    buf: usize,
    len: usize,
    _flags: usize,
    addr: usize,
    addrlen: usize,
) -> isize {
    let Some(dst) = user_bytes_mut(process, buf, len, true) else {
        return neg_errno(LinuxError::EFAULT);
    };
    let (n, sockaddr) = match process.fds.lock().recvfrom_socket(fd as i32, dst) {
        Ok(result) => result,
        Err(err) => return neg_errno(err),
    };
    match write_sockaddr_in(process, addr, addrlen, sockaddr) {
        Ok(()) => n as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_listen(process: &UserProcess, fd: usize, _backlog: usize) -> isize {
    match process.fds.lock().listen_socket(fd as i32) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_connect(process: &UserProcess, fd: usize, addr: usize, addrlen: usize) -> isize {
    let sockaddr = match read_sockaddr_in(process, addr, addrlen) {
        Ok(sockaddr) => sockaddr,
        Err(err) => return neg_errno(err),
    };
    match process.fds.lock().connect_socket(fd as i32, sockaddr) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_accept4(
    process: &UserProcess,
    fd: usize,
    addr: usize,
    addrlen: usize,
    flags: usize,
) -> isize {
    let (new_fd, sockaddr) = match process.fds.lock().accept_socket(fd as i32, flags as u32) {
        Ok(result) => result,
        Err(err) => return neg_errno(err),
    };
    match write_sockaddr_in(process, addr, addrlen, sockaddr) {
        Ok(()) => new_fd as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_prlimit64(
    process: &UserProcess,
    pid: i32,
    resource: u32,
    new_limit: usize,
    old_limit: usize,
) -> isize {
    if pid != 0 && pid != current_tid() {
        return neg_errno(LinuxError::ESRCH);
    }

    if old_limit != 0 {
        let current = process.get_rlimit(resource);
        let ret = write_user_value(process, old_limit, &current);
        if ret != 0 {
            return ret;
        }
    }

    if new_limit != 0 {
        let limit = match read_user_value::<UserRlimit>(process, new_limit) {
            Ok(limit) => limit,
            Err(err) => return neg_errno(err),
        };
        if limit.rlim_cur > limit.rlim_max {
            return neg_errno(LinuxError::EINVAL);
        }
        process.set_rlimit(resource, limit);
        if resource == RLIMIT_NOFILE_RESOURCE {
            process
                .fds
                .lock()
                .set_limit(cmp::min(limit.rlim_cur, usize::MAX as u64) as usize);
        }
    }

    0
}

fn sys_exit(process: &UserProcess, tf: &TrapFrame, code: i32) -> ! {
    let _ = tf;
    user_trace!(
        "user-exit: tid={} code={code} sp={:#x} tp={:#x} ra={:#x} pc={:#x}",
        current_tid(),
        tf.regs.sp,
        tf.regs.tp,
        tf.regs.ra,
        user_pc(tf),
    );
    terminate_current_thread(process, code)
}

fn sys_exit_group(process: &UserProcess, tf: &TrapFrame, code: i32) -> ! {
    let _ = tf;
    user_trace!(
        "user-exit-group: tid={} code={code} sp={:#x} tp={:#x} ra={:#x} pc={:#x}",
        current_tid(),
        tf.regs.sp,
        tf.regs.tp,
        tf.regs.ra,
        user_pc(tf),
    );
    process.request_exit_group(code);
    terminate_current_thread(process, code)
}

fn with_readable_slice(
    process: &UserProcess,
    ptr: usize,
    len: usize,
    f: impl FnOnce(&[u8]) -> Result<usize, LinuxError>,
) -> isize {
    let Some(slice) = user_bytes(process, ptr, len, false) else {
        return neg_errno(LinuxError::EFAULT);
    };
    match f(slice) {
        Ok(v) => v as isize,
        Err(err) => neg_errno(err),
    }
}

fn with_writable_slice(
    process: &UserProcess,
    ptr: usize,
    len: usize,
    f: impl FnOnce(&mut [u8]) -> Result<usize, LinuxError>,
) -> isize {
    let Some(slice) = user_bytes_mut(process, ptr, len, true) else {
        return neg_errno(LinuxError::EFAULT);
    };
    match f(slice) {
        Ok(v) => v as isize,
        Err(err) => neg_errno(err),
    }
}

fn user_bytes<'a>(process: &UserProcess, ptr: usize, len: usize, write: bool) -> Option<&'a [u8]> {
    if len == 0 {
        return Some(&[]);
    }
    let flags = if write {
        MappingFlags::READ | MappingFlags::WRITE
    } else {
        MappingFlags::READ
    };
    if !process
        .aspace
        .lock()
        .can_access_range(VirtAddr::from(ptr), len, flags)
    {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(ptr as *const u8, len) })
}

fn user_bytes_mut<'a>(
    process: &UserProcess,
    ptr: usize,
    len: usize,
    write: bool,
) -> Option<&'a mut [u8]> {
    if len == 0 {
        return Some(&mut []);
    }
    let flags = if write {
        MappingFlags::READ | MappingFlags::WRITE
    } else {
        MappingFlags::READ
    };
    if !process
        .aspace
        .lock()
        .can_access_range(VirtAddr::from(ptr), len, flags)
    {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, len) })
}

fn write_user_value<T: Copy>(process: &UserProcess, ptr: usize, value: &T) -> isize {
    let Some(dst) = user_bytes_mut(process, ptr, size_of::<T>(), true) else {
        return neg_errno(LinuxError::EFAULT);
    };
    unsafe {
        ptr::copy_nonoverlapping(
            value as *const T as *const u8,
            dst.as_mut_ptr(),
            size_of::<T>(),
        );
    }
    0
}

fn read_user_value<T: Copy>(process: &UserProcess, ptr: usize) -> Result<T, LinuxError> {
    let Some(src) = user_bytes(process, ptr, size_of::<T>(), false) else {
        return Err(LinuxError::EFAULT);
    };
    Ok(unsafe { ptr::read_unaligned(src.as_ptr() as *const T) })
}

fn read_cstr(process: &UserProcess, ptr: usize) -> Result<String, LinuxError> {
    if ptr == 0 {
        return Err(LinuxError::EFAULT);
    }
    if !process
        .aspace
        .lock()
        .can_access_range(VirtAddr::from(ptr), 1, MappingFlags::READ)
    {
        return Err(LinuxError::EFAULT);
    }
    unsafe { CStr::from_ptr(ptr as *const c_char) }
        .to_str()
        .map(|s| s.to_string())
        .map_err(|_| LinuxError::EINVAL)
}

fn read_user_word(process: &UserProcess, ptr: usize) -> Result<usize, LinuxError> {
    let Some(bytes) = user_bytes(process, ptr, size_of::<usize>(), false) else {
        return Err(LinuxError::EFAULT);
    };
    let mut raw = [0u8; size_of::<usize>()];
    raw.copy_from_slice(bytes);
    Ok(usize::from_ne_bytes(raw))
}

fn read_execve_argv(
    process: &UserProcess,
    argv_ptr: usize,
    default_argv0: &str,
) -> Result<Vec<String>, LinuxError> {
    const MAX_ARGC: usize = 256;

    if argv_ptr == 0 {
        return Ok(vec![default_argv0.into()]);
    }

    let mut argv = Vec::new();
    for idx in 0..MAX_ARGC {
        let item_ptr = read_user_word(process, argv_ptr + idx * size_of::<usize>())?;
        if item_ptr == 0 {
            break;
        }
        argv.push(read_cstr(process, item_ptr)?);
    }
    if argv.is_empty() {
        argv.push(default_argv0.into());
    }
    Ok(argv)
}

fn current_cwd() -> String {
    axfs::api::current_dir().unwrap_or_else(|_| "/".into())
}

fn resolve_host_path(cwd: String, path: &str) -> Result<String, String> {
    normalize_path(cwd.as_str(), path).ok_or_else(|| format!("invalid path: {path}"))
}

fn derive_exec_root_from_path(path: &str) -> String {
    if path == "/musl" || path.starts_with("/musl/") {
        return "/musl".into();
    }
    if path == "/glibc" || path.starts_with("/glibc/") {
        return "/glibc".into();
    }
    if path.starts_with(TESTSUITE_STAGE_ROOT) {
        let Some(rest) = path.strip_prefix(TESTSUITE_STAGE_ROOT) else {
            return "/".into();
        };
        if rest == "/musl" || rest.starts_with("/musl/") {
            return "/musl".into();
        }
        if rest == "/glibc" || rest.starts_with("/glibc/") {
            return "/glibc".into();
        }
    }
    "/".into()
}

fn resolve_runtime_support_file(exec_root: &str, path: &str) -> Result<String, String> {
    let candidates = if path.starts_with('/') {
        runtime_absolute_path_candidates(exec_root, path)
    } else if !path.contains('/') {
        runtime_library_name_candidates(exec_root, path)
    } else {
        vec![normalize_path("/", path).ok_or_else(|| format!("invalid path: {path}"))?]
    };
    candidates
        .into_iter()
        .find(|candidate| matches!(axfs::api::metadata(candidate), Ok(meta) if meta.is_file()))
        .ok_or_else(|| format!("runtime support file not found: {path}"))
}

fn runtime_absolute_path_candidates(exec_root: &str, path: &str) -> Vec<String> {
    let Some(normalized) = normalize_path("/", path) else {
        return Vec::new();
    };
    let mut candidates = vec![normalized.clone()];
    for root in runtime_root_candidates(exec_root, normalized.as_str()) {
        if normalized == "/lib" || normalized.starts_with("/lib/") {
            push_runtime_candidate(
                &mut candidates,
                join_runtime_root(root.as_str(), normalized.as_str()),
            );
            if normalized == "/lib" {
                push_runtime_candidate(&mut candidates, join_runtime_root(root.as_str(), "/lib64"));
            } else if let Some(suffix) = normalized.strip_prefix("/lib/") {
                push_runtime_candidate(
                    &mut candidates,
                    join_runtime_root(root.as_str(), format!("/lib64/{suffix}").as_str()),
                );
                push_multiarch_runtime_aliases(&mut candidates, root.as_str(), suffix);
            }
        } else if normalized == "/lib64" || normalized.starts_with("/lib64/") {
            push_runtime_candidate(
                &mut candidates,
                join_runtime_root(root.as_str(), normalized.as_str()),
            );
            if normalized == "/lib64" {
                push_runtime_candidate(&mut candidates, join_runtime_root(root.as_str(), "/lib"));
            } else if let Some(suffix) = normalized.strip_prefix("/lib64/") {
                push_runtime_candidate(
                    &mut candidates,
                    join_runtime_root(root.as_str(), format!("/lib/{suffix}").as_str()),
                );
                push_multiarch_runtime_aliases(&mut candidates, root.as_str(), suffix);
            }
        } else if normalized == "/usr/lib" || normalized.starts_with("/usr/lib/") {
            push_runtime_candidate(
                &mut candidates,
                join_runtime_root(root.as_str(), normalized.as_str()),
            );
            if normalized == "/usr/lib" {
                push_runtime_candidate(&mut candidates, join_runtime_root(root.as_str(), "/lib"));
                push_runtime_candidate(&mut candidates, join_runtime_root(root.as_str(), "/lib64"));
            } else if let Some(suffix) = normalized.strip_prefix("/usr/lib/") {
                push_runtime_candidate(
                    &mut candidates,
                    join_runtime_root(root.as_str(), format!("/lib/{suffix}").as_str()),
                );
                push_runtime_candidate(
                    &mut candidates,
                    join_runtime_root(root.as_str(), format!("/lib64/{suffix}").as_str()),
                );
                push_multiarch_runtime_aliases(&mut candidates, root.as_str(), suffix);
            }
        } else if normalized == "/usr/lib64" || normalized.starts_with("/usr/lib64/") {
            push_runtime_candidate(
                &mut candidates,
                join_runtime_root(root.as_str(), normalized.as_str()),
            );
            if normalized == "/usr/lib64" {
                push_runtime_candidate(&mut candidates, join_runtime_root(root.as_str(), "/lib64"));
                push_runtime_candidate(&mut candidates, join_runtime_root(root.as_str(), "/lib"));
            } else if let Some(suffix) = normalized.strip_prefix("/usr/lib64/") {
                push_runtime_candidate(
                    &mut candidates,
                    join_runtime_root(root.as_str(), format!("/lib64/{suffix}").as_str()),
                );
                push_runtime_candidate(
                    &mut candidates,
                    join_runtime_root(root.as_str(), format!("/lib/{suffix}").as_str()),
                );
                push_multiarch_runtime_aliases(&mut candidates, root.as_str(), suffix);
            }
        } else if normalized.starts_with("/etc/ld") {
            push_runtime_candidate(
                &mut candidates,
                join_runtime_root(root.as_str(), normalized.as_str()),
            );
        }
        push_musl_loader_aliases(&mut candidates, root.as_str(), normalized.as_str());
    }
    candidates
}

fn runtime_library_name_candidates(exec_root: &str, name: &str) -> Vec<String> {
    if name.contains('/') || !looks_like_runtime_library_name(name) {
        return Vec::new();
    }
    let mut candidates = Vec::new();
    for root in runtime_root_candidates(exec_root, name) {
        push_runtime_candidate(
            &mut candidates,
            join_runtime_root(root.as_str(), format!("/lib/{name}").as_str()),
        );
        push_runtime_candidate(
            &mut candidates,
            join_runtime_root(root.as_str(), format!("/lib64/{name}").as_str()),
        );
        push_runtime_candidate(
            &mut candidates,
            join_runtime_root(root.as_str(), format!("/usr/lib/{name}").as_str()),
        );
        push_runtime_candidate(
            &mut candidates,
            join_runtime_root(root.as_str(), format!("/usr/lib64/{name}").as_str()),
        );
        push_musl_loader_aliases(&mut candidates, root.as_str(), name);
    }
    candidates
}

fn runtime_root_candidates(exec_root: &str, path: &str) -> Vec<String> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let mut roots = Vec::new();
    let mut push = |root: &str| {
        if !roots.iter().any(|item| item == root) {
            roots.push(root.to_string());
        }
    };
    if is_glibc_runtime_name(name) {
        push("/glibc");
    }
    if is_musl_runtime_name(name) {
        push("/musl");
    }
    if exec_root != "/" {
        push(exec_root);
    }
    push("/musl");
    push("/glibc");
    roots
}

fn join_runtime_root(root: &str, path: &str) -> Option<String> {
    let normalized = normalize_path("/", path)?;
    if root == "/" {
        return Some(normalized);
    }
    let rel = normalized.trim_start_matches('/');
    Some(if rel.is_empty() {
        root.to_string()
    } else {
        format!("{}/{}", root.trim_end_matches('/'), rel)
    })
}

fn push_runtime_candidate(candidates: &mut Vec<String>, candidate: Option<String>) {
    let Some(candidate) = candidate else {
        return;
    };
    if !candidates.iter().any(|item| item == &candidate) {
        candidates.push(candidate);
    }
}

fn push_multiarch_runtime_aliases(candidates: &mut Vec<String>, root: &str, suffix: &str) {
    let Some((_, tail)) = suffix.split_once('/') else {
        return;
    };
    if tail.is_empty() {
        return;
    }
    push_runtime_candidate(
        candidates,
        join_runtime_root(root, format!("/lib/{tail}").as_str()),
    );
    push_runtime_candidate(
        candidates,
        join_runtime_root(root, format!("/lib64/{tail}").as_str()),
    );
}

fn push_musl_loader_aliases(candidates: &mut Vec<String>, root: &str, path: &str) {
    let name = path.rsplit('/').next().unwrap_or(path);
    if !name.starts_with("ld-musl-") || !name.ends_with(".so.1") {
        return;
    }
    push_runtime_candidate(candidates, join_runtime_root(root, "/lib/libc.so"));
    push_runtime_candidate(candidates, join_runtime_root(root, "/lib64/libc.so"));
}

fn is_glibc_runtime_name(name: &str) -> bool {
    name.starts_with("ld-linux-") || name.ends_with(".so.6")
}

fn is_musl_runtime_name(name: &str) -> bool {
    name.starts_with("ld-musl-") || name == "libc.so"
}

fn looks_like_runtime_library_name(name: &str) -> bool {
    name.starts_with("ld-") || name.contains(".so")
}

fn normalize_path(base: &str, path: &str) -> Option<String> {
    let mut parts = Vec::new();
    let input = if path.starts_with('/') {
        path.to_string()
    } else if base == "/" {
        format!("/{path}")
    } else {
        format!("{}/{}", base.trim_end_matches('/'), path)
    };
    for part in input.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    let mut normalized = String::from("/");
    normalized.push_str(&parts.join("/"));
    Some(normalized)
}

fn file_attr_to_stat(attr: &FileAttr, path: Option<&str>) -> general::stat {
    let st_mode = file_type_mode(attr.file_type()) | attr.perm().bits() as u32;
    let mut st: general::stat = unsafe { core::mem::zeroed() };
    st.st_dev = 1;
    st.st_ino = path_inode(path);
    st.st_mode = st_mode;
    st.st_nlink = 1;
    st.st_size = attr.size() as _;
    st.st_blksize = 512;
    st.st_blocks = attr.blocks() as _;
    apply_path_times_to_stat(&mut st, path);
    st
}

fn zero_timespec() -> general::timespec {
    general::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    }
}

fn realtime_timespec() -> general::timespec {
    let now = axhal::time::wall_time();
    general::timespec {
        tv_sec: now.as_secs() as _,
        tv_nsec: now.subsec_nanos() as _,
    }
}

fn apply_stat_times(st: &mut general::stat, times: FileTimes) {
    st.st_atime = times.atime.tv_sec as _;
    st.st_atime_nsec = times.atime.tv_nsec as _;
    st.st_mtime = times.mtime.tv_sec as _;
    st.st_mtime_nsec = times.mtime.tv_nsec as _;
    st.st_ctime = times.ctime.tv_sec as _;
    st.st_ctime_nsec = times.ctime.tv_nsec as _;
}

fn read_utime_updates(
    process: &UserProcess,
    times: usize,
) -> Result<(UtimeUpdate, UtimeUpdate), LinuxError> {
    if times == 0 {
        return Ok((utime_now_update(), utime_now_update()));
    }
    let atime = read_user_value::<general::timespec>(process, times)?;
    let mtime =
        read_user_value::<general::timespec>(process, times + size_of::<general::timespec>())?;
    Ok((parse_utime_update(atime)?, parse_utime_update(mtime)?))
}

fn parse_utime_update(ts: general::timespec) -> Result<UtimeUpdate, LinuxError> {
    match ts.tv_nsec {
        UTIME_NOW_NSEC => Ok(utime_now_update()),
        UTIME_OMIT_NSEC => Ok(UtimeUpdate {
            time: None,
            omit: true,
        }),
        nsec if (0..1_000_000_000).contains(&nsec) && ts.tv_sec >= 0 => Ok(UtimeUpdate {
            time: Some(ts),
            omit: false,
        }),
        _ => Err(LinuxError::EINVAL),
    }
}

fn utime_now_update() -> UtimeUpdate {
    UtimeUpdate {
        time: Some(realtime_timespec()),
        omit: false,
    }
}

fn apply_utime_updates(
    mut current: FileTimes,
    atime: UtimeUpdate,
    mtime: UtimeUpdate,
) -> FileTimes {
    let changed = !atime.omit || !mtime.omit;
    if let Some(time) = atime.time {
        current.atime = time;
    }
    if let Some(time) = mtime.time {
        current.mtime = time;
    }
    if changed {
        current.ctime = realtime_timespec();
    }
    current
}

fn path_inode(path: Option<&str>) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let Some(path) = path else {
        return 1;
    };
    let mut hash = FNV_OFFSET;
    for &byte in path.as_bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash.max(1)
}

fn file_type_mode(ty: FileType) -> u32 {
    match ty {
        FileType::Dir => ST_MODE_DIR,
        FileType::CharDevice => ST_MODE_CHR,
        FileType::SymLink => ST_MODE_LNK,
        _ => ST_MODE_FILE,
    }
}

fn flags_from_ph(flags: PhFlags) -> MappingFlags {
    let mut out = MappingFlags::USER;
    if flags.is_read() || flags.is_execute() {
        out |= MappingFlags::READ;
    }
    if flags.is_write() {
        out |= MappingFlags::WRITE;
    }
    if flags.is_execute() {
        out |= MappingFlags::EXECUTE;
    }
    out
}

fn mmap_prot_to_flags(prot: u32) -> MappingFlags {
    let mut flags = MappingFlags::USER;
    if prot & general::PROT_READ != 0 {
        flags |= MappingFlags::READ;
    }
    if prot & general::PROT_WRITE != 0 {
        flags |= MappingFlags::READ | MappingFlags::WRITE;
    }
    if prot & general::PROT_EXEC != 0 {
        flags |= MappingFlags::READ | MappingFlags::EXECUTE;
    }
    flags
}

fn user_mapping_flags(read: bool, write: bool, exec: bool) -> MappingFlags {
    let mut flags = MappingFlags::USER;
    if read {
        flags |= MappingFlags::READ;
    }
    if write {
        flags |= MappingFlags::WRITE;
    }
    if exec {
        flags |= MappingFlags::EXECUTE;
    }
    flags
}

fn align_down(value: usize, align: usize) -> usize {
    value & !(align - 1)
}

fn align_up(value: usize, align: usize) -> usize {
    if value == 0 {
        0
    } else {
        align_down(value + align - 1, align)
    }
}

fn default_rlimit(resource: u32) -> UserRlimit {
    match resource {
        RLIMIT_STACK_RESOURCE => UserRlimit {
            rlim_cur: USER_STACK_SIZE as u64,
            rlim_max: USER_STACK_SIZE as u64,
        },
        RLIMIT_NOFILE_RESOURCE => UserRlimit {
            rlim_cur: DEFAULT_NOFILE_LIMIT,
            rlim_max: DEFAULT_NOFILE_LIMIT,
        },
        _ => UserRlimit {
            rlim_cur: u64::MAX,
            rlim_max: u64::MAX,
        },
    }
}

fn neg_errno(err: LinuxError) -> isize {
    -(err.code() as isize)
}

fn str_err(err: &'static str) -> String {
    err.into()
}

impl FdEntry {
    fn duplicate_for_fork(&self) -> Result<Self, LinuxError> {
        match self {
            Self::Stdin => Ok(Self::Stdin),
            Self::Stdout => Ok(Self::Stdout),
            Self::Stderr => Ok(Self::Stderr),
            Self::DevNull => Ok(Self::DevNull),
            Self::File(file) => Ok(Self::File(file.clone())),
            Self::Directory(dir) => Ok(Self::Directory(dir.clone())),
            Self::Pipe(pipe) => Ok(Self::Pipe(pipe.clone())),
            Self::Socket(socket) => Ok(Self::Socket(socket.clone())),
        }
    }
}

impl FdTable {
    fn new() -> Self {
        Self {
            entries: vec![
                Some(FdEntry::Stdin),
                Some(FdEntry::Stdout),
                Some(FdEntry::Stderr),
            ],
            path_times: BTreeMap::new(),
            next_socket_port: 40000,
            limit: DEFAULT_NOFILE_LIMIT as usize,
        }
    }

    fn fork_copy(&self) -> Result<Self, LinuxError> {
        let mut entries = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            entries.push(match entry {
                Some(entry) => Some(entry.duplicate_for_fork()?),
                None => None,
            });
        }
        Ok(Self {
            entries,
            path_times: self.path_times.clone(),
            next_socket_port: self.next_socket_port,
            limit: self.limit,
        })
    }

    fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
    }

    fn is_stdio(&self, fd: i32) -> bool {
        matches!(fd, 0..=2)
    }

    fn poll_state(&self, fd: i32) -> Result<PollState, LinuxError> {
        let entry = self.entry(fd)?;
        match entry {
            FdEntry::Stdin => Ok(PollState {
                readable: false,
                writable: false,
            }),
            FdEntry::Stdout | FdEntry::Stderr => Ok(PollState {
                readable: false,
                writable: true,
            }),
            FdEntry::DevNull | FdEntry::File(_) => Ok(PollState {
                readable: true,
                writable: true,
            }),
            FdEntry::Directory(_) => Ok(PollState {
                readable: true,
                writable: false,
            }),
            FdEntry::Pipe(pipe) => Ok(pipe.poll()),
            #[cfg(feature = "net")]
            FdEntry::Socket(socket) => socket_poll_state(socket),
        }
    }

    fn poll(&self, fd: i32, mode: SelectMode) -> bool {
        let Ok(state) = self.poll_state(fd) else {
            return matches!(mode, SelectMode::Except);
        };
        match mode {
            SelectMode::Read => match entry {
                FdEntry::Stdin => false,
                FdEntry::Stdout | FdEntry::Stderr => false,
                FdEntry::DevNull | FdEntry::File(_) | FdEntry::Directory(_) => true,
                FdEntry::Pipe(pipe) => pipe.poll().readable,
                FdEntry::Socket(socket) => !socket.recv_queue.is_empty() || socket.listening,
            },
            SelectMode::Write => match entry {
                FdEntry::Stdin => false,
                FdEntry::Stdout | FdEntry::Stderr | FdEntry::DevNull => true,
                FdEntry::File(_) => true,
                FdEntry::Directory(_) => false,
                FdEntry::Pipe(pipe) => pipe.poll().writable,
                FdEntry::Socket(_) => true,
            },
            SelectMode::Except => false,
        }
    }

    fn read(&mut self, fd: i32, dst: &mut [u8]) -> Result<usize, LinuxError> {
        match self.entry_mut(fd)? {
            FdEntry::Stdin => Ok(0),
            FdEntry::DevNull => Ok(0),
            FdEntry::File(file) => file.file.read(dst).map_err(LinuxError::from),
            FdEntry::Directory(_) => Err(LinuxError::EISDIR),
            FdEntry::Pipe(pipe) => pipe.read(dst),
            FdEntry::Socket(socket) => {
                if socket.recv_queue.is_empty() {
                    return Err(LinuxError::EAGAIN);
                }
                let data = socket.recv_queue.remove(0);
                let n = cmp::min(dst.len(), data.len());
                dst[..n].copy_from_slice(&data[..n]);
                Ok(n)
            }
            _ => Err(LinuxError::EBADF),
        }
    }

    fn write(&mut self, fd: i32, src: &[u8]) -> Result<usize, LinuxError> {
        match self.entry_mut(fd)? {
            FdEntry::Stdout | FdEntry::Stderr => {
                axhal::console::write_bytes(src);
                Ok(src.len())
            }
            FdEntry::DevNull => Ok(src.len()),
            FdEntry::File(file) => file.file.write(src).map_err(LinuxError::from),
            FdEntry::Pipe(pipe) => pipe.write(src),
            FdEntry::Socket(_) => Ok(src.len()),
            _ => Err(LinuxError::EBADF),
        }
    }

    fn open(
        &mut self,
        process: &UserProcess,
        dirfd: i32,
        path: &str,
        flags: u32,
    ) -> Result<i32, LinuxError> {
        let entry = open_fd_entry(process, self, dirfd, path, flags)?;
        let fd = self.insert(entry, process.nofile_limit())?;
        if flags & general::O_CLOEXEC != 0 {
            self.set_fd_flags(fd, FD_CLOEXEC_FLAG)?;
        }
        Ok(fd)
    }

    fn socket(&mut self, domain: u32, ty: u32, protocol: u32) -> Result<i32, LinuxError> {
        if domain != AF_INET_NUM {
            return Err(LinuxError::EAFNOSUPPORT);
        }
        let kind = match ty & SOCK_TYPE_MASK {
            SOCK_DGRAM_NUM if protocol == 0 || protocol == IPPROTO_UDP_NUM => SocketKind::Datagram,
            SOCK_STREAM_NUM if protocol == 0 || protocol == IPPROTO_TCP_NUM => SocketKind::Stream,
            _ => return Err(LinuxError::EPROTONOSUPPORT),
        };
        let entry = FdEntry::Socket(SocketEntry {
            kind,
            cloexec: ty & SOCK_CLOEXEC_FLAG != 0,
            nonblock: ty & SOCK_NONBLOCK_FLAG != 0,
            local_port: 0,
            recv_queue: Vec::new(),
            listening: false,
            pending_stream: false,
        });
        self.insert(entry)
    }

    fn bind_socket(&mut self, fd: i32, addr: SockAddrIn) -> Result<(), LinuxError> {
        if addr.sin_family != AF_INET_NUM as u16 {
            return Err(LinuxError::EAFNOSUPPORT);
        }
        let port = u16::from_be(addr.sin_port);
        let assigned = if port == 0 {
            let port = self.next_socket_port;
            self.next_socket_port = self.next_socket_port.saturating_add(1).max(40000);
            port
        } else {
            port
        };
        let FdEntry::Socket(socket) = self.entry_mut(fd)? else {
            return Err(LinuxError::ENOTSOCK);
        };
        socket.local_port = assigned;
        Ok(())
    }

    fn socket_name(&self, fd: i32) -> Result<SockAddrIn, LinuxError> {
        let FdEntry::Socket(socket) = self.entry(fd)? else {
            return Err(LinuxError::ENOTSOCK);
        };
        Ok(SockAddrIn {
            sin_family: AF_INET_NUM as u16,
            sin_port: socket.local_port.to_be(),
            sin_addr: 0,
            sin_zero: [0; 8],
        })
    }

    fn listen_socket(&mut self, fd: i32) -> Result<(), LinuxError> {
        let FdEntry::Socket(socket) = self.entry_mut(fd)? else {
            return Err(LinuxError::ENOTSOCK);
        };
        if socket.kind != SocketKind::Stream {
            return Err(LinuxError::EOPNOTSUPP);
        }
        socket.listening = true;
        Ok(())
    }

    fn connect_socket(&mut self, fd: i32, addr: SockAddrIn) -> Result<(), LinuxError> {
        let port = u16::from_be(addr.sin_port);
        let FdEntry::Socket(client) = self.entry(fd)? else {
            return Err(LinuxError::ENOTSOCK);
        };
        if client.kind != SocketKind::Stream {
            return Err(LinuxError::EOPNOTSUPP);
        }
        let listener = self.entries.iter_mut().find_map(|entry| match entry {
            Some(FdEntry::Socket(socket))
                if socket.kind == SocketKind::Stream
                    && socket.listening
                    && socket.local_port == port =>
            {
                Some(socket)
            }
            _ => None,
        });
        let Some(listener) = listener else {
            return Err(LinuxError::ECONNREFUSED);
        };
        listener.pending_stream = true;
        Ok(())
    }

    fn accept_socket(&mut self, fd: i32, flags: u32) -> Result<(i32, SockAddrIn), LinuxError> {
        let FdEntry::Socket(listener) = self.entry_mut(fd)? else {
            return Err(LinuxError::ENOTSOCK);
        };
        if listener.kind != SocketKind::Stream || !listener.listening {
            return Err(LinuxError::EINVAL);
        }
        listener.pending_stream = false;
        let addr = SockAddrIn {
            sin_family: AF_INET_NUM as u16,
            sin_port: listener.local_port.to_be(),
            sin_addr: u32::from_be(0x7f00_0001),
            sin_zero: [0; 8],
        };
        let accepted = SocketEntry {
            kind: SocketKind::Stream,
            cloexec: flags & SOCK_CLOEXEC_FLAG != 0,
            nonblock: flags & SOCK_NONBLOCK_FLAG != 0,
            local_port: listener.local_port,
            recv_queue: Vec::new(),
            listening: false,
            pending_stream: false,
        };
        let new_fd = self.insert(FdEntry::Socket(accepted))?;
        Ok((new_fd, addr))
    }

    fn sendto_socket(
        &mut self,
        fd: i32,
        data: &[u8],
        addr: SockAddrIn,
    ) -> Result<usize, LinuxError> {
        let FdEntry::Socket(sender) = self.entry(fd)? else {
            return Err(LinuxError::ENOTSOCK);
        };
        if sender.kind != SocketKind::Datagram {
            return Err(LinuxError::EOPNOTSUPP);
        }
        let port = u16::from_be(addr.sin_port);
        let receiver = self.entries.iter_mut().find_map(|entry| match entry {
            Some(FdEntry::Socket(socket))
                if socket.kind == SocketKind::Datagram && socket.local_port == port =>
            {
                Some(socket)
            }
            _ => None,
        });
        let Some(receiver) = receiver else {
            return Err(LinuxError::ECONNREFUSED);
        };
        receiver.recv_queue.push(data.to_vec());
        Ok(data.len())
    }

    fn recvfrom_socket(
        &mut self,
        fd: i32,
        dst: &mut [u8],
    ) -> Result<(usize, SockAddrIn), LinuxError> {
        let FdEntry::Socket(socket) = self.entry_mut(fd)? else {
            return Err(LinuxError::ENOTSOCK);
        };
        if socket.kind != SocketKind::Datagram {
            return Err(LinuxError::EOPNOTSUPP);
        }
        if socket.recv_queue.is_empty() {
            return Err(LinuxError::EAGAIN);
        }
        let data = socket.recv_queue.remove(0);
        let n = cmp::min(dst.len(), data.len());
        dst[..n].copy_from_slice(&data[..n]);
        Ok((
            n,
            SockAddrIn {
                sin_family: AF_INET_NUM as u16,
                sin_port: socket.local_port.to_be(),
                sin_addr: u32::from_be(0x7f00_0001),
                sin_zero: [0; 8],
            },
        ))
    }

    fn mkdirat(&mut self, process: &UserProcess, dirfd: i32, path: &str) -> Result<(), LinuxError> {
        if path.starts_with('/') || dirfd == general::AT_FDCWD {
            let cwd = process.cwd();
            let abs_path = resolve_host_path(cwd, path).map_err(|_| LinuxError::EINVAL)?;
            return directory_create_dir(abs_path.as_str());
        }
        let FdEntry::Directory(dir) = self.entry(dirfd)? else {
            return Err(LinuxError::ENOTDIR);
        };
        dir.dir.create_dir(path).map_err(LinuxError::from)
    }

    fn unlinkat(
        &mut self,
        process: &UserProcess,
        dirfd: i32,
        path: &str,
        flags: u32,
    ) -> Result<(), LinuxError> {
        let remove_dir = flags & general::AT_REMOVEDIR != 0;
        if path.starts_with('/') || dirfd == general::AT_FDCWD {
            let cwd = process.cwd();
            let abs_path = if let Some(shm_path) = dev_shm_host_path(path) {
                shm_path
            } else {
                resolve_host_path(cwd, path).map_err(|_| LinuxError::EINVAL)?
            };
            let result = if remove_dir {
                directory_remove_dir(abs_path.as_str())
            } else {
                directory_remove_file(abs_path.as_str())
            };
            if result.is_ok() {
                self.path_times.remove(abs_path.as_str());
            }
            return result;
        }
        let FdEntry::Directory(dir) = self.entry(dirfd)? else {
            return Err(LinuxError::ENOTDIR);
        };
        if remove_dir {
            dir.dir.remove_dir(path).map_err(LinuxError::from)
        } else {
            dir.dir.remove_file(path).map_err(LinuxError::from)
        }
    }

    fn close(&mut self, fd: i32) -> Result<(), LinuxError> {
        if !(0..self.entries.len() as i32).contains(&fd) || self.entries[fd as usize].is_none() {
            return Err(LinuxError::EBADF);
        }
        self.entries[fd as usize] = None;
        if let Some(flags) = self.fd_flags.get_mut(fd as usize) {
            *flags = 0;
        }
        Ok(())
    }

    fn close_on_exec(&mut self) {
        for (idx, entry) in self.entries.iter_mut().enumerate() {
            if self.fd_flags.get(idx).copied().unwrap_or(0) & FD_CLOEXEC_FLAG != 0 {
                *entry = None;
                if let Some(flags) = self.fd_flags.get_mut(idx) {
                    *flags = 0;
                }
            }
        }
    }

    fn close_all(&mut self) {
        self.entries.clear();
        self.fd_flags.clear();
    }

    fn stat(&mut self, fd: i32) -> Result<general::stat, LinuxError> {
        match self.entry_mut(fd)? {
            FdEntry::Stdin => Ok(stdio_stat(true)),
            FdEntry::Stdout | FdEntry::Stderr => Ok(stdio_stat(false)),
            FdEntry::DevNull => Ok(stdio_stat(false)),
            FdEntry::File(file) => {
                let mut st = file_attr_to_stat(
                    &file.file.get_attr().map_err(LinuxError::from)?,
                    Some(file.path.as_str()),
                );
                apply_stat_times(&mut st, file.times);
                Ok(st)
            }
            FdEntry::Directory(dir) => {
                let mut st = file_attr_to_stat(&dir.attr, Some(dir.path.as_str()));
                apply_stat_times(&mut st, dir.times);
                Ok(st)
            }
            FdEntry::Pipe(pipe) => Ok(pipe.stat()),
            FdEntry::Socket(_) => Ok(socket_stat()),
        }
    }

    fn stat_path(
        &mut self,
        process: &UserProcess,
        dirfd: i32,
        path: &str,
    ) -> Result<general::stat, LinuxError> {
        match open_fd_entry(process, self, dirfd, path, general::O_RDONLY) {
            Ok(FdEntry::File(file)) => {
                let mut st = file_attr_to_stat(
                    &file.file.get_attr().map_err(LinuxError::from)?,
                    Some(file.path.as_str()),
                );
                apply_stat_times(&mut st, file.times);
                Ok(st)
            }
            Ok(FdEntry::Directory(dir)) => {
                let mut st = file_attr_to_stat(&dir.attr, Some(dir.path.as_str()));
                apply_stat_times(&mut st, dir.times);
                Ok(st)
            }
            Ok(FdEntry::DevNull) => Ok(stdio_stat(false)),
            Ok(FdEntry::Socket(_)) => Ok(socket_stat()),
            Ok(_) => Err(LinuxError::EINVAL),
            Err(err) => Err(err),
        }
    }

    fn update_fd_times(
        &mut self,
        fd: i32,
        atime: UtimeUpdate,
        mtime: UtimeUpdate,
    ) -> Result<(), LinuxError> {
        let path_update = match self.entry_mut(fd)? {
            FdEntry::File(file) => {
                file.times = apply_utime_updates(file.times, atime, mtime);
                Some((file.path.clone(), file.times))
            }
            FdEntry::Directory(dir) => {
                dir.times = apply_utime_updates(dir.times, atime, mtime);
                Some((dir.path.clone(), dir.times))
            }
            FdEntry::DevNull => return Ok(()),
            _ => return Err(LinuxError::EINVAL),
        };
        if let Some((path, times)) = path_update {
            self.path_times.insert(path, times);
        }
        Ok(())
    }

    fn update_path_times(
        &mut self,
        path: &str,
        atime: UtimeUpdate,
        mtime: UtimeUpdate,
    ) -> Result<(), LinuxError> {
        let current = self.path_times.get(path).copied().unwrap_or_default();
        let updated = apply_utime_updates(current, atime, mtime);
        self.path_times.insert(path.to_string(), updated);
        for entry in &mut self.entries {
            match entry {
                Some(FdEntry::File(file)) if file.path == path => file.times = updated,
                Some(FdEntry::Directory(dir)) if dir.path == path => dir.times = updated,
                _ => {}
            }
        }
        Ok(())
    }

    fn truncate(&mut self, fd: i32, size: u64) -> Result<(), LinuxError> {
        match self.entry_mut(fd)? {
            FdEntry::File(file) => file.file.truncate(size).map_err(LinuxError::from),
            FdEntry::DevNull => Ok(()),
            _ => Err(LinuxError::EINVAL),
        }
    }

    fn sync(&mut self, fd: i32) -> Result<(), LinuxError> {
        match self.entry_mut(fd)? {
            FdEntry::File(file) => file.file.flush().map_err(LinuxError::from),
            FdEntry::Directory(_) => Ok(()),
            FdEntry::Stdin | FdEntry::Stdout | FdEntry::Stderr | FdEntry::DevNull => Ok(()),
            FdEntry::Pipe(_) => Err(LinuxError::EINVAL),
            #[cfg(feature = "net")]
            FdEntry::Socket(_) => Err(LinuxError::EINVAL),
        }
    }

    fn fcntl(
        &mut self,
        fd: i32,
        cmd: u32,
        _arg: usize,
        nofile_limit: usize,
    ) -> Result<i32, LinuxError> {
        let _ = self.entry(fd)?;
        match cmd {
            general::F_DUPFD | general::F_DUPFD_CLOEXEC => self.dup_min(fd, _arg as i32),
            general::F_GETFD => Ok(match self.entry(fd)? {
                FdEntry::Socket(socket) if socket.cloexec => general::FD_CLOEXEC as i32,
                _ => 0,
            }),
            general::F_GETFL => Ok(match self.entry(fd)? {
                FdEntry::Socket(socket) if socket.nonblock => general::O_NONBLOCK as i32,
                _ => 0,
            }),
            general::F_SETFD | general::F_SETFL => Ok(0),
            _ => Ok(0),
        }
    }

    fn ioctl(
        &mut self,
        process: &UserProcess,
        fd: i32,
        req: u32,
        arg: usize,
    ) -> Result<i32, LinuxError> {
        if req == ioctl::FIOCLEX {
            let _ = self.entry(fd)?;
            self.set_fd_flags(fd, FD_CLOEXEC_FLAG)?;
            return Ok(0);
        }
        match self.entry_mut(fd)? {
            #[cfg(feature = "net")]
            FdEntry::Socket(socket) => match req {
                ioctl::FIONBIO => {
                    let enabled = read_user_value::<c_int>(process, arg)? != 0;
                    socket.socket.set_nonblocking(enabled);
                    let flags = if enabled { general::O_NONBLOCK } else { 0 };
                    socket.status_flags.store(flags, Ordering::Release);
                    Ok(0)
                }
                _ => Err(LinuxError::ENOTTY),
            },
            FdEntry::File(file) if is_rtc_device_path(file.path.as_str()) => match req {
                ioctl::RTC_RD_TIME => {
                    let rtc = imp_system::current_rtc_time();
                    if write_user_value(process, arg, &rtc) == 0 {
                        Ok(0)
                    } else {
                        Err(LinuxError::EFAULT)
                    }
                }
                _ => Err(LinuxError::ENOTTY),
            },
            _ => Err(LinuxError::ENOTTY),
        }
    }

    fn lseek(&mut self, fd: i32, offset: i64, whence: u32) -> Result<u64, LinuxError> {
        let pos = match whence {
            general::SEEK_SET => SeekFrom::Start(offset.max(0) as u64),
            general::SEEK_CUR => SeekFrom::Current(offset),
            general::SEEK_END => SeekFrom::End(offset),
            _ => return Err(LinuxError::EINVAL),
        };
        match self.entry_mut(fd)? {
            FdEntry::File(file) => file.file.seek(pos).map_err(LinuxError::from),
            FdEntry::DevNull => Ok(0),
            FdEntry::Directory(_) => Err(LinuxError::EISDIR),
            FdEntry::Pipe(_) => Err(LinuxError::ESPIPE),
            FdEntry::Socket(_) => Err(LinuxError::ESPIPE),
            _ => Err(LinuxError::ESPIPE),
        }
    }

    fn dup(&mut self, fd: i32, nofile_limit: usize) -> Result<i32, LinuxError> {
        self.dup_min_with_flags(fd, 0, 0, nofile_limit)
    }

    fn dup_min_with_flags(
        &mut self,
        fd: i32,
        min_fd: i32,
        flags: u32,
        nofile_limit: usize,
    ) -> Result<i32, LinuxError> {
        if min_fd < 0 {
            return Err(LinuxError::EINVAL);
        }
        let entry = self.entry(fd)?.duplicate_for_fork()?;
        let newfd = self.insert_min(entry, min_fd as usize, nofile_limit)?;
        self.set_fd_flags(newfd, flags & FD_CLOEXEC_FLAG)?;
        Ok(newfd)
    }

    fn dup3(
        &mut self,
        oldfd: i32,
        newfd: i32,
        _flags: u32,
        nofile_limit: usize,
    ) -> Result<i32, LinuxError> {
        if oldfd == newfd {
            return Err(LinuxError::EINVAL);
        }
        let entry = self.entry(oldfd)?.duplicate_for_fork()?;
        if newfd < 0 || newfd as usize >= self.limit {
            return Err(LinuxError::EBADF);
        }
        let newfd = newfd as usize;
        if newfd >= nofile_limit {
            return Err(LinuxError::EBADF);
        }
        if self.entries.len() <= newfd {
            self.entries.resize_with(newfd + 1, || None);
        }
        if self.fd_flags.len() <= newfd {
            self.fd_flags.resize(newfd + 1, 0);
        }
        self.entries[newfd] = Some(entry);
        self.fd_flags[newfd] = if _flags & general::O_CLOEXEC != 0 {
            FD_CLOEXEC_FLAG
        } else {
            0
        };
        Ok(newfd as i32)
    }

    fn getdents64(&mut self, fd: i32, dst: &mut [u8]) -> Result<usize, LinuxError> {
        let entry = self.entry_mut(fd)?;
        let FdEntry::Directory(dir) = entry else {
            return Err(LinuxError::ENOTDIR);
        };
        let mut read_buf: [fops::DirEntry; 16] =
            core::array::from_fn(|_| fops::DirEntry::default());
        let count = dir.dir.read_dir(&mut read_buf).map_err(LinuxError::from)?;
        let mut written = 0usize;
        for (idx, item) in read_buf[..count].iter().enumerate() {
            let name = item.name_as_bytes();
            let reclen = align_up(
                offset_of!(general::linux_dirent64, d_name) + name.len() + 1,
                8,
            );
            if written + reclen > dst.len() {
                break;
            }
            unsafe {
                let dirent = dst.as_mut_ptr().add(written) as *mut general::linux_dirent64;
                ptr::write_unaligned(
                    dirent,
                    general::linux_dirent64 {
                        d_ino: (idx + 1) as _,
                        d_off: 0,
                        d_reclen: reclen as _,
                        d_type: dirent_type(item.entry_type()) as u8,
                        d_name: Default::default(),
                    },
                );
                let name_ptr = dst
                    .as_mut_ptr()
                    .add(written + offset_of!(general::linux_dirent64, d_name));
                ptr::copy_nonoverlapping(name.as_ptr(), name_ptr, name.len());
                *name_ptr.add(name.len()) = 0;
            }
            written += reclen;
        }
        Ok(written)
    }

    fn read_file_at(&mut self, fd: i32, offset: u64, len: usize) -> Result<Vec<u8>, LinuxError> {
        let FdEntry::File(file) = self.entry_mut(fd)? else {
            return Err(LinuxError::EBADF);
        };
        let mut buf = vec![0u8; len];
        let mut filled = 0usize;
        while filled < buf.len() {
            let read = file
                .file
                .read_at(offset + filled as u64, &mut buf[filled..])
                .map_err(LinuxError::from)?;
            if read == 0 {
                break;
            }
            filled += read;
        }
        buf.truncate(filled);
        Ok(buf)
    }

    fn insert(&mut self, entry: FdEntry, nofile_limit: usize) -> Result<i32, LinuxError> {
        self.insert_min(entry, 0, nofile_limit)
    }

    fn insert_min(&mut self, entry: FdEntry, min_fd: usize) -> Result<i32, LinuxError> {
        if min_fd >= self.limit {
            return Err(LinuxError::EMFILE);
        }
        if self.entries.len() < min_fd {
            self.entries.resize_with(min_fd, || None);
        }
        if let Some((idx, slot)) = self
            .entries
            .iter_mut()
            .enumerate()
            .take(limit)
            .skip(min_fd)
            .take(self.limit.saturating_sub(min_fd))
            .find(|(_, slot)| slot.is_none())
        {
            *slot = Some(entry);
            if self.fd_flags.len() <= idx {
                self.fd_flags.resize(idx + 1, 0);
            }
            self.fd_flags[idx] = 0;
            return Ok(idx as i32);
        }
        if self.entries.len() >= self.limit {
            return Err(LinuxError::EMFILE);
        }
        self.entries.push(Some(entry));
        self.fd_flags.push(0);
        Ok((self.entries.len() - 1) as i32)
    }

    fn set_fd_flags(&mut self, fd: i32, flags: u32) -> Result<(), LinuxError> {
        if fd < 0 || self.entry(fd).is_err() {
            return Err(LinuxError::EBADF);
        }
        if self.fd_flags.len() <= fd as usize {
            self.fd_flags.resize(fd as usize + 1, 0);
        }
        self.fd_flags[fd as usize] = flags & FD_CLOEXEC_FLAG;
        Ok(())
    }

    fn fd_flags(&self, fd: i32) -> Result<u32, LinuxError> {
        self.entry(fd)?;
        Ok(self.fd_flags.get(fd as usize).copied().unwrap_or(0) & FD_CLOEXEC_FLAG)
    }

    fn entry(&self, fd: i32) -> Result<&FdEntry, LinuxError> {
        self.entries
            .get(fd as usize)
            .and_then(|entry| entry.as_ref())
            .ok_or(LinuxError::EBADF)
    }

    fn entry_mut(&mut self, fd: i32) -> Result<&mut FdEntry, LinuxError> {
        self.entries
            .get_mut(fd as usize)
            .and_then(|entry| entry.as_mut())
            .ok_or(LinuxError::EBADF)
    }
}

fn open_fd_entry(
    process: &UserProcess,
    table: &FdTable,
    dirfd: i32,
    path: &str,
    flags: u32,
) -> Result<FdEntry, LinuxError> {
    let mut opts = OpenOptions::new();
    let access = flags & general::O_ACCMODE;
    if access == general::O_WRONLY {
        opts.write(true);
    } else if access == general::O_RDWR {
        opts.read(true);
        opts.write(true);
    } else {
        opts.read(true);
    }
    if flags & general::O_APPEND != 0 {
        opts.append(true);
    }
    if flags & general::O_TRUNC != 0 {
        opts.truncate(true);
    }
    if flags & general::O_CREAT != 0 {
        opts.create(true);
    }
    if flags & general::O_EXCL != 0 {
        opts.create_new(true);
    }

    let prefer_dir = flags & general::O_DIRECTORY != 0;
    let absolute = path.starts_with('/');
    let exec_root = process.exec_root();

    if absolute || dirfd == general::AT_FDCWD {
        let candidates = if absolute {
            if let Some(path) = dev_shm_host_path(path) {
                ensure_dev_shm_dir()?;
                return open_fd_candidates(table, &[path], prefer_dir, &opts);
            }
            runtime_absolute_path_candidates(exec_root.as_str(), path)
        } else {
            let cwd = process.cwd();
            let primary = normalize_path(cwd.as_str(), path).ok_or(LinuxError::EINVAL)?;
            let mut candidates = vec![primary];
            for extra in runtime_library_name_candidates(exec_root.as_str(), path) {
                push_runtime_candidate(&mut candidates, Some(extra));
            }
            candidates
        };
        if candidates.is_empty() {
            return Err(LinuxError::EINVAL);
        }
        open_fd_candidates(table, &candidates, prefer_dir, &opts)
    } else {
        let FdEntry::Directory(dir) = table.entry(dirfd)? else {
            return Err(LinuxError::ENOTDIR);
        };
        let primary = normalize_path(dir.path.as_str(), path).ok_or(LinuxError::EINVAL)?;
        let mut candidates = vec![primary];
        for extra in runtime_library_name_candidates(exec_root.as_str(), path) {
            push_runtime_candidate(&mut candidates, Some(extra));
        }
        open_fd_candidates(table, &candidates, prefer_dir, &opts)
    }
}

fn open_fd_candidates(
    table: &FdTable,
    candidates: &[String],
    prefer_dir: bool,
    opts: &OpenOptions,
) -> Result<FdEntry, LinuxError> {
    let mut last_err = LinuxError::ENOENT;
    for path in candidates {
        if prefer_dir {
            let times = table
                .path_times
                .get(path.as_str())
                .copied()
                .unwrap_or_default();
            match open_dir_entry(path.as_str(), times) {
                Ok(entry) => return Ok(entry),
                Err(err) => {
                    last_err = err;
                    if err != LinuxError::ENOENT {
                        return Err(err);
                    }
                }
            }
            continue;
        }
        match File::open(path.as_str(), opts) {
            Ok(file) => {
                let times = table
                    .path_times
                    .get(path.as_str())
                    .copied()
                    .unwrap_or_default();
                return Ok(FdEntry::File(FileEntry {
                    file,
                    path: path.clone(),
                    times,
                }));
            }
            Err(err) => {
                let err = LinuxError::from(err);
                if err == LinuxError::EISDIR {
                    let times = table
                        .path_times
                        .get(path.as_str())
                        .copied()
                        .unwrap_or_default();
                    return open_dir_entry(path.as_str(), times);
                }
                last_err = err;
                if err != LinuxError::ENOENT {
                    return Err(err);
                }
            }
        }
    }
    Err(last_err)
}

fn dev_shm_host_path(path: &str) -> Option<String> {
    let normalized = normalize_path("/", path)?;
    let rel = normalized.strip_prefix("/dev/shm/")?;
    if rel.is_empty() {
        return None;
    }
    Some(format!("/tmp/shm/{rel}"))
}

fn ensure_dev_shm_dir() -> Result<(), LinuxError> {
    ensure_host_dir("/tmp")?;
    ensure_host_dir("/tmp/shm")
}

fn ensure_host_dir(path: &str) -> Result<(), LinuxError> {
    if axfs::api::metadata(path).is_ok() {
        return Ok(());
    }
    axfs::api::create_dir(path).map_err(LinuxError::from)
}

fn open_dir_entry(path: &str, times: FileTimes) -> Result<FdEntry, LinuxError> {
    let mut opts = OpenOptions::new();
    opts.read(true);
    let dir = Directory::open_dir(path, &opts).map_err(LinuxError::from)?;
    let file = File::open(path, &opts).map_err(LinuxError::from)?;
    let attr = file.get_attr().map_err(LinuxError::from)?;
    Ok(FdEntry::Directory(DirectoryEntry {
        dir,
        attr,
        path: path.into(),
        times,
    }))
}

fn directory_create_dir(path: &str) -> Result<(), LinuxError> {
    axfs::api::create_dir(path).map_err(LinuxError::from)
}

fn directory_remove_file(path: &str) -> Result<(), LinuxError> {
    axfs::api::remove_file(path).map_err(LinuxError::from)
}

fn directory_remove_dir(path: &str) -> Result<(), LinuxError> {
    axfs::api::remove_dir(path).map_err(LinuxError::from)
}

fn resolve_dirfd_path(
    process: &UserProcess,
    table: &FdTable,
    dirfd: i32,
    path: &str,
) -> Result<String, LinuxError> {
    if path.starts_with('/') {
        return normalize_path("/", path).ok_or(LinuxError::EINVAL);
    }
    if dirfd == general::AT_FDCWD {
        let cwd = process.cwd();
        return normalize_path(cwd.as_str(), path).ok_or(LinuxError::EINVAL);
    }
    let FdEntry::Directory(dir) = table.entry(dirfd)? else {
        return Err(LinuxError::ENOTDIR);
    };
    normalize_path(dir.path.as_str(), path).ok_or(LinuxError::EINVAL)
}

fn proc_magiclink_target(process: &UserProcess, table: &FdTable, path: &str) -> Option<String> {
    let normalized = normalize_path("/", path)?;
    if normalized == "/proc/self/exe" || normalized == format!("/proc/{}/exe", process.pid()) {
        return Some(process.exec_path());
    }
    if normalized == "/proc/self/cwd" || normalized == format!("/proc/{}/cwd", process.pid()) {
        return Some(process.cwd());
    }
    if normalized == "/proc/self/root" || normalized == format!("/proc/{}/root", process.pid()) {
        return Some("/".into());
    }
    if normalized == "/proc/thread-self" {
        return Some(format!("/proc/{}/task/{}", process.pid(), current_tid()));
    }

    for prefix in [
        "/proc/self/fd/",
        format!("/proc/{}/fd/", process.pid()).as_str(),
    ] {
        if let Some(fd) = normalized.strip_prefix(prefix) {
            let fd = fd.parse::<i32>().ok()?;
            return fd_link_target(table, fd);
        }
    }
    None
}

fn fd_link_target(table: &FdTable, fd: i32) -> Option<String> {
    let entry = table.entry(fd).ok()?;
    match entry {
        FdEntry::Stdin | FdEntry::Stdout | FdEntry::Stderr => Some("/dev/console".into()),
        FdEntry::DevNull => Some("/dev/null".into()),
        FdEntry::File(file) => Some(file.path.clone()),
        FdEntry::Directory(dir) => Some(dir.path.clone()),
        FdEntry::Pipe(_) => Some(format!("pipe:[{}]", fd)),
        #[cfg(feature = "net")]
        FdEntry::Socket(_) => Some(format!("socket:[{}]", fd)),
    }
}

fn dirent_type(ty: FileType) -> u32 {
    match ty {
        FileType::Dir => general::DT_DIR,
        FileType::CharDevice => general::DT_CHR,
        FileType::BlockDevice => general::DT_BLK,
        FileType::Fifo => general::DT_FIFO,
        FileType::Socket => general::DT_SOCK,
        FileType::SymLink => general::DT_LNK,
        _ => general::DT_REG,
    }
}

fn stdio_stat(readable: bool) -> general::stat {
    let perm = if readable { 0o440 } else { 0o220 };
    let mut st: general::stat = unsafe { core::mem::zeroed() };
    st.st_ino = 1;
    st.st_mode = ST_MODE_CHR | perm;
    st.st_nlink = 1;
    st.st_blksize = 512;
    st
}

fn socket_stat() -> general::stat {
    let mut st: general::stat = unsafe { core::mem::zeroed() };
    st.st_ino = 1;
    st.st_mode = 0o140000 | 0o666;
    st.st_nlink = 1;
    st.st_blksize = 512;
    st
}

fn stat_timestamp_to_statx(sec: i64, nsec: i64) -> general::statx_timestamp {
    general::statx_timestamp {
        tv_sec: sec,
        tv_nsec: nsec.clamp(0, 999_999_999) as u32,
        __reserved: 0,
    }
}

fn stat_to_statx(st: &general::stat) -> general::statx {
    let mut stx: general::statx = unsafe { core::mem::zeroed() };
    stx.stx_mask = general::STATX_BASIC_STATS;
    stx.stx_blksize = st.st_blksize as u32;
    stx.stx_nlink = st.st_nlink as u32;
    stx.stx_uid = st.st_uid;
    stx.stx_gid = st.st_gid;
    stx.stx_mode = st.st_mode as u16;
    stx.stx_ino = st.st_ino as u64;
    stx.stx_size = st.st_size as u64;
    stx.stx_blocks = st.st_blocks as u64;
    stx.stx_atime = stat_timestamp_to_statx(st.st_atime as i64, st.st_atime_nsec as i64);
    stx.stx_mtime = stat_timestamp_to_statx(st.st_mtime as i64, st.st_mtime_nsec as i64);
    stx.stx_ctime = stat_timestamp_to_statx(st.st_ctime as i64, st.st_ctime_nsec as i64);
    stx
}

fn default_statfs() -> general::statfs {
    let mut st: general::statfs = unsafe { core::mem::zeroed() };
    st.f_type = 0xef53;
    st.f_bsize = 4096;
    st.f_blocks = 1024 * 1024;
    st.f_bfree = 512 * 1024;
    st.f_bavail = 512 * 1024;
    st.f_files = 1024 * 1024;
    st.f_ffree = 512 * 1024;
    st.f_fsid = general::__kernel_fsid_t { val: [0, 0] };
    st.f_namelen = 255;
    st.f_frsize = 4096;
    st
}
