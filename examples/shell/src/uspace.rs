use core::cmp;
use core::ffi::{c_char, c_long, CStr};
use core::fmt::Write as _;
use core::mem::{offset_of, size_of};
use core::ptr;
use core::sync::atomic::{
    AtomicBool, AtomicI32, AtomicIsize, AtomicU32, AtomicU64, AtomicUsize, Ordering,
};

use axalloc::global_allocator;
use axerrno::LinuxError;
use axfs::fops::{self, Directory, File, FileAttr, FileType, OpenOptions};
use axhal::context::{TrapFrame, UspaceContext};
use axhal::mem::{phys_to_virt, virt_to_phys};
use axhal::paging::MappingFlags;
use axhal::trap::{
    register_trap_handler, register_user_return_handler, PageFaultFlags, PAGE_FAULT, SYSCALL,
};
use axio::{PollState, SeekFrom};
use axmm::AddrSpace;
use axns::AxNamespace;
use axsync::Mutex;
use axtask::{AxTaskRef, TaskInner, WaitQueue};
use lazyinit::LazyInit;
use linux_raw_sys::{auxvec, general, ioctl, net, system};
use memory_addr::{PageIter4K, PhysAddr, VirtAddr, PAGE_SIZE_4K};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::string::{String, ToString};
use std::sync::Arc;
use std::vec::Vec;
use xmas_elf::header::{Machine, Type as ElfType};
use xmas_elf::program::{Flags as PhFlags, ProgramHeader, Type as PhType};
use xmas_elf::ElfFile;

#[cfg(target_arch = "riscv64")]
use riscv::register::sstatus::{Sstatus, FS};

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
#[cfg(target_arch = "riscv64")]
const SI_TKILL_CODE: i32 = -6;
#[cfg(target_arch = "riscv64")]
const SA_NODEFER_FLAG: u64 = 0x4000_0000;
const KERNEL_SIGSET_BYTES: usize = size_of::<u64>();
const SIG_BLOCK_HOW: usize = 0;
const SIG_UNBLOCK_HOW: usize = 1;
const SIG_SETMASK_HOW: usize = 2;
const FUTEX_BUCKET_COUNT: usize = 64;
const FUTEX_WAITV_MAX: usize = 128;
const SYSV_SHMMIN: usize = 1;
const SYSV_SHMMAX: usize = 8192;
const SYSV_SHMMNI: usize = 4096;
const SYSV_MSGMNB: usize = 16384;
const SYSV_MSGMAX: usize = 8192;
const SYSV_MSGMNI_DEFAULT: usize = 32;
const SYSV_SEMMSL: usize = 32000;
const SYSV_SEMOPM: usize = 500;
const SYSV_SEMMNI_DEFAULT: usize = 32;
const SYSV_SEMVMX: i32 = 32767;
const IPC_CREAT_FLAG: i32 = 0o1000;
const IPC_EXCL_FLAG: i32 = 0o2000;
const IPC_NOWAIT_FLAG: i32 = 0o4000;
const IPC_PRIVATE_KEY: i32 = 0;
const IPC_RMID_CMD: i32 = 0;
const IPC_SET_CMD: i32 = 1;
const IPC_STAT_CMD: i32 = 2;
const IPC_INFO_CMD: i32 = 3;
const MSG_STAT_CMD: i32 = 11;
const MSG_INFO_CMD: i32 = 12;
const MSG_STAT_ANY_CMD: i32 = 13;
const MSG_NOERROR_FLAG: i32 = 0o10000;
const MSG_EXCEPT_FLAG: i32 = 0o20000;
const MSG_COPY_FLAG: i32 = 0o40000;
const GETPID_CMD: i32 = 11;
const GETVAL_CMD: i32 = 12;
const GETALL_CMD: i32 = 13;
const GETNCNT_CMD: i32 = 14;
const GETZCNT_CMD: i32 = 15;
const SETVAL_CMD: i32 = 16;
const SETALL_CMD: i32 = 17;
const SEM_STAT_CMD: i32 = 18;
const SEM_INFO_CMD: i32 = 19;
const SEM_STAT_ANY_CMD: i32 = 20;
const SHM_HUGETLB_FLAG: i32 = 0o4000;
const SHM_RDONLY_FLAG: i32 = 0o10000;
const SHM_RND_FLAG: i32 = 0o20000;
const SHM_REMAP_FLAG: i32 = 0o40000;
const SHM_EXEC_FLAG: i32 = 0o100000;
const SHM_LOCK_CMD: i32 = 11;
const SHM_UNLOCK_CMD: i32 = 12;
const SHM_STAT_CMD: i32 = 13;
const SHM_INFO_CMD: i32 = 14;
const SHM_STAT_ANY_CMD: i32 = 15;
const SHM_DEST_MODE: u32 = 0o1000;
const SHM_LOCKED_MODE: u32 = 0o2000;
const MODE_MASK: u32 = 0o777;
const RLIMIT_STACK_RESOURCE: u32 = 3;
const RLIMIT_NOFILE_RESOURCE: u32 = 7;
const DEFAULT_NOFILE_LIMIT: u64 = 1024;
const MAX_GROUPS: usize = 256;
const FD_SETSIZE: usize = 1024;
const BITS_PER_USIZE: usize = usize::BITS as usize;
const FD_SET_WORDS: usize = FD_SETSIZE.div_ceil(BITS_PER_USIZE);
#[cfg(target_arch = "riscv64")]
const RISCV_SIGNAL_SIGSET_RESERVED_BYTES: usize = 120;
#[cfg(target_arch = "riscv64")]
const RISCV_SIGNAL_FPSTATE_BYTES: usize = 528;
#[cfg(target_arch = "riscv64")]
const SS_DISABLE: i32 = 2;
#[cfg(target_arch = "riscv64")]
const RISCV_SIGTRAMP_CODE: [u32; 3] = [0x08b0_0893, 0x0000_0073, 0x0010_0073];

const ST_MODE_DIR: u32 = 0o040000;
const ST_MODE_FILE: u32 = 0o100000;
const ST_MODE_CHR: u32 = 0o020000;

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
    futex_wait: AtomicUsize,
    futex_wait_state: Mutex<Option<Arc<FutexState>>>,
    robust_list_head: AtomicUsize,
    robust_list_len: AtomicUsize,
    deferred_unmap_start: AtomicUsize,
    deferred_unmap_len: AtomicUsize,
    signal_frame: AtomicUsize,
    pending_sigreturn: Mutex<Option<TrapFrame>>,
}

axtask::def_task_ext!(UserTaskExt);

struct AxNamespaceImpl;

struct UserProcess {
    aspace: Mutex<AddrSpace>,
    brk: Mutex<BrkState>,
    fds: Mutex<FdTable>,
    aio_contexts: Mutex<BTreeMap<u64, Arc<AioContext>>>,
    creds: Mutex<UserCreds>,
    shm_attachments: Mutex<BTreeMap<usize, ShmAttachment>>,
    time_offsets: Mutex<BTreeMap<u32, TimeOffset>>,
    child_time_offsets: Mutex<Option<BTreeMap<u32, TimeOffset>>>,
    cwd: Mutex<String>,
    exec_root: Mutex<String>,
    children: Mutex<Vec<ChildTask>>,
    rlimits: Mutex<BTreeMap<u32, UserRlimit>>,
    signal_actions: Mutex<BTreeMap<usize, general::kernel_sigaction>>,
    next_aio_context: AtomicU64,
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

#[derive(Clone)]
struct UserCreds {
    ruid: u32,
    euid: u32,
    suid: u32,
    rgid: u32,
    egid: u32,
    sgid: u32,
    groups: Vec<u32>,
}

impl UserCreds {
    fn root() -> Self {
        Self {
            ruid: 0,
            euid: 0,
            suid: 0,
            rgid: 0,
            egid: 0,
            sgid: 0,
            groups: vec![0],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxIpcPerm {
    key: i32,
    uid: u32,
    gid: u32,
    cuid: u32,
    cgid: u32,
    mode: u32,
    __pad1: [u8; 4 - size_of::<u32>()],
    seq: u16,
    __pad2: u16,
    __unused1: usize,
    __unused2: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxShmIdDs {
    shm_perm: LinuxIpcPerm,
    shm_segsz: usize,
    shm_atime: i64,
    shm_dtime: i64,
    shm_ctime: i64,
    shm_cpid: i32,
    shm_lpid: i32,
    shm_nattch: usize,
    __unused4: usize,
    __unused5: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxShmInfo {
    used_ids: i32,
    shm_tot: usize,
    shm_rss: usize,
    shm_swp: usize,
    swap_attempts: usize,
    swap_successes: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxShmInfoParams {
    shmmax: usize,
    shmmin: usize,
    shmmni: usize,
    shmseg: usize,
    shmall: usize,
    __unused: [usize; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxMsqidDs {
    msg_perm: LinuxIpcPerm,
    msg_stime: i64,
    msg_rtime: i64,
    msg_ctime: i64,
    msg_cbytes: usize,
    msg_qnum: usize,
    msg_qbytes: usize,
    msg_lspid: i32,
    msg_lrpid: i32,
    __unused4: usize,
    __unused5: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxMsgInfo {
    msgpool: i32,
    msgmap: i32,
    msgmax: i32,
    msgmnb: i32,
    msgmni: i32,
    msgssz: i32,
    msgtql: i32,
    msgseg: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxSemidDs {
    sem_perm: LinuxIpcPerm,
    sem_otime: i64,
    sem_ctime: i64,
    sem_nsems: usize,
    __unused3: usize,
    __unused4: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxSemInfo {
    semmap: i32,
    semmni: i32,
    semmns: i32,
    semmnu: i32,
    semmsl: i32,
    semopm: i32,
    semume: i32,
    semusz: i32,
    semvmx: i32,
    semaem: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxSembuf {
    sem_num: u16,
    sem_op: i16,
    sem_flg: i16,
}

#[derive(Clone)]
struct ShmAttachment {
    segment: Arc<ShmSegment>,
    addr: usize,
    size: usize,
}

struct MsgQueueRecord {
    id: i32,
    key: i32,
    state: Mutex<MsgQueueState>,
    send_wait: WaitQueue,
    recv_wait: WaitQueue,
}

struct MsgQueueState {
    perm: LinuxIpcPerm,
    stime: i64,
    rtime: i64,
    ctime: i64,
    cbytes: usize,
    qbytes: usize,
    lspid: i32,
    lrpid: i32,
    removed: bool,
    messages: VecDeque<MsgMessage>,
}

struct MsgMessage {
    mtype: i64,
    data: Vec<u8>,
}

struct SemSetRecord {
    id: i32,
    key: i32,
    state: Mutex<SemSetState>,
    wait: WaitQueue,
}

struct SemSetState {
    perm: LinuxIpcPerm,
    otime: i64,
    ctime: i64,
    removed: bool,
    sems: Vec<SemState>,
}

#[derive(Clone, Copy)]
struct SemState {
    val: i32,
    pid: i32,
    ncnt: usize,
    zcnt: usize,
}

#[allow(dead_code)]
struct SysvMsgRegistry {
    by_id: BTreeMap<i32, Arc<MsgQueueRecord>>,
    by_key: BTreeMap<i32, i32>,
    next_id: i32,
    next_hint: Option<i32>,
    max_queues: usize,
}

#[allow(dead_code)]
struct SysvSemRegistry {
    by_id: BTreeMap<i32, Arc<SemSetRecord>>,
    by_key: BTreeMap<i32, i32>,
    next_id: i32,
    max_sets: usize,
    max_per_set: usize,
    max_ops: usize,
}

struct SysvShmRegistry {
    by_id: BTreeMap<i32, Arc<ShmSegment>>,
    by_key: BTreeMap<i32, i32>,
    next_id: i32,
    next_hint: Option<i32>,
}

#[allow(dead_code)]
struct SysvRegistry {
    msg: SysvMsgRegistry,
    sem: SysvSemRegistry,
    shm: SysvShmRegistry,
}

struct ShmSegment {
    id: i32,
    key: i32,
    size: usize,
    map_size: usize,
    start_vaddr: usize,
    start_paddr: PhysAddr,
    num_pages: usize,
    meta: Mutex<ShmMeta>,
}

#[derive(Clone, Copy)]
struct ShmMeta {
    perm: LinuxIpcPerm,
    atime: i64,
    dtime: i64,
    ctime: i64,
    cpid: i32,
    lpid: i32,
    nattch: usize,
    removed: bool,
}

struct FdTable {
    entries: Vec<Option<FdEntry>>,
}

#[derive(Clone)]
enum FdEntry {
    Stdin,
    Stdout,
    Stderr,
    DevNull,
    File(FileEntry),
    Directory(DirectoryEntry),
    Pipe(PipeEndpoint),
    Event(EventFdEntry),
    Timer(TimerFdEntry),
    Socket(SocketEntry),
    Epoll(EpollEntry),
    TimeNsOffsets(TimeNsOffsetsFile),
    ProcPseudo(ProcPseudoFile),
}

#[derive(Clone)]
struct FileEntry {
    file: File,
    path: String,
}

#[derive(Clone)]
struct DirectoryEntry {
    dir: Directory,
    attr: FileAttr,
    path: String,
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

const PIPE_BUF_SIZE: usize = 4096;
const IOCB_CMD_PREAD: u16 = 0;
const IOCB_CMD_PWRITE: u16 = 1;
const IOCB_FLAG_RESFD: u32 = 1 << 0;

struct PipeRingBuffer {
    data: [u8; PIPE_BUF_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
}

struct PipeShared {
    buffer: Mutex<PipeRingBuffer>,
    read_wait: WaitQueue,
    write_wait: WaitQueue,
    readers: AtomicUsize,
    writers: AtomicUsize,
}

struct PipeEndpoint {
    readable: bool,
    shared: Arc<PipeShared>,
    status_flags: u32,
    fd_flags: u32,
}

struct EventFdState {
    counter: u64,
    overflow: bool,
}

struct EventFdShared {
    state: Mutex<EventFdState>,
    read_wait: WaitQueue,
    write_wait: WaitQueue,
    semaphore: bool,
}

#[derive(Clone)]
struct EventFdEntry {
    shared: Arc<EventFdShared>,
    status_flags: u32,
    fd_flags: u32,
}

#[derive(Clone, Copy, Default)]
struct TimeOffset {
    secs: i64,
    nanos: i32,
}

struct TimerFdState {
    next_deadline: Option<core::time::Duration>,
    interval: core::time::Duration,
    pending_ticks: u64,
}

struct TimerFdShared {
    state: Mutex<TimerFdState>,
    wait: WaitQueue,
    clockid: u32,
}

#[derive(Clone)]
struct TimerFdEntry {
    shared: Arc<TimerFdShared>,
    status_flags: u32,
    fd_flags: u32,
}

#[derive(Clone)]
struct TimeNsOffsetsFile;

#[derive(Clone)]
struct ProcPseudoFile {
    kind: ProcPseudoKind,
    offset: usize,
}

#[derive(Clone, Copy)]
enum ProcPseudoKind {
    KernelShmMax,
    KernelShmMin,
    KernelShmMni,
    KernelShmAll,
    KernelShmNextId,
    SysvipcShm,
    KernelMsgMni,
    KernelMsgNextId,
    SysvipcMsg,
    KernelSem,
    SysvipcSem,
}

#[derive(Clone)]
struct SocketEntry {
    kind: SocketKind,
    status_flags: u32,
    fd_flags: u32,
}

#[derive(Clone)]
enum SocketKind {
    UnixStream(UnixSocketEndpoint),
    InetPending(InetPendingState),
    InetListener(Arc<InetListenerState>),
    InetStream(InetStreamState),
}

#[derive(Clone)]
struct UnixSocketEndpoint {
    reader: PipeEndpoint,
    writer: PipeEndpoint,
    read_shutdown: Arc<AtomicBool>,
}

#[derive(Clone)]
struct InetPendingState {
    local_port: Option<u16>,
}

struct InetListenerState {
    port: u16,
}

#[derive(Clone)]
struct InetStreamState {
    local_port: u16,
    read_shutdown: Arc<AtomicBool>,
}

#[derive(Clone)]
struct EpollEntry {
    shared: Arc<EpollShared>,
    fd_flags: u32,
}

struct EpollShared {
    watches: Mutex<BTreeMap<i32, EpollWatch>>,
}

#[derive(Clone)]
struct EpollWatch {
    entry: FdEntry,
    events: u32,
    data: u64,
    last_mask: u32,
    oneshot_disabled: bool,
}

struct AioCompletionQueue {
    completions: VecDeque<LinuxIoEvent>,
}

struct AioContext {
    maxevents: usize,
    state: Mutex<AioCompletionQueue>,
    wait: WaitQueue,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxIoEvent {
    data: u64,
    obj: u64,
    res: i64,
    res2: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxIocb {
    aio_data: u64,
    aio_key: u32,
    aio_rw_flags: u32,
    aio_lio_opcode: u16,
    aio_reqprio: i16,
    aio_fildes: u32,
    aio_buf: u64,
    aio_nbytes: u64,
    aio_offset: i64,
    aio_reserved2: u64,
    aio_flags: u32,
    aio_resfd: u32,
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
    seq: AtomicU32,
    waiters: AtomicUsize,
    queue: WaitQueue,
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum FutexKey {
    Shared { uaddr: usize },
    Private { process: usize, uaddr: usize },
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

#[cfg(target_arch = "riscv64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct RiscvSignalInfo {
    bytes: [u8; 128],
}

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

#[cfg(target_arch = "riscv64")]
const _: [(); RISCV_SIGNAL_FPSTATE_BYTES] = [(); size_of::<RiscvSignalFpState>()];
#[cfg(target_arch = "riscv64")]
const _: [(); 784] = [(); size_of::<RiscvSignalSigcontext>()];
#[cfg(target_arch = "riscv64")]
const _: [(); 960] = [(); size_of::<RiscvSignalUcontext>()];
#[cfg(target_arch = "riscv64")]
const _: [(); 1104] = [(); size_of::<RiscvSignalFrame>()];

#[repr(C)]
#[derive(Clone, Copy)]
struct UserFdSet {
    fds_bits: [usize; FD_SET_WORDS],
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
    fn new_pair_with_flags(flags: u32) -> Result<(Self, Self), LinuxError> {
        let supported = general::O_NONBLOCK | general::O_CLOEXEC;
        if flags & !supported != 0 {
            return Err(LinuxError::EINVAL);
        }
        let status_flags = flags & general::O_NONBLOCK;
        let fd_flags = if flags & general::O_CLOEXEC != 0 {
            general::FD_CLOEXEC
        } else {
            0
        };
        let shared = Arc::new(PipeShared {
            buffer: Mutex::new(PipeRingBuffer::new()),
            read_wait: WaitQueue::new(),
            write_wait: WaitQueue::new(),
            readers: AtomicUsize::new(1),
            writers: AtomicUsize::new(1),
        });
        Ok((
            Self {
                readable: true,
                shared: shared.clone(),
                status_flags,
                fd_flags,
            },
            Self {
                readable: false,
                shared,
                status_flags,
                fd_flags,
            },
        ))
    }

    const fn writable(&self) -> bool {
        !self.readable
    }

    fn reader_count(&self) -> usize {
        self.shared.readers.load(Ordering::Acquire)
    }

    fn writer_count(&self) -> usize {
        self.shared.writers.load(Ordering::Acquire)
    }

    fn nonblock(&self) -> bool {
        self.status_flags & general::O_NONBLOCK != 0
    }

    fn read(&self, dst: &mut [u8]) -> Result<usize, LinuxError> {
        if !self.readable {
            return Err(LinuxError::EBADF);
        }
        if dst.is_empty() {
            return Ok(0);
        }
        let mut read_len = 0usize;
        loop {
            let mut ring = self.shared.buffer.lock();
            let available = ring.available_read();
            if available > 0 {
                while read_len < dst.len() && ring.available_read() > 0 {
                    dst[read_len] = ring.read_byte();
                    read_len += 1;
                }
                drop(ring);
                self.shared.write_wait.notify_all(true);
                return Ok(read_len);
            }
            if read_len > 0 || self.writer_count() == 0 {
                return Ok(read_len);
            }
            if self.nonblock() {
                return Err(LinuxError::EAGAIN);
            }
            drop(ring);
            self.shared.read_wait.wait_until(|| {
                let ring = self.shared.buffer.lock();
                ring.available_read() > 0 || self.writer_count() == 0
            });
        }
    }

    fn write(&self, src: &[u8]) -> Result<usize, LinuxError> {
        if !self.writable() {
            return Err(LinuxError::EBADF);
        }
        if src.is_empty() {
            return Ok(0);
        }
        let mut written = 0usize;
        loop {
            if self.reader_count() == 0 {
                return if written > 0 {
                    Ok(written)
                } else {
                    Err(LinuxError::EPIPE)
                };
            }
            let mut ring = self.shared.buffer.lock();
            let available = ring.available_write();
            if available > 0 {
                while written < src.len() && ring.available_write() > 0 {
                    ring.write_byte(src[written]);
                    written += 1;
                }
                drop(ring);
                self.shared.read_wait.notify_all(true);
                if written == src.len() {
                    return Ok(written);
                }
                continue;
            }
            if self.nonblock() {
                return if written > 0 {
                    Ok(written)
                } else {
                    Err(LinuxError::EAGAIN)
                };
            }
            drop(ring);
            self.shared.write_wait.wait_until(|| {
                let ring = self.shared.buffer.lock();
                ring.available_write() > 0 || self.reader_count() == 0
            });
        }
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
        let ring = self.shared.buffer.lock();
        PollState {
            readable: self.readable && (ring.available_read() > 0 || self.writer_count() == 0),
            writable: self.writable() && self.reader_count() > 0 && ring.available_write() > 0,
        }
    }

    fn getfd(&self) -> i32 {
        self.fd_flags as i32
    }

    fn setfd(&mut self, flags: u32) -> i32 {
        self.fd_flags = flags & general::FD_CLOEXEC;
        0
    }

    fn getfl(&self) -> i32 {
        let access = if self.readable {
            general::O_RDONLY
        } else {
            general::O_WRONLY
        };
        (access | self.status_flags) as i32
    }

    fn setfl(&mut self, flags: u32) -> i32 {
        self.status_flags = flags & general::O_NONBLOCK;
        0
    }
}

impl EventFdEntry {
    fn new(initval: u32, flags: u32) -> Result<Self, LinuxError> {
        let supported = general::EFD_SEMAPHORE | general::EFD_CLOEXEC | general::EFD_NONBLOCK;
        if flags & !supported != 0 {
            return Err(LinuxError::EINVAL);
        }
        Ok(Self {
            shared: Arc::new(EventFdShared {
                state: Mutex::new(EventFdState {
                    counter: initval as u64,
                    overflow: false,
                }),
                read_wait: WaitQueue::new(),
                write_wait: WaitQueue::new(),
                semaphore: flags & general::EFD_SEMAPHORE != 0,
            }),
            status_flags: flags & general::O_NONBLOCK,
            fd_flags: if flags & general::EFD_CLOEXEC != 0 {
                general::FD_CLOEXEC
            } else {
                0
            },
        })
    }

    fn nonblock(&self) -> bool {
        self.status_flags & general::O_NONBLOCK != 0
    }

    fn read(&self, dst: &mut [u8]) -> Result<usize, LinuxError> {
        if dst.len() < size_of::<u64>() {
            return Err(LinuxError::EINVAL);
        }
        loop {
            let mut state = self.shared.state.lock();
            if state.overflow {
                let bytes = u64::MAX.to_ne_bytes();
                state.overflow = false;
                state.counter = 0;
                drop(state);
                dst[..size_of::<u64>()].copy_from_slice(&bytes);
                self.shared.write_wait.notify_all(true);
                return Ok(size_of::<u64>());
            }
            if state.counter != 0 {
                let value = if self.shared.semaphore {
                    state.counter -= 1;
                    1
                } else {
                    let value = state.counter;
                    state.counter = 0;
                    value
                };
                let more_to_read = state.counter != 0;
                drop(state);
                dst[..size_of::<u64>()].copy_from_slice(&value.to_ne_bytes());
                if more_to_read {
                    self.shared.read_wait.notify_all(true);
                }
                self.shared.write_wait.notify_all(true);
                return Ok(size_of::<u64>());
            }
            if self.nonblock() {
                return Err(LinuxError::EAGAIN);
            }
            drop(state);
            self.shared.read_wait.wait_until(|| {
                let state = self.shared.state.lock();
                state.overflow || state.counter != 0
            });
        }
    }

    fn write(&self, src: &[u8]) -> Result<usize, LinuxError> {
        if src.len() < size_of::<u64>() {
            return Err(LinuxError::EINVAL);
        }
        let value = u64::from_ne_bytes(src[..size_of::<u64>()].try_into().unwrap());
        if value == u64::MAX {
            return Err(LinuxError::EINVAL);
        }
        loop {
            let mut state = self.shared.state.lock();
            let limit = u64::MAX - 1;
            if !state.overflow && value <= limit.saturating_sub(state.counter) {
                let was_empty = state.counter == 0;
                state.counter += value;
                drop(state);
                if was_empty && value != 0 {
                    self.shared.read_wait.notify_all(true);
                }
                return Ok(size_of::<u64>());
            }
            if self.nonblock() {
                return Err(LinuxError::EAGAIN);
            }
            drop(state);
            self.shared.write_wait.wait_until(|| {
                let state = self.shared.state.lock();
                !state.overflow && value <= (u64::MAX - 1).saturating_sub(state.counter)
            });
        }
    }

    fn poll(&self) -> PollState {
        let state = self.shared.state.lock();
        PollState {
            readable: state.overflow || state.counter != 0,
            writable: state.overflow || state.counter < u64::MAX - 1,
        }
    }

    fn kernel_signal(&self, value: u64) {
        let mut state = self.shared.state.lock();
        if state.overflow {
            return;
        }
        let limit = u64::MAX - 1;
        if value <= limit.saturating_sub(state.counter) {
            let was_empty = state.counter == 0;
            state.counter += value;
            drop(state);
            if was_empty && value != 0 {
                self.shared.read_wait.notify_all(true);
            }
        } else {
            state.counter = 0;
            state.overflow = true;
            drop(state);
            self.shared.read_wait.notify_all(true);
            self.shared.write_wait.notify_all(true);
        }
    }

    fn stat(&self) -> general::stat {
        let mut st: general::stat = unsafe { core::mem::zeroed() };
        st.st_ino = 2;
        st.st_mode = ST_MODE_FILE | 0o600;
        st.st_nlink = 1;
        st.st_blksize = size_of::<u64>() as _;
        st
    }

    fn getfd(&self) -> i32 {
        self.fd_flags as i32
    }

    fn setfd(&mut self, flags: u32) -> i32 {
        self.fd_flags = flags & general::FD_CLOEXEC;
        0
    }

    fn getfl(&self) -> i32 {
        (general::O_RDWR | self.status_flags) as i32
    }

    fn setfl(&mut self, flags: u32) -> i32 {
        self.status_flags = flags & general::O_NONBLOCK;
        0
    }
}

impl TimerFdEntry {
    fn new(clockid: u32, flags: u32) -> Result<Self, LinuxError> {
        if flags & !general::TFD_CREATE_FLAGS != 0 {
            return Err(LinuxError::EINVAL);
        }
        validate_clock_id(clockid)?;
        Ok(Self {
            shared: Arc::new(TimerFdShared {
                state: Mutex::new(TimerFdState {
                    next_deadline: None,
                    interval: core::time::Duration::ZERO,
                    pending_ticks: 0,
                }),
                wait: WaitQueue::new(),
                clockid,
            }),
            status_flags: flags & general::TFD_NONBLOCK,
            fd_flags: if flags & general::TFD_CLOEXEC != 0 {
                general::FD_CLOEXEC
            } else {
                0
            },
        })
    }

    fn nonblock(&self) -> bool {
        self.status_flags & general::O_NONBLOCK != 0
    }

    fn refresh_state(
        state: &mut TimerFdState,
        now: core::time::Duration,
    ) -> Result<(), LinuxError> {
        let Some(deadline) = state.next_deadline else {
            return Ok(());
        };
        if now < deadline {
            return Ok(());
        }
        if state.interval.is_zero() {
            state.pending_ticks = state.pending_ticks.saturating_add(1);
            state.next_deadline = None;
            return Ok(());
        }
        let interval_ns = state.interval.as_nanos();
        let overdue_ns = now
            .checked_sub(deadline)
            .ok_or(LinuxError::EINVAL)?
            .as_nanos();
        let missed = 1 + overdue_ns / interval_ns;
        state.pending_ticks = state.pending_ticks.saturating_add(missed as u64);
        let next_ns = deadline.as_nanos() + missed * interval_ns;
        let secs = (next_ns / 1_000_000_000) as u64;
        let nanos = (next_ns % 1_000_000_000) as u32;
        state.next_deadline = Some(core::time::Duration::new(secs, nanos));
        Ok(())
    }

    fn read(&self, process: &UserProcess, dst: &mut [u8]) -> Result<usize, LinuxError> {
        if dst.len() < size_of::<u64>() {
            return Err(LinuxError::EINVAL);
        }
        loop {
            let now = clock_now_duration(process, self.shared.clockid)?;
            let mut state = self.shared.state.lock();
            Self::refresh_state(&mut state, now)?;
            if state.pending_ticks != 0 {
                let ticks = state.pending_ticks;
                state.pending_ticks = 0;
                drop(state);
                dst[..size_of::<u64>()].copy_from_slice(&ticks.to_ne_bytes());
                return Ok(size_of::<u64>());
            }
            if self.nonblock() {
                return Err(LinuxError::EAGAIN);
            }
            let wait_for = state
                .next_deadline
                .and_then(|deadline| deadline.checked_sub(now));
            drop(state);
            let Some(wait_for) = wait_for else {
                self.shared.wait.wait();
                continue;
            };
            if self.shared.wait.wait_timeout_until(wait_for, || {
                let now = match clock_now_duration(process, self.shared.clockid) {
                    Ok(now) => now,
                    Err(_) => return true,
                };
                let mut state = self.shared.state.lock();
                Self::refresh_state(&mut state, now).is_ok() && state.pending_ticks != 0
            }) {
                continue;
            }
        }
    }

    fn poll(&self, process: &UserProcess) -> PollState {
        let now = clock_now_duration(process, self.shared.clockid).ok();
        let mut state = self.shared.state.lock();
        if let Some(now) = now {
            let _ = Self::refresh_state(&mut state, now);
        }
        PollState {
            readable: state.pending_ticks != 0,
            writable: true,
        }
    }

    fn stat(&self) -> general::stat {
        let mut st: general::stat = unsafe { core::mem::zeroed() };
        st.st_ino = 3;
        st.st_mode = ST_MODE_FILE | 0o600;
        st.st_nlink = 1;
        st.st_blksize = size_of::<u64>() as _;
        st
    }

    fn getfd(&self) -> i32 {
        self.fd_flags as i32
    }

    fn setfd(&mut self, flags: u32) -> i32 {
        self.fd_flags = flags & general::FD_CLOEXEC;
        0
    }

    fn getfl(&self) -> i32 {
        (general::O_RDWR | self.status_flags) as i32
    }

    fn setfl(&mut self, flags: u32) -> i32 {
        self.status_flags = flags & general::O_NONBLOCK;
        0
    }
}

impl SocketEntry {
    fn new_inet_stream(flags: u32) -> Result<Self, LinuxError> {
        let status_flags = flags & general::O_NONBLOCK;
        let fd_flags = if flags & general::O_CLOEXEC != 0 {
            general::FD_CLOEXEC
        } else {
            0
        };
        Ok(Self {
            kind: SocketKind::InetPending(InetPendingState { local_port: None }),
            status_flags,
            fd_flags,
        })
    }

    fn new_socketpair(flags: u32) -> Result<(Self, Self), LinuxError> {
        let (left_reader, right_writer) = PipeEndpoint::new_pair_with_flags(flags)?;
        let (right_reader, left_writer) = PipeEndpoint::new_pair_with_flags(flags)?;
        let status_flags = flags & general::O_NONBLOCK;
        let fd_flags = if flags & general::O_CLOEXEC != 0 {
            general::FD_CLOEXEC
        } else {
            0
        };
        let left = Self {
            kind: SocketKind::UnixStream(UnixSocketEndpoint {
                reader: left_reader,
                writer: left_writer,
                read_shutdown: Arc::new(AtomicBool::new(false)),
            }),
            status_flags,
            fd_flags,
        };
        let right = Self {
            kind: SocketKind::UnixStream(UnixSocketEndpoint {
                reader: right_reader,
                writer: right_writer,
                read_shutdown: Arc::new(AtomicBool::new(false)),
            }),
            status_flags,
            fd_flags,
        };
        Ok((left, right))
    }

    fn read(&self, dst: &mut [u8]) -> Result<usize, LinuxError> {
        match &self.kind {
            SocketKind::UnixStream(stream) => stream.reader.read(dst),
            SocketKind::InetStream(stream) => {
                if stream.read_shutdown.load(Ordering::Acquire) {
                    Ok(0)
                } else if self.status_flags & general::O_NONBLOCK != 0 {
                    Err(LinuxError::EAGAIN)
                } else {
                    Ok(0)
                }
            }
            _ => Err(LinuxError::ENOTCONN),
        }
    }

    fn write(&self, src: &[u8]) -> Result<usize, LinuxError> {
        match &self.kind {
            SocketKind::UnixStream(stream) => stream.writer.write(src),
            SocketKind::InetStream(_) => Ok(src.len()),
            _ => Err(LinuxError::ENOTCONN),
        }
    }

    fn ready_mask(&self, process: &UserProcess) -> u32 {
        let _ = process;
        match &self.kind {
            SocketKind::UnixStream(stream) => {
                let read = stream.reader.poll();
                let write = stream.writer.poll();
                let mut mask = 0u32;
                if read.readable {
                    mask |= general::EPOLLIN;
                }
                if write.writable {
                    mask |= general::EPOLLOUT;
                }
                if stream.read_shutdown.load(Ordering::Acquire) {
                    mask |= general::EPOLLRDHUP;
                }
                mask
            }
            SocketKind::InetStream(stream) => {
                let mut mask = general::EPOLLOUT;
                if stream.read_shutdown.load(Ordering::Acquire) {
                    mask |= general::EPOLLRDHUP;
                }
                mask
            }
            SocketKind::InetListener(_) => general::EPOLLOUT,
            SocketKind::InetPending(_) => 0,
        }
    }

    fn bind(
        &mut self,
        process: &UserProcess,
        addr: usize,
        addrlen: usize,
    ) -> Result<(), LinuxError> {
        let (_, port) = read_sockaddr_in(process, addr, addrlen)?;
        let port = if port == 0 { next_inet_port() } else { port };
        let SocketKind::InetPending(state) = &mut self.kind else {
            return Err(LinuxError::EINVAL);
        };
        if state.local_port.is_some() {
            return Err(LinuxError::EINVAL);
        }
        if inet_listener_table().lock().contains_key(&port) {
            return Err(LinuxError::EADDRINUSE);
        }
        state.local_port = Some(port);
        Ok(())
    }

    fn listen(&mut self, backlog: usize) -> Result<(), LinuxError> {
        let _ = backlog;
        let SocketKind::InetPending(state) = &self.kind else {
            return Err(LinuxError::EINVAL);
        };
        let port = state.local_port.ok_or(LinuxError::EINVAL)?;
        let listener = Arc::new(InetListenerState { port });
        inet_listener_table().lock().insert(port, listener.clone());
        self.kind = SocketKind::InetListener(listener);
        Ok(())
    }

    fn connect(
        &mut self,
        process: &UserProcess,
        addr: usize,
        addrlen: usize,
    ) -> Result<(), LinuxError> {
        let (_, peer_port) = read_sockaddr_in(process, addr, addrlen)?;
        if !inet_listener_table().lock().contains_key(&peer_port) {
            return Err(LinuxError::ECONNREFUSED);
        }
        let SocketKind::InetPending(state) = &self.kind else {
            return Err(LinuxError::EINVAL);
        };
        let local_port = state.local_port.unwrap_or_else(next_inet_port);
        self.kind = SocketKind::InetStream(InetStreamState {
            local_port,
            read_shutdown: Arc::new(AtomicBool::new(false)),
        });
        Ok(())
    }

    fn shutdown(&mut self, how: u32) -> Result<(), LinuxError> {
        match &mut self.kind {
            SocketKind::UnixStream(stream) => {
                if how == net::SHUT_RD || how == net::SHUT_RDWR {
                    stream.read_shutdown.store(true, Ordering::Release);
                }
                Ok(())
            }
            SocketKind::InetStream(stream) => {
                if how == net::SHUT_RD || how == net::SHUT_RDWR {
                    stream.read_shutdown.store(true, Ordering::Release);
                }
                Ok(())
            }
            _ => Err(LinuxError::ENOTCONN),
        }
    }

    fn getsockname(
        &self,
        process: &UserProcess,
        addr: usize,
        addrlen: usize,
    ) -> Result<(), LinuxError> {
        let port = match &self.kind {
            SocketKind::InetPending(state) => state.local_port.unwrap_or(0),
            SocketKind::InetListener(listener) => listener.port,
            SocketKind::InetStream(stream) => stream.local_port,
            SocketKind::UnixStream(_) => return Err(LinuxError::EINVAL),
        };
        write_sockaddr_in(process, addr, addrlen, port)
    }

    fn stat(&self) -> general::stat {
        let mut st: general::stat = unsafe { core::mem::zeroed() };
        st.st_ino = 4;
        st.st_mode = ST_MODE_FILE | 0o600;
        st.st_nlink = 1;
        st
    }

    fn getfd(&self) -> i32 {
        self.fd_flags as i32
    }

    fn setfd(&mut self, flags: u32) -> i32 {
        self.fd_flags = flags & general::FD_CLOEXEC;
        0
    }

    fn getfl(&self) -> i32 {
        (general::O_RDWR | self.status_flags) as i32
    }

    fn setfl(&mut self, flags: u32) -> i32 {
        self.status_flags = flags & general::O_NONBLOCK;
        if let SocketKind::UnixStream(stream) = &mut self.kind {
            stream.reader.setfl(flags);
            stream.writer.setfl(flags);
        }
        0
    }
}

impl EpollEntry {
    fn new(flags: u32) -> Result<Self, LinuxError> {
        if flags & !general::EPOLL_CLOEXEC != 0 {
            return Err(LinuxError::EINVAL);
        }
        Ok(Self {
            shared: Arc::new(EpollShared {
                watches: Mutex::new(BTreeMap::new()),
            }),
            fd_flags: if flags & general::EPOLL_CLOEXEC != 0 {
                general::FD_CLOEXEC
            } else {
                0
            },
        })
    }

    fn getfd(&self) -> i32 {
        self.fd_flags as i32
    }

    fn setfd(&mut self, flags: u32) -> i32 {
        self.fd_flags = flags & general::FD_CLOEXEC;
        0
    }

    fn stat(&self) -> general::stat {
        let mut st: general::stat = unsafe { core::mem::zeroed() };
        st.st_ino = 5;
        st.st_mode = ST_MODE_FILE | 0o600;
        st.st_nlink = 1;
        st
    }
}

impl Drop for InetListenerState {
    fn drop(&mut self) {
        inet_listener_table().lock().remove(&self.port);
    }
}

impl AioContext {
    fn new(maxevents: usize) -> Self {
        Self {
            maxevents,
            state: Mutex::new(AioCompletionQueue {
                completions: VecDeque::new(),
            }),
            wait: WaitQueue::new(),
        }
    }

    fn push_completion(&self, event: LinuxIoEvent) {
        let mut state = self.state.lock();
        if state.completions.len() < self.maxevents.max(1) {
            state.completions.push_back(event);
        }
        drop(state);
        self.wait.notify_all(true);
    }
}

impl Clone for PipeEndpoint {
    fn clone(&self) -> Self {
        if self.readable {
            self.shared.readers.fetch_add(1, Ordering::AcqRel);
        } else {
            self.shared.writers.fetch_add(1, Ordering::AcqRel);
        }
        Self {
            readable: self.readable,
            shared: self.shared.clone(),
            status_flags: self.status_flags,
            fd_flags: self.fd_flags,
        }
    }
}

impl Drop for PipeEndpoint {
    fn drop(&mut self) {
        if self.readable {
            if self.shared.readers.fetch_sub(1, Ordering::AcqRel) == 1 {
                self.shared.write_wait.notify_all(true);
            }
        } else if self.shared.writers.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.shared.read_wait.notify_all(true);
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
        futex_wait: AtomicUsize::new(0),
        futex_wait_state: Mutex::new(None),
        robust_list_head: AtomicUsize::new(0),
        robust_list_len: AtomicUsize::new(0),
        deferred_unmap_start: AtomicUsize::new(0),
        deferred_unmap_len: AtomicUsize::new(0),
        signal_frame: AtomicUsize::new(0),
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
        aio_contexts: Mutex::new(BTreeMap::new()),
        creds: Mutex::new(UserCreds::root()),
        shm_attachments: Mutex::new(BTreeMap::new()),
        time_offsets: Mutex::new(BTreeMap::new()),
        child_time_offsets: Mutex::new(None),
        cwd: Mutex::new(cwd.into()),
        exec_root: Mutex::new(image.exec_root.clone()),
        children: Mutex::new(Vec::new()),
        rlimits: Mutex::new(BTreeMap::new()),
        signal_actions: Mutex::new(BTreeMap::new()),
        next_aio_context: AtomicU64::new(1),
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
        let interp_image = std::fs::read(interp_path.as_str())
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
            true,
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
        std::fs::read(path.as_str()).map_err(|err| format!("failed to read {path}: {err}"))?;

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
    if matches!(std::fs::metadata(&resolved), Ok(meta) if meta.is_file()) {
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
        matches!(std::fs::metadata(path), Ok(meta) if meta.is_file()).then(|| path.to_string())
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
    aspace: &AddrSpace,
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
    aspace
        .write(VirtAddr::from(sp), &bytes)
        .map_err(|err| format!("failed to populate user stack: {err}"))?;
    Ok(sp)
}

fn push_stack_bytes(
    aspace: &AddrSpace,
    stack_base: usize,
    sp: &mut usize,
    data: &[u8],
    align: usize,
) -> Result<usize, String> {
    *sp = align_down(sp.saturating_sub(data.len()), align.max(1));
    if *sp < stack_base {
        return Err("user stack overflow".into());
    }
    aspace
        .write(VirtAddr::from(*sp), data)
        .map_err(|err| format!("failed to write user stack data: {err}"))?;
    Ok(*sp)
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
    let argv_refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
    let image = {
        let mut aspace = process.aspace.lock();
        load_program_image(&mut aspace, cwd, &argv_refs)?
    };
    *process.brk.lock() = image.brk;
    process.set_exec_root(image.exec_root);
    Ok((image.entry, image.stack_ptr, image.argc))
}

impl UserProcess {
    fn cwd(&self) -> String {
        self.cwd.lock().clone()
    }

    fn exec_root(&self) -> String {
        self.exec_root.lock().clone()
    }

    fn set_cwd(&self, cwd: String) {
        *self.cwd.lock() = cwd;
    }

    fn set_exec_root(&self, exec_root: String) {
        *self.exec_root.lock() = exec_root;
    }

    fn teardown(&self) {
        self.detach_all_shm();
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

    fn creds(&self) -> UserCreds {
        self.creds.lock().clone()
    }

    fn effective_uid(&self) -> u32 {
        self.creds.lock().euid
    }

    fn effective_gid(&self) -> u32 {
        self.creds.lock().egid
    }

    fn is_superuser(&self) -> bool {
        self.effective_uid() == 0
    }

    fn setuid(&self, uid: u32) -> Result<(), LinuxError> {
        let mut creds = self.creds.lock();
        if creds.euid == 0 {
            creds.ruid = uid;
            creds.euid = uid;
            creds.suid = uid;
            return Ok(());
        }
        if uid == creds.ruid || uid == creds.suid {
            creds.euid = uid;
            return Ok(());
        }
        Err(LinuxError::EPERM)
    }

    fn setgid(&self, gid: u32) -> Result<(), LinuxError> {
        let mut creds = self.creds.lock();
        if creds.euid == 0 {
            creds.rgid = gid;
            creds.egid = gid;
            creds.sgid = gid;
            return Ok(());
        }
        if gid == creds.rgid || gid == creds.sgid {
            creds.egid = gid;
            return Ok(());
        }
        Err(LinuxError::EPERM)
    }

    fn setresuid(
        &self,
        ruid: Option<u32>,
        euid: Option<u32>,
        suid: Option<u32>,
    ) -> Result<(), LinuxError> {
        let mut creds = self.creds.lock();
        if creds.euid != 0 {
            for uid in [ruid, euid, suid].into_iter().flatten() {
                if uid != creds.ruid && uid != creds.euid && uid != creds.suid {
                    return Err(LinuxError::EPERM);
                }
            }
        }
        if let Some(uid) = ruid {
            creds.ruid = uid;
        }
        if let Some(uid) = euid {
            creds.euid = uid;
        }
        if let Some(uid) = suid {
            creds.suid = uid;
        }
        Ok(())
    }

    fn setresgid(
        &self,
        rgid: Option<u32>,
        egid: Option<u32>,
        sgid: Option<u32>,
    ) -> Result<(), LinuxError> {
        let mut creds = self.creds.lock();
        if creds.euid != 0 {
            for gid in [rgid, egid, sgid].into_iter().flatten() {
                if gid != creds.rgid && gid != creds.egid && gid != creds.sgid {
                    return Err(LinuxError::EPERM);
                }
            }
        }
        if let Some(gid) = rgid {
            creds.rgid = gid;
        }
        if let Some(gid) = egid {
            creds.egid = gid;
        }
        if let Some(gid) = sgid {
            creds.sgid = gid;
        }
        Ok(())
    }

    fn supplementary_groups(&self) -> Vec<u32> {
        self.creds.lock().groups.clone()
    }

    fn setgroups(&self, groups: Vec<u32>) -> Result<(), LinuxError> {
        if !self.is_superuser() {
            return Err(LinuxError::EPERM);
        }
        self.creds.lock().groups = groups;
        Ok(())
    }

    fn add_thread(&self) {
        self.live_threads.fetch_add(1, Ordering::AcqRel);
    }

    fn note_thread_exit(&self, code: i32) {
        self.exit_code.store(code, Ordering::Release);
        if self.live_threads.fetch_sub(1, Ordering::AcqRel) == 1 {
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

    fn effective_time_offsets(&self) -> BTreeMap<u32, TimeOffset> {
        self.time_offsets.lock().clone()
    }

    fn set_child_time_namespace(&self) {
        *self.child_time_offsets.lock() = Some(BTreeMap::new());
    }

    fn write_child_time_offset(&self, clockid: u32, offset: TimeOffset) -> Result<(), LinuxError> {
        let mut pending = self.child_time_offsets.lock();
        let Some(offsets) = pending.as_mut() else {
            return Err(LinuxError::EINVAL);
        };
        offsets.insert(clockid, offset);
        Ok(())
    }

    fn detach_all_shm(&self) {
        let addrs = self
            .shm_attachments
            .lock()
            .keys()
            .copied()
            .collect::<Vec<_>>();
        for addr in addrs {
            let _ = self.shmdt(addr);
        }
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

        let child = Arc::new(UserProcess {
            aspace: Mutex::new(aspace),
            brk: Mutex::new(*self.brk.lock()),
            fds: Mutex::new(self.fds.lock().fork_copy()?),
            aio_contexts: Mutex::new(BTreeMap::new()),
            creds: Mutex::new(self.creds.lock().clone()),
            shm_attachments: Mutex::new(BTreeMap::new()),
            time_offsets: Mutex::new(
                self.child_time_offsets
                    .lock()
                    .clone()
                    .unwrap_or_else(|| self.effective_time_offsets()),
            ),
            child_time_offsets: Mutex::new(None),
            cwd: Mutex::new(self.cwd()),
            exec_root: Mutex::new(self.exec_root()),
            children: Mutex::new(Vec::new()),
            rlimits: Mutex::new(self.rlimits.lock().clone()),
            signal_actions: Mutex::new(self.signal_actions.lock().clone()),
            next_aio_context: AtomicU64::new(1),
            pid: AtomicI32::new(0),
            ppid: axtask::current().id().as_u64() as i32,
            live_threads: AtomicUsize::new(1),
            exit_group_code: AtomicI32::new(NO_EXIT_GROUP_CODE),
            exit_code: AtomicI32::new(0),
            exit_wait: WaitQueue::new(),
        });
        child.inherit_shm_attachments_from(self)?;
        Ok(child)
    }

    fn inherit_shm_attachments_from(&self, parent: &UserProcess) -> Result<(), LinuxError> {
        let attachments = parent.shm_attachments.lock().clone();
        for attachment in attachments.values() {
            self.map_shm_attachment(
                attachment.segment.clone(),
                attachment.addr,
                false,
                false,
                true,
            )?;
        }
        Ok(())
    }

    fn map_shm_attachment(
        &self,
        segment: Arc<ShmSegment>,
        addr: usize,
        readonly: bool,
        exec: bool,
        inherited: bool,
    ) -> Result<usize, LinuxError> {
        let mut flags = MappingFlags::USER | MappingFlags::READ;
        if !readonly {
            flags |= MappingFlags::WRITE;
        }
        if exec {
            flags |= MappingFlags::EXECUTE;
        }
        let mut aspace = self.aspace.lock();
        if inherited {
            let _ = aspace.unmap(VirtAddr::from(addr), segment.map_size);
        }
        aspace
            .map_linear(
                VirtAddr::from(addr),
                segment.start_paddr,
                segment.map_size,
                flags,
            )
            .map_err(LinuxError::from)?;
        self.shm_attachments.lock().insert(
            addr,
            ShmAttachment {
                segment: segment.clone(),
                addr,
                size: segment.map_size,
            },
        );
        let mut meta = segment.meta.lock();
        meta.nattch += 1;
        meta.lpid = self.pid();
        meta.atime = now_unix_secs();
        Ok(addr)
    }

    fn shmdt(&self, addr: usize) -> Result<(), LinuxError> {
        if addr % PAGE_SIZE_4K != 0 {
            return Err(LinuxError::EINVAL);
        }
        let attachment = self
            .shm_attachments
            .lock()
            .remove(&addr)
            .ok_or(LinuxError::EINVAL)?;
        self.aspace
            .lock()
            .unmap(VirtAddr::from(addr), attachment.size)
            .map_err(LinuxError::from)?;
        let mut meta = attachment.segment.meta.lock();
        meta.nattch = meta.nattch.saturating_sub(1);
        meta.dtime = now_unix_secs();
        meta.lpid = self.pid();
        let reclaim = meta.removed && meta.nattch == 0;
        drop(meta);
        if reclaim {
            reclaim_shm_segment(&attachment.segment);
        }
        Ok(())
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

        let child = {
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

            if let Some(index) = exited_index {
                children.remove(index)
            } else if nohang {
                return Ok(None);
            } else if pid == -1 {
                children.remove(0)
            } else {
                let index = children
                    .iter()
                    .position(|child| child.pid == pid)
                    .ok_or(LinuxError::ECHILD)?;
                children.remove(index)
            }
        };
        let status = child.task.join().ok_or(LinuxError::ECHILD)?;
        let child_pid = child.pid;
        child.process.teardown();
        drop(child);
        axtask::yield_now();
        Ok(Some((child_pid, status)))
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

fn futex_buckets() -> &'static Vec<Mutex<BTreeMap<FutexKey, Arc<FutexState>>>> {
    static FUTEXES: LazyInit<Vec<Mutex<BTreeMap<FutexKey, Arc<FutexState>>>>> = LazyInit::new();
    if !FUTEXES.is_inited() {
        FUTEXES.init_once(
            (0..FUTEX_BUCKET_COUNT)
                .map(|_| Mutex::new(BTreeMap::new()))
                .collect(),
        );
    }
    &FUTEXES
}

fn sysv_registry() -> &'static Mutex<SysvRegistry> {
    static SYSV: LazyInit<Mutex<SysvRegistry>> = LazyInit::new();
    if !SYSV.is_inited() {
        SYSV.init_once(Mutex::new(SysvRegistry {
            msg: SysvMsgRegistry {
                by_id: BTreeMap::new(),
                by_key: BTreeMap::new(),
                next_id: 0,
                next_hint: None,
                max_queues: SYSV_MSGMNI_DEFAULT,
            },
            sem: SysvSemRegistry {
                by_id: BTreeMap::new(),
                by_key: BTreeMap::new(),
                next_id: 0,
                max_sets: SYSV_SEMMNI_DEFAULT,
                max_per_set: SYSV_SEMMSL,
                max_ops: SYSV_SEMOPM,
            },
            shm: SysvShmRegistry {
                by_id: BTreeMap::new(),
                by_key: BTreeMap::new(),
                next_id: 0,
                next_hint: None,
            },
        }));
    }
    &SYSV
}

fn futex_bucket_index(key: FutexKey) -> usize {
    let raw = match key {
        FutexKey::Shared { uaddr } => uaddr.rotate_right(3),
        FutexKey::Private { process, uaddr } => process.rotate_left(7) ^ uaddr.rotate_right(5),
    };
    raw % FUTEX_BUCKET_COUNT
}

fn futex_key(process: &UserProcess, uaddr: usize, op: u32) -> FutexKey {
    if op & general::FUTEX_PRIVATE_FLAG as u32 != 0 {
        FutexKey::Private {
            process: process as *const UserProcess as usize,
            uaddr,
        }
    } else {
        FutexKey::Shared { uaddr }
    }
}

fn user_thread_table() -> &'static Mutex<BTreeMap<i32, UserThreadEntry>> {
    static USER_THREADS: LazyInit<Mutex<BTreeMap<i32, UserThreadEntry>>> = LazyInit::new();
    if !USER_THREADS.is_inited() {
        USER_THREADS.init_once(Mutex::new(BTreeMap::new()));
    }
    &USER_THREADS
}

fn inet_listener_table() -> &'static Mutex<BTreeMap<u16, Arc<InetListenerState>>> {
    static INET_LISTENERS: LazyInit<Mutex<BTreeMap<u16, Arc<InetListenerState>>>> = LazyInit::new();
    if !INET_LISTENERS.is_inited() {
        INET_LISTENERS.init_once(Mutex::new(BTreeMap::new()));
    }
    &INET_LISTENERS
}

fn next_inet_port() -> u16 {
    static NEXT_INET_PORT: AtomicU32 = AtomicU32::new(40000);
    NEXT_INET_PORT.fetch_add(1, Ordering::AcqRel) as u16
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

fn deliver_user_signal(entry: &UserThreadEntry, sig: i32) -> Result<(), LinuxError> {
    if sig == 0 {
        return Ok(());
    }
    let ext = task_ext(&entry.task).ok_or(LinuxError::ESRCH)?;
    ext.pending_signal.store(sig, Ordering::Release);
    if sig == SIGCANCEL_NUM {
        user_trace!(
            "sigdbg: deliver tid={} blocked={} futex_wait={:#x}",
            entry.task.id().as_u64(),
            signal_is_blocked(ext, sig),
            ext.futex_wait.load(Ordering::Acquire),
        );
    }
    if sig == SIGCANCEL_NUM && !signal_is_blocked(ext, sig) {
        let state = ext.futex_wait_state.lock().clone();
        if let Some(state) = state {
            state.seq.fetch_add(1, Ordering::Release);
            let _ = state.queue.notify_task(true, &entry.task);
        }
    }
    if !signal_is_blocked(ext, sig) {
        notify_all_sysv_msg_waiters();
        notify_all_sysv_sem_waiters();
    }
    Ok(())
}

fn futex_state(key: FutexKey) -> Arc<FutexState> {
    let bucket = &futex_buckets()[futex_bucket_index(key)];
    let mut table = bucket.lock();
    table
        .entry(key)
        .or_insert_with(|| {
            Arc::new(FutexState {
                seq: AtomicU32::new(0),
                waiters: AtomicUsize::new(0),
                queue: WaitQueue::new(),
            })
        })
        .clone()
}

fn futex_wake(key: FutexKey, count: usize) -> usize {
    let bucket = &futex_buckets()[futex_bucket_index(key)];
    let Some(state) = bucket.lock().get(&key).cloned() else {
        return 0;
    };
    state.seq.fetch_add(1, Ordering::Release);
    let mut woken = 0usize;
    for _ in 0..count {
        if !state.queue.notify_one(true) {
            break;
        }
        woken += 1;
    }
    woken
}

fn arm_current_futex_wait(uaddr: usize, state: &Arc<FutexState>) {
    if let Some(ext) = current_task_ext() {
        ext.futex_wait.store(uaddr, Ordering::Release);
        *ext.futex_wait_state.lock() = Some(state.clone());
    }
}

fn clear_current_futex_wait() {
    if let Some(ext) = current_task_ext() {
        ext.futex_wait.store(0, Ordering::Release);
        *ext.futex_wait_state.lock() = None;
    }
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
    let key = futex_key(
        ext.process.as_ref(),
        clear_tid,
        general::FUTEX_PRIVATE_FLAG as u32,
    );
    let _ = futex_wake(key, 1);
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

fn current_sigcancel_pending() -> bool {
    current_task_ext().is_some_and(|ext| {
        ext.pending_signal.load(Ordering::Acquire) == SIGCANCEL_NUM
            && !signal_is_blocked(ext, SIGCANCEL_NUM)
    })
}

fn current_unblocked_signal_pending() -> bool {
    current_task_ext().is_some_and(|ext| {
        let sig = ext.pending_signal.load(Ordering::Acquire);
        sig != 0 && !signal_is_blocked(ext, sig)
    })
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
    #[cfg(target_arch = "riscv64")]
    if ext.signal_frame.load(Ordering::Acquire) == 0 {
        let sig = ext.pending_signal.load(Ordering::Acquire);
        if sig != 0 && !signal_is_blocked(ext, sig) {
            let _ = inject_pending_signal(tf, ext, sig);
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

#[cfg(target_arch = "riscv64")]
fn make_riscv_siginfo(sig: i32, code: i32, tid: i32) -> RiscvSignalInfo {
    let mut info = RiscvSignalInfo { bytes: [0; 128] };
    info.bytes[0..4].copy_from_slice(&sig.to_ne_bytes());
    info.bytes[4..8].copy_from_slice(&0i32.to_ne_bytes());
    info.bytes[8..12].copy_from_slice(&code.to_ne_bytes());
    info.bytes[16..20].copy_from_slice(&tid.to_ne_bytes());
    info.bytes[20..24].copy_from_slice(&0u32.to_ne_bytes());
    info
}

#[cfg(target_arch = "riscv64")]
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
        let _ = aspace.handle_page_fault(VirtAddr::from(page), PageFaultFlags::WRITE);
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
    if sig >= 32 {
        user_trace!(
            "sigdbg: inject tid={} sig={sig} handler={handler:#x} flags={:#x} sp={:#x} tp={:#x}",
            current_tid(),
            action.sa_flags,
            tf.regs.sp,
            tf.regs.tp,
        );
    }
    if handler <= 1 {
        ext.pending_signal.store(0, Ordering::Release);
        return Ok(());
    }
    let current_mask = ext.signal_mask.load(Ordering::Acquire);
    let frame_size = size_of::<RiscvSignalFrame>();
    let frame_addr = align_down(tf.regs.sp.saturating_sub(frame_size), 16);
    ensure_signal_frame_pages(ext.process.as_ref(), frame_addr, frame_size)?;

    let frame = RiscvSignalFrame {
        info: make_riscv_siginfo(sig, SI_TKILL_CODE, current_tid()),
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
            mcontext: trap_frame_to_riscv_sigcontext(tf),
        },
        trampoline: RISCV_SIGTRAMP_CODE,
    };

    let frame_ret = write_user_value(ext.process.as_ref(), frame_addr, &frame);
    if frame_ret != 0 {
        return Err(LinuxError::EFAULT);
    }

    *ext.pending_sigreturn.lock() = Some(*tf);
    ext.signal_frame.store(frame_addr, Ordering::Release);
    ext.pending_signal.store(0, Ordering::Release);
    let mut next_mask = current_mask | action.sa_mask.sig[0];
    if action.sa_flags & SA_NODEFER_FLAG == 0 {
        next_mask |= signal_mask_bit(sig);
    }
    ext.signal_mask.store(next_mask, Ordering::Release);
    if sig >= 32 {
        user_trace!(
            "sigdbg: frame tid={} sig={sig} frame_addr={frame_addr:#x} size={frame_size:#x}",
            current_tid(),
        );
    }

    tf.regs.sp = frame_addr;
    tf.regs.ra = frame_addr + offset_of!(RiscvSignalFrame, trampoline);
    tf.regs.a0 = sig as usize;
    tf.regs.a1 = frame_addr + offset_of!(RiscvSignalFrame, info);
    tf.regs.a2 = frame_addr + offset_of!(RiscvSignalFrame, ucontext);
    tf.sepc = handler;
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
        general::__NR_io_setup => sys_io_setup(&process, tf.arg0(), tf.arg1()),
        general::__NR_io_destroy => sys_io_destroy(&process, tf.arg0()),
        general::__NR_io_submit => sys_io_submit(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_io_getevents => sys_io_getevents(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
        general::__NR_read => sys_read(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_pread64 => sys_pread64(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3()),
        general::__NR_write => sys_write(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_writev => sys_writev(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_readv => sys_readv(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_getcwd => sys_getcwd(&process, tf.arg0(), tf.arg1()),
        general::__NR_chdir => sys_chdir(&process, tf.arg0()),
        general::__NR_openat => sys_openat(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3()),
        general::__NR_mkdirat => sys_mkdirat(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_mknodat => sys_mknodat(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3()),
        general::__NR_unlinkat => sys_unlinkat(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_socket => sys_socket(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_socketpair => {
            sys_socketpair(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_bind => sys_bind(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_listen => sys_listen(&process, tf.arg0(), tf.arg1()),
        general::__NR_connect => sys_connect(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_getsockname => sys_getsockname(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_shutdown => sys_shutdown(&process, tf.arg0(), tf.arg1()),
        general::__NR_eventfd2 => sys_eventfd2(&process, tf.arg0(), tf.arg1()),
        general::__NR_epoll_create1 => sys_epoll_create1(&process, tf.arg0()),
        general::__NR_epoll_ctl => {
            sys_epoll_ctl(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_timerfd_create => sys_timerfd_create(&process, tf.arg0(), tf.arg1()),
        general::__NR_timerfd_settime => {
            sys_timerfd_settime(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_timerfd_gettime => sys_timerfd_gettime(&process, tf.arg0(), tf.arg1()),
        general::__NR_pipe2 => sys_pipe2(&process, tf.arg0(), tf.arg1()),
        general::__NR_ftruncate => sys_ftruncate(&process, tf.arg0(), tf.arg1()),
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
        general::__NR_getdents64 => sys_getdents64(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_lseek => sys_lseek(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_dup => sys_dup(&process, tf.arg0()),
        general::__NR_dup3 => sys_dup3(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_fcntl => sys_fcntl(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_pselect6 => sys_pselect6(
            &process,
            tf.arg0() as i32,
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
            tf.arg5(),
        ),
        general::__NR_ppoll => sys_ppoll(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
        general::__NR_epoll_pwait => sys_epoll_pwait(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2() as i32,
            tf.arg3() as i32,
            tf.arg4(),
            tf.arg5(),
        ),
        general::__NR_epoll_pwait2 => sys_epoll_pwait2(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2() as i32,
            tf.arg3(),
            tf.arg4(),
            tf.arg5(),
        ),
        general::__NR_ioctl => sys_ioctl(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_clock_gettime => sys_clock_gettime(&process, tf.arg0(), tf.arg1()),
        general::__NR_clock_getres => sys_clock_getres(&process, tf.arg0(), tf.arg1()),
        general::__NR_gettimeofday => sys_gettimeofday(&process, tf.arg0(), tf.arg1()),
        general::__NR_setitimer => sys_setitimer(&process, tf.arg0() as i32, tf.arg1(), tf.arg2()),
        general::__NR_times => sys_times(&process, tf.arg0()),
        general::__NR_getrusage => sys_getrusage(&process, tf.arg0() as i32, tf.arg1()),
        general::__NR_uname => sys_uname(&process, tf.arg0()),
        general::__NR_nanosleep => sys_nanosleep(&process, tf.arg0(), tf.arg1()),
        general::__NR_clock_nanosleep => {
            sys_clock_nanosleep(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
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
        general::__NR_gettid => axtask::current().id().as_u64() as isize,
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
        general::__NR_unshare => sys_unshare(&process, tf.arg0()),
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
        general::__NR_futex_waitv => sys_futex_waitv(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
        general::__NR_msgget => sys_msgget(&process, tf.arg0() as i32, tf.arg1() as i32),
        general::__NR_msgsnd => sys_msgsnd(
            &process,
            tf.arg0() as i32,
            tf.arg1(),
            tf.arg2(),
            tf.arg3() as i32,
        ),
        general::__NR_msgrcv => sys_msgrcv(
            &process,
            tf.arg0() as i32,
            tf.arg1(),
            tf.arg2(),
            tf.arg3() as isize,
            tf.arg4() as i32,
        ),
        general::__NR_msgctl => sys_msgctl(&process, tf.arg0() as i32, tf.arg1() as i32, tf.arg2()),
        general::__NR_semget => sys_semget(
            &process,
            tf.arg0() as i32,
            tf.arg1() as i32,
            tf.arg2() as i32,
        ),
        general::__NR_semctl => sys_semctl(
            &process,
            tf.arg0() as i32,
            tf.arg1() as i32,
            tf.arg2() as i32,
            tf.arg3(),
        ),
        general::__NR_semop => sys_semop(&process, tf.arg0() as i32, tf.arg1(), tf.arg2(), None),
        general::__NR_semtimedop => sys_semop(
            &process,
            tf.arg0() as i32,
            tf.arg1(),
            tf.arg2(),
            Some(tf.arg3()),
        ),
        general::__NR_shmget => sys_shmget(&process, tf.arg0() as i32, tf.arg1(), tf.arg2() as i32),
        general::__NR_shmat => sys_shmat(&process, tf.arg0() as i32, tf.arg1(), tf.arg2() as i32),
        general::__NR_shmdt => sys_shmdt(&process, tf.arg0()),
        general::__NR_shmctl => sys_shmctl(&process, tf.arg0() as i32, tf.arg1() as i32, tf.arg2()),
        general::__NR_getuid => process.creds().ruid as isize,
        general::__NR_geteuid => process.effective_uid() as isize,
        general::__NR_getgid => process.creds().rgid as isize,
        general::__NR_getegid => process.effective_gid() as isize,
        general::__NR_getgroups => sys_getgroups(&process, tf.arg0() as i32, tf.arg1()),
        general::__NR_setgroups => sys_setgroups(&process, tf.arg0(), tf.arg1()),
        general::__NR_setuid => sys_setuid(&process, tf.arg0() as u32),
        general::__NR_setgid => sys_setgid(&process, tf.arg0() as u32),
        general::__NR_setreuid => sys_setreuid(&process, tf.arg0(), tf.arg1()),
        general::__NR_setregid => sys_setregid(&process, tf.arg0(), tf.arg1()),
        general::__NR_setresuid => sys_setresuid(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_setresgid => sys_setresgid(&process, tf.arg0(), tf.arg1(), tf.arg2()),
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
        general::__NR_prlimit64 => sys_prlimit64(
            &process,
            tf.arg0() as i32,
            tf.arg1() as u32,
            tf.arg2(),
            tf.arg3(),
        ),
        general::__NR_getpid => process.pid() as isize,
        general::__NR_getppid => process.ppid() as isize,
        general::__NR_clone => sys_clone(
            &process,
            tf,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
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
        process.fds.lock().read(process, fd as i32, dst)
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

fn sys_write(process: &UserProcess, fd: usize, buf: usize, count: usize) -> isize {
    with_readable_slice(process, buf, count, |src| {
        process.fds.lock().write(process, fd as i32, src)
    })
}

fn sys_sched_yield(_tf: &TrapFrame) -> isize {
    axtask::yield_now();
    0
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
    let (read_end, write_end) = match PipeEndpoint::new_pair_with_flags(flags as u32) {
        Ok(pair) => pair,
        Err(err) => return neg_errno(err),
    };
    let fds = {
        let mut table = process.fds.lock();
        let read_fd = match table.insert(FdEntry::Pipe(read_end)) {
            Ok(fd) => fd,
            Err(err) => return neg_errno(err),
        };
        let write_fd = match table.insert(FdEntry::Pipe(write_end)) {
            Ok(fd) => fd,
            Err(err) => {
                let _ = table.close(read_fd);
                return neg_errno(err);
            }
        };
        [read_fd, write_fd]
    };
    write_user_value(process, pipefd, &fds)
}

fn sys_eventfd2(process: &UserProcess, initval: usize, flags: usize) -> isize {
    let event = match EventFdEntry::new(initval as u32, flags as u32) {
        Ok(event) => event,
        Err(err) => return neg_errno(err),
    };
    match process.fds.lock().insert(FdEntry::Event(event)) {
        Ok(fd) => fd as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_epoll_create1(process: &UserProcess, flags: usize) -> isize {
    let epoll = match EpollEntry::new(flags as u32) {
        Ok(epoll) => epoll,
        Err(err) => return neg_errno(err),
    };
    match process.fds.lock().insert(FdEntry::Epoll(epoll)) {
        Ok(fd) => fd as isize,
        Err(err) => neg_errno(err),
    }
}

fn duration_to_timespec(duration: core::time::Duration) -> general::timespec {
    general::timespec {
        tv_sec: duration.as_secs() as _,
        tv_nsec: duration.subsec_nanos() as _,
    }
}

fn timespec_to_duration(ts: &general::timespec) -> Result<core::time::Duration, LinuxError> {
    if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000 {
        return Err(LinuxError::EINVAL);
    }
    Ok(core::time::Duration::new(
        ts.tv_sec as u64,
        ts.tv_nsec as u32,
    ))
}

fn timerfd_remaining(
    process: &UserProcess,
    timer: &TimerFdEntry,
) -> Result<general::itimerspec, LinuxError> {
    let now = clock_now_duration(process, timer.shared.clockid)?;
    let mut state = timer.shared.state.lock();
    TimerFdEntry::refresh_state(&mut state, now)?;
    let remaining = state
        .next_deadline
        .and_then(|deadline| deadline.checked_sub(now))
        .unwrap_or(core::time::Duration::ZERO);
    Ok(general::itimerspec {
        it_interval: duration_to_timespec(state.interval),
        it_value: duration_to_timespec(remaining),
    })
}

fn sys_timerfd_create(process: &UserProcess, clockid: usize, flags: usize) -> isize {
    let timer = match TimerFdEntry::new(clockid as u32, flags as u32) {
        Ok(timer) => timer,
        Err(err) => return neg_errno(err),
    };
    match process.fds.lock().insert(FdEntry::Timer(timer)) {
        Ok(fd) => fd as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_timerfd_settime(
    process: &UserProcess,
    fd: usize,
    flags: usize,
    new_value: usize,
    old_value: usize,
) -> isize {
    let supported = general::TFD_TIMER_ABSTIME | general::TFD_TIMER_CANCEL_ON_SET;
    if flags as u32 & !supported != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let new_value = match read_user_value::<general::itimerspec>(process, new_value) {
        Ok(value) => value,
        Err(err) => return neg_errno(err),
    };
    let new_interval = match timespec_to_duration(&new_value.it_interval) {
        Ok(value) => value,
        Err(err) => return neg_errno(err),
    };
    let new_deadline = match timespec_to_duration(&new_value.it_value) {
        Ok(value) => value,
        Err(err) => return neg_errno(err),
    };
    let timer = {
        let table = process.fds.lock();
        match table.entry(fd as i32) {
            Ok(FdEntry::Timer(timer)) => timer.clone(),
            Ok(_) => return neg_errno(LinuxError::EINVAL),
            Err(err) => return neg_errno(err),
        }
    };
    if old_value != 0 {
        let old = match timerfd_remaining(process, &timer) {
            Ok(old) => old,
            Err(err) => return neg_errno(err),
        };
        let ret = write_user_value(process, old_value, &old);
        if ret != 0 {
            return ret;
        }
    }
    let now = match clock_now_duration(process, timer.shared.clockid) {
        Ok(now) => now,
        Err(err) => return neg_errno(err),
    };
    let mut state = timer.shared.state.lock();
    state.interval = new_interval;
    state.pending_ticks = 0;
    state.next_deadline = if new_deadline.is_zero() {
        None
    } else if flags as u32 & general::TFD_TIMER_ABSTIME != 0 {
        Some(new_deadline)
    } else {
        now.checked_add(new_deadline)
    };
    drop(state);
    timer.shared.wait.notify_all(true);
    0
}

fn sys_timerfd_gettime(process: &UserProcess, fd: usize, curr_value: usize) -> isize {
    let timer = {
        let table = process.fds.lock();
        match table.entry(fd as i32) {
            Ok(FdEntry::Timer(timer)) => timer.clone(),
            Ok(_) => return neg_errno(LinuxError::EINVAL),
            Err(err) => return neg_errno(err),
        }
    };
    let curr = match timerfd_remaining(process, &timer) {
        Ok(curr) => curr,
        Err(err) => return neg_errno(err),
    };
    write_user_value(process, curr_value, &curr)
}

fn sys_unshare(process: &UserProcess, flags: usize) -> isize {
    if flags as u32 == general::CLONE_NEWTIME {
        process.set_child_time_namespace();
        return 0;
    }
    if flags == 0 {
        return 0;
    }
    neg_errno(LinuxError::EINVAL)
}

fn lookup_aio_context(process: &UserProcess, ctx: u64) -> Result<Arc<AioContext>, LinuxError> {
    process
        .aio_contexts
        .lock()
        .get(&ctx)
        .cloned()
        .ok_or(LinuxError::EINVAL)
}

fn sys_io_setup(process: &UserProcess, nr_events: usize, ctxp: usize) -> isize {
    if ctxp == 0 {
        return neg_errno(LinuxError::EFAULT);
    }
    if nr_events == 0 || nr_events > i32::MAX as usize {
        return neg_errno(LinuxError::EINVAL);
    }
    let current = match read_user_value::<u64>(process, ctxp) {
        Ok(ctx) => ctx,
        Err(err) => return neg_errno(err),
    };
    if current != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let ctx = process.next_aio_context.fetch_add(1, Ordering::AcqRel);
    process
        .aio_contexts
        .lock()
        .insert(ctx, Arc::new(AioContext::new(nr_events)));
    write_user_value(process, ctxp, &ctx)
}

fn sys_io_destroy(process: &UserProcess, ctx: usize) -> isize {
    if process.aio_contexts.lock().remove(&(ctx as u64)).is_some() {
        0
    } else {
        neg_errno(LinuxError::EINVAL)
    }
}

fn sys_io_submit(process: &UserProcess, ctx: usize, nr: usize, iocbpp: usize) -> isize {
    let ctx = match lookup_aio_context(process, ctx as u64) {
        Ok(ctx) => ctx,
        Err(err) => return neg_errno(err),
    };
    if nr == 0 {
        return 0;
    }
    let Some(iocb_ptrs) = user_bytes(process, iocbpp, nr * size_of::<usize>(), false) else {
        return neg_errno(LinuxError::EFAULT);
    };
    let mut submitted = 0isize;
    for index in 0..nr {
        let ptr_bytes = &iocb_ptrs[index * size_of::<usize>()..(index + 1) * size_of::<usize>()];
        let iocb_ptr = unsafe { ptr::read_unaligned(ptr_bytes.as_ptr() as *const usize) };
        if iocb_ptr == 0 {
            return neg_errno(LinuxError::EFAULT);
        }
        let iocb = match read_user_value::<LinuxIocb>(process, iocb_ptr) {
            Ok(iocb) => iocb,
            Err(err) => return neg_errno(err),
        };
        let res = match iocb.aio_lio_opcode {
            IOCB_CMD_PWRITE => {
                let Some(buf) = user_bytes(
                    process,
                    iocb.aio_buf as usize,
                    iocb.aio_nbytes as usize,
                    false,
                ) else {
                    return neg_errno(LinuxError::EFAULT);
                };
                let mut table = process.fds.lock();
                match table.write_file_at(iocb.aio_fildes as i32, iocb.aio_offset as u64, buf) {
                    Ok(written) => written as i64,
                    Err(err) => return neg_errno(err),
                }
            }
            IOCB_CMD_PREAD => {
                let mut table = process.fds.lock();
                let data = match table.read_file_at(
                    iocb.aio_fildes as i32,
                    iocb.aio_offset as u64,
                    iocb.aio_nbytes as usize,
                ) {
                    Ok(data) => data,
                    Err(err) => return neg_errno(err),
                };
                let Some(dst) = user_bytes_mut(process, iocb.aio_buf as usize, data.len(), true)
                else {
                    return neg_errno(LinuxError::EFAULT);
                };
                dst[..data.len()].copy_from_slice(&data);
                data.len() as i64
            }
            _ => return neg_errno(LinuxError::EINVAL),
        };

        if iocb.aio_flags & IOCB_FLAG_RESFD != 0 {
            let event = {
                let table = process.fds.lock();
                match table.entry(iocb.aio_resfd as i32) {
                    Ok(FdEntry::Event(event)) => Some(event.clone()),
                    _ => None,
                }
            };
            if let Some(event) = event {
                event.kernel_signal(1);
            }
        }

        ctx.push_completion(LinuxIoEvent {
            data: iocb.aio_data,
            obj: iocb_ptr as u64,
            res,
            res2: 0,
        });
        submitted += 1;
    }
    submitted
}

fn sys_io_getevents(
    process: &UserProcess,
    ctx: usize,
    min_nr: usize,
    nr: usize,
    events: usize,
    timeout: usize,
) -> isize {
    let ctx = match lookup_aio_context(process, ctx as u64) {
        Ok(ctx) => ctx,
        Err(err) => return neg_errno(err),
    };
    if min_nr > nr {
        return neg_errno(LinuxError::EINVAL);
    }
    if nr == 0 {
        return 0;
    }
    if events == 0 {
        return neg_errno(LinuxError::EFAULT);
    }
    if timeout != 0 && read_user_value::<general::timespec>(process, timeout).is_err() {
        return neg_errno(LinuxError::EFAULT);
    }

    if min_nr > 0 {
        ctx.wait
            .wait_until(|| ctx.state.lock().completions.len() >= min_nr);
    }

    let mut state = ctx.state.lock();
    let count = state.completions.len().min(nr);
    let Some(dst) = user_bytes_mut(process, events, count * size_of::<LinuxIoEvent>(), true) else {
        return neg_errno(LinuxError::EFAULT);
    };
    for idx in 0..count {
        let event = state.completions.pop_front().unwrap();
        unsafe {
            ptr::write_unaligned(
                dst.as_mut_ptr().add(idx * size_of::<LinuxIoEvent>()) as *mut LinuxIoEvent,
                event,
            );
        }
    }
    count as isize
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
        let mut ready_read = [0usize; FD_SET_WORDS];
        let mut ready_write = [0usize; FD_SET_WORDS];
        let mut ready_except = [0usize; FD_SET_WORDS];
        let ready = {
            let table = process.fds.lock();
            let mut count = 0usize;
            count += match poll_fd_set(
                process,
                &table,
                nfds,
                &read_bits,
                &mut ready_read,
                SelectMode::Read,
            ) {
                Ok(count) => count,
                Err(err) => return neg_errno(err),
            };
            count += match poll_fd_set(
                process,
                &table,
                nfds,
                &write_bits,
                &mut ready_write,
                SelectMode::Write,
            ) {
                Ok(count) => count,
                Err(err) => return neg_errno(err),
            };
            count += match poll_fd_set(
                process,
                &table,
                nfds,
                &except_bits,
                &mut ready_except,
                SelectMode::Except,
            ) {
                Ok(count) => count,
                Err(err) => return neg_errno(err),
            };
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

fn sys_ppoll(
    process: &UserProcess,
    fds: usize,
    nfds: usize,
    timeout: usize,
    sigmask: usize,
    sigsetsize: usize,
) -> isize {
    if nfds > process.get_rlimit(RLIMIT_NOFILE_RESOURCE).rlim_cur as usize {
        return neg_errno(LinuxError::EINVAL);
    }
    let deadline = match read_ppoll_deadline(process, timeout) {
        Ok(deadline) => deadline,
        Err(err) => return neg_errno(err),
    };
    let mut pollfds = match read_pollfds(process, fds, nfds) {
        Ok(pollfds) => pollfds,
        Err(err) => return neg_errno(err),
    };
    let new_mask = match read_optional_sigmask(process, sigmask, sigsetsize) {
        Ok(mask) => mask,
        Err(err) => return neg_errno(err),
    };
    let Some(ext) = current_task_ext() else {
        return neg_errno(LinuxError::EINVAL);
    };
    let saved_mask = ext.signal_mask.load(Ordering::Acquire);
    if let Some(mask) = new_mask {
        ext.signal_mask.store(mask, Ordering::Release);
    }
    loop {
        let ready = {
            let table = process.fds.lock();
            let mut ready = 0usize;
            for pollfd in &mut pollfds {
                let revents = match poll_revents(process, &table, pollfd.fd, pollfd.events) {
                    Ok(revents) => revents,
                    Err(err) => {
                        ext.signal_mask.store(saved_mask, Ordering::Release);
                        return neg_errno(err);
                    }
                };
                pollfd.revents = revents;
                if revents != 0 {
                    ready += 1;
                }
            }
            ready
        };
        if ready > 0 {
            let ret = match write_pollfds(process, fds, &pollfds) {
                Ok(()) => ready as isize,
                Err(err) => neg_errno(err),
            };
            ext.signal_mask.store(saved_mask, Ordering::Release);
            return ret;
        }
        if current_unblocked_signal_pending() {
            ext.signal_mask.store(saved_mask, Ordering::Release);
            return neg_errno(LinuxError::EINTR);
        }
        if deadline.is_some_and(|ddl| axhal::time::wall_time() >= ddl) {
            let ret = match write_pollfds(process, fds, &pollfds) {
                Ok(()) => 0,
                Err(err) => neg_errno(err),
            };
            ext.signal_mask.store(saved_mask, Ordering::Release);
            return ret;
        }
        axtask::yield_now();
    }
}

fn sys_epoll_ctl(process: &UserProcess, epfd: usize, op: usize, fd: usize, event: usize) -> isize {
    let event = if op as u32 == general::EPOLL_CTL_DEL {
        None
    } else {
        if event == 0 {
            return neg_errno(LinuxError::EFAULT);
        }
        match read_user_value::<general::epoll_event>(process, event) {
            Ok(event) => Some(event),
            Err(err) => return neg_errno(err),
        }
    };
    match process
        .fds
        .lock()
        .epoll_ctl(process, epfd as i32, op as u32, fd as i32, event)
    {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_epoll_pwait(
    process: &UserProcess,
    epfd: usize,
    events: usize,
    maxevents: i32,
    timeout_ms: i32,
    sigmask: usize,
    sigsetsize: usize,
) -> isize {
    let deadline = match read_epoll_timeout_ms(timeout_ms) {
        Ok(deadline) => deadline,
        Err(err) => return neg_errno(err),
    };
    match sys_epoll_wait_common(
        process,
        epfd as i32,
        events,
        maxevents,
        deadline,
        sigmask,
        sigsetsize,
    ) {
        Ok(ret) => ret,
        Err(err) => neg_errno(err),
    }
}

fn sys_epoll_pwait2(
    process: &UserProcess,
    epfd: usize,
    events: usize,
    maxevents: i32,
    timeout: usize,
    sigmask: usize,
    sigsetsize: usize,
) -> isize {
    let deadline = if timeout == 0 {
        Ok(None)
    } else {
        read_epoll_timeout_ts(process, timeout)
    };
    match deadline.and_then(|deadline| {
        sys_epoll_wait_common(
            process,
            epfd as i32,
            events,
            maxevents,
            deadline,
            sigmask,
            sigsetsize,
        )
    }) {
        Ok(ret) => ret,
        Err(err) => neg_errno(err),
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
        let n = match process.fds.lock().write(process, fd as i32, src) {
            Ok(v) => v,
            Err(err) => return neg_errno(err),
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
        let n = match process.fds.lock().read(process, fd as i32, dst) {
            Ok(v) => v,
            Err(err) => return neg_errno(err),
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
    if open_dir_entry(abs_path.as_str()).is_err() {
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
    tls: usize,
    ctid: usize,
) -> isize {
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
                return ret;
            }
        }
        if clone_flags & general::CLONE_CHILD_SETTID as usize != 0 {
            let ret = write_user_value(child_process.as_ref(), ctid, &pid);
            if ret != 0 {
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
            futex_wait: AtomicUsize::new(0),
            futex_wait_state: Mutex::new(None),
            robust_list_head: AtomicUsize::new(0),
            robust_list_len: AtomicUsize::new(0),
            deferred_unmap_start: AtomicUsize::new(0),
            deferred_unmap_len: AtomicUsize::new(0),
            signal_frame: AtomicUsize::new(0),
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
        futex_wait: AtomicUsize::new(0),
        futex_wait_state: Mutex::new(None),
        robust_list_head: AtomicUsize::new(0),
        robust_list_len: AtomicUsize::new(0),
        deferred_unmap_start: AtomicUsize::new(0),
        deferred_unmap_len: AtomicUsize::new(0),
        signal_frame: AtomicUsize::new(0),
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

fn sys_mknodat(
    process: &UserProcess,
    dirfd: usize,
    pathname: usize,
    mode: usize,
    _dev: usize,
) -> isize {
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    match process
        .fds
        .lock()
        .mknodat(process, dirfd as i32, path.as_str(), mode as u32)
    {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
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

fn sys_socketpair(
    process: &UserProcess,
    domain: usize,
    ty: usize,
    protocol: usize,
    sv: usize,
) -> isize {
    let fds = match process
        .fds
        .lock()
        .socketpair(domain as u32, ty as u32, protocol as u32)
    {
        Ok(fds) => fds,
        Err(err) => return neg_errno(err),
    };
    write_user_value(process, sv, &fds)
}

fn sys_bind(process: &UserProcess, fd: usize, addr: usize, addrlen: usize) -> isize {
    match process.fds.lock().bind(process, fd as i32, addr, addrlen) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_listen(process: &UserProcess, fd: usize, backlog: usize) -> isize {
    match process.fds.lock().listen(fd as i32, backlog) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_connect(process: &UserProcess, fd: usize, addr: usize, addrlen: usize) -> isize {
    match process
        .fds
        .lock()
        .connect(process, fd as i32, addr, addrlen)
    {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_getsockname(process: &UserProcess, fd: usize, addr: usize, addrlen: usize) -> isize {
    match process
        .fds
        .lock()
        .getsockname(process, fd as i32, addr, addrlen)
    {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_shutdown(process: &UserProcess, fd: usize, how: usize) -> isize {
    match process.fds.lock().shutdown(fd as i32, how as u32) {
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

fn sys_ftruncate(process: &UserProcess, fd: usize, length: usize) -> isize {
    match process.fds.lock().truncate(fd as i32, length as u64) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_utimensat(
    process: &UserProcess,
    dirfd: usize,
    pathname: usize,
    _times: usize,
    _flags: usize,
) -> isize {
    if pathname == 0 {
        let table = process.fds.lock();
        return if table.entry(dirfd as i32).is_ok() {
            0
        } else {
            neg_errno(LinuxError::EBADF)
        };
    }
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    let abs_path = {
        let table = process.fds.lock();
        match resolve_dirfd_path(process, &table, dirfd as i32, path.as_str()) {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        }
    };
    match axfs::api::metadata(abs_path.as_str()) {
        Ok(_) => 0,
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
    _flags: usize,
) -> isize {
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
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
    match process.fds.lock().dup(fd as i32) {
        Ok(new_fd) => new_fd as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_dup3(process: &UserProcess, oldfd: usize, newfd: usize, flags: usize) -> isize {
    match process
        .fds
        .lock()
        .dup3(oldfd as i32, newfd as i32, flags as u32)
    {
        Ok(fd) => fd as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_fcntl(process: &UserProcess, fd: usize, cmd: usize, arg: usize) -> isize {
    match process.fds.lock().fcntl(fd as i32, cmd as u32, arg) {
        Ok(v) => v as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_ioctl(process: &UserProcess, fd: usize, req: usize, arg: usize) -> isize {
    if req as u32 == ioctl::TIOCGWINSZ {
        let winsize = general::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        if process.fds.lock().is_stdio(fd as i32) {
            return write_user_value(process, arg, &winsize);
        }
    }
    neg_errno(LinuxError::ENOTTY)
}

fn sys_clock_gettime(process: &UserProcess, clk_id: usize, tp: usize) -> isize {
    let now = match clock_now_duration(process, clk_id as u32) {
        Ok(now) => now,
        Err(err) => return neg_errno(err),
    };
    let ts = general::timespec {
        tv_sec: now.as_secs() as _,
        tv_nsec: now.subsec_nanos() as _,
    };
    write_user_value(process, tp, &ts)
}

fn sys_clock_getres(process: &UserProcess, clk_id: usize, tp: usize) -> isize {
    if let Err(err) = validate_clock_id(clk_id as u32) {
        return neg_errno(err);
    }
    if tp == 0 {
        return 0;
    }
    let ts = general::timespec {
        tv_sec: 0,
        tv_nsec: 1,
    };
    write_user_value(process, tp, &ts)
}

fn sys_gettimeofday(process: &UserProcess, tv: usize, tz: usize) -> isize {
    if tv != 0 {
        let now = axhal::time::wall_time();
        let value = general::timeval {
            tv_sec: now.as_secs() as _,
            tv_usec: now.subsec_micros() as _,
        };
        let ret = write_user_value(process, tv, &value);
        if ret != 0 {
            return ret;
        }
    }
    if tz != 0 {
        let value = general::timezone {
            tz_minuteswest: 0,
            tz_dsttime: 0,
        };
        let ret = write_user_value(process, tz, &value);
        if ret != 0 {
            return ret;
        }
    }
    0
}

fn sys_setitimer(process: &UserProcess, which: i32, new_value: usize, old_value: usize) -> isize {
    if which != general::ITIMER_REAL as i32 {
        return neg_errno(LinuxError::EINVAL);
    }
    if new_value != 0 && read_user_value::<general::itimerval>(process, new_value).is_err() {
        return neg_errno(LinuxError::EFAULT);
    }
    if old_value != 0 {
        let value: general::itimerval = unsafe { core::mem::zeroed() };
        let ret = write_user_value(process, old_value, &value);
        if ret != 0 {
            return ret;
        }
    }
    0
}

fn sys_times(process: &UserProcess, buf: usize) -> isize {
    let tms = Tms {
        tms_utime: 0,
        tms_stime: 0,
        tms_cutime: 0,
        tms_cstime: 0,
    };
    let ret = write_user_value(process, buf, &tms);
    if ret != 0 {
        return ret;
    }
    axhal::time::monotonic_time().as_millis() as isize
}

fn is_same_sched_target(process: &UserProcess, pid: i32) -> bool {
    pid == 0 || pid == current_tid() || pid == process.pid()
}

fn sys_sched_setparam(process: &UserProcess, pid: i32, param: usize) -> isize {
    if !is_same_sched_target(process, pid) {
        return neg_errno(LinuxError::ESRCH);
    }
    if param == 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    match read_user_value::<UserSchedParam>(process, param) {
        Ok(value) if value.sched_priority == 0 => 0,
        Ok(_) => neg_errno(LinuxError::EINVAL),
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
    match policy as u32 {
        0 if param.sched_priority == 0 => 0,
        general::SCHED_FIFO | general::SCHED_RR if (1..=99).contains(&param.sched_priority) => 0,
        general::SCHED_BATCH | general::SCHED_IDLE if param.sched_priority == 0 => 0,
        _ => neg_errno(LinuxError::EINVAL),
    }
}

fn sys_sched_getscheduler(process: &UserProcess, pid: i32) -> isize {
    if !is_same_sched_target(process, pid) {
        return neg_errno(LinuxError::ESRCH);
    }
    0
}

fn sys_sched_setaffinity(process: &UserProcess, pid: i32, cpusetsize: usize, mask: usize) -> isize {
    if !is_same_sched_target(process, pid) {
        return neg_errno(LinuxError::ESRCH);
    }
    if cpusetsize == 0 || mask == 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    with_readable_slice(process, mask, cpusetsize, |src| {
        if src[0] & 1 == 0 {
            return Err(LinuxError::EINVAL);
        }
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
        dst.fill(0);
        dst[0] = 1;
        Ok(cmp::min(cpusetsize, size_of::<usize>()))
    })
}

fn sys_syslog(process: &UserProcess, log_type: i32, buf: usize, len: usize) -> isize {
    match log_type {
        // SYSLOG_ACTION_READ_ALL and READ_CLEAR. Expose an empty kernel log.
        3 | 4 => {
            if len > 0 && buf != 0 {
                let ret = with_writable_slice(process, buf, len, |dst| {
                    dst[0] = 0;
                    Ok(0)
                });
                if ret != 0 {
                    return ret;
                }
            }
            0
        }
        // SYSLOG_ACTION_SIZE_BUFFER.
        10 => 0,
        // Console control operations are accepted as no-ops.
        6..=8 => 0,
        _ => neg_errno(LinuxError::EINVAL),
    }
}

fn sys_getrusage(process: &UserProcess, who: i32, usage: usize) -> isize {
    match who {
        x if x == general::RUSAGE_SELF as i32
            || x == general::RUSAGE_THREAD as i32
            || x == general::RUSAGE_CHILDREN => {}
        _ => return neg_errno(LinuxError::EINVAL),
    }
    let value: general::rusage = unsafe { core::mem::zeroed() };
    write_user_value(process, usage, &value)
}

fn sys_uname(process: &UserProcess, buf: usize) -> isize {
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
    write_user_value(process, buf, &uts)
}

fn sys_nanosleep(process: &UserProcess, req: usize, rem: usize) -> isize {
    let duration = match read_timespec_duration(process, req) {
        Ok(duration) => duration,
        Err(err) => return neg_errno(err),
    };
    axtask::sleep(duration);
    if rem != 0 {
        let zero = general::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let ret = write_user_value(process, rem, &zero);
        if ret != 0 {
            return ret;
        }
    }
    0
}

fn sys_clock_nanosleep(
    process: &UserProcess,
    clockid: usize,
    flags: usize,
    req: usize,
    rem: usize,
) -> isize {
    let duration = match read_timespec_duration(process, req) {
        Ok(duration) => duration,
        Err(err) => return neg_errno(err),
    };
    if flags as u32 & !general::TIMER_ABSTIME != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    if flags as u32 & general::TIMER_ABSTIME != 0 {
        let now = match clock_now_duration(process, clockid as u32) {
            Ok(now) => now,
            Err(err) => return neg_errno(err),
        };
        if let Some(delta) = duration.checked_sub(now) {
            axtask::sleep(delta);
        }
        return 0;
    }
    sys_nanosleep(process, req, rem)
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

fn base_clock_now_duration(clockid: u32) -> Result<core::time::Duration, LinuxError> {
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

fn apply_time_offset(
    base: core::time::Duration,
    offset: TimeOffset,
) -> Result<core::time::Duration, LinuxError> {
    let nanos =
        base.as_nanos() as i128 + (offset.secs as i128) * 1_000_000_000 + (offset.nanos as i128);
    if nanos < 0 {
        return Err(LinuxError::EINVAL);
    }
    Ok(core::time::Duration::new(
        (nanos / 1_000_000_000) as u64,
        (nanos % 1_000_000_000) as u32,
    ))
}

fn clock_now_duration(
    process: &UserProcess,
    clockid: u32,
) -> Result<core::time::Duration, LinuxError> {
    let base = base_clock_now_duration(clockid)?;
    match clockid {
        general::CLOCK_MONOTONIC | general::CLOCK_BOOTTIME => {
            let offset = process
                .time_offsets
                .lock()
                .get(&clockid)
                .copied()
                .unwrap_or_default();
            apply_time_offset(base, offset)
        }
        _ => Ok(base),
    }
}

fn validate_clock_id(clockid: u32) -> Result<(), LinuxError> {
    base_clock_now_duration(clockid).map(|_| ())
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

fn read_ppoll_deadline(
    process: &UserProcess,
    timeout: usize,
) -> Result<Option<core::time::Duration>, LinuxError> {
    if timeout == 0 {
        return Ok(None);
    }
    let ts = read_user_value::<general::timespec>(process, timeout)?;
    Ok(Some(axhal::time::wall_time() + timespec_to_duration(&ts)?))
}

fn read_epoll_timeout_ms(timeout_ms: i32) -> Result<Option<core::time::Duration>, LinuxError> {
    if timeout_ms < -1 {
        return Err(LinuxError::EINVAL);
    }
    if timeout_ms < 0 {
        return Ok(None);
    }
    Ok(Some(
        axhal::time::wall_time() + core::time::Duration::from_millis(timeout_ms as u64),
    ))
}

fn read_epoll_timeout_ts(
    process: &UserProcess,
    timeout: usize,
) -> Result<Option<core::time::Duration>, LinuxError> {
    let ts = read_user_value::<general::timespec>(process, timeout)?;
    Ok(Some(axhal::time::wall_time() + timespec_to_duration(&ts)?))
}

fn read_fd_set(process: &UserProcess, ptr: usize) -> Result<[usize; FD_SET_WORDS], LinuxError> {
    if ptr == 0 {
        return Ok([0; FD_SET_WORDS]);
    }
    Ok(read_user_value::<UserFdSet>(process, ptr)?.fds_bits)
}

fn read_optional_sigmask(
    process: &UserProcess,
    sigmask: usize,
    sigsetsize: usize,
) -> Result<Option<u64>, LinuxError> {
    if sigmask == 0 {
        return Ok(None);
    }
    if sigsetsize != 0 && sigsetsize < KERNEL_SIGSET_BYTES {
        return Err(LinuxError::EINVAL);
    }
    let Some(src) = user_bytes(process, sigmask, KERNEL_SIGSET_BYTES, false) else {
        return Err(LinuxError::EFAULT);
    };
    let mut mask = [0u8; KERNEL_SIGSET_BYTES];
    mask.copy_from_slice(src);
    Ok(Some(u64::from_ne_bytes(mask)))
}

fn read_pollfds(
    process: &UserProcess,
    fds: usize,
    nfds: usize,
) -> Result<Vec<general::pollfd>, LinuxError> {
    let mut pollfds = Vec::with_capacity(nfds);
    for idx in 0..nfds {
        pollfds.push(read_user_value::<general::pollfd>(
            process,
            fds + idx * size_of::<general::pollfd>(),
        )?);
    }
    Ok(pollfds)
}

fn write_pollfds(
    process: &UserProcess,
    fds: usize,
    pollfds: &[general::pollfd],
) -> Result<(), LinuxError> {
    for (idx, pollfd) in pollfds.iter().enumerate() {
        let ret = write_user_value(process, fds + idx * size_of::<general::pollfd>(), pollfd);
        if ret != 0 {
            return Err(LinuxError::EFAULT);
        }
    }
    Ok(())
}

fn poll_revents(
    process: &UserProcess,
    table: &FdTable,
    fd: i32,
    events: i16,
) -> Result<i16, LinuxError> {
    if fd < 0 {
        return Ok(0);
    }
    if table.entry(fd).is_err() {
        return Ok(general::POLLNVAL as i16);
    }
    let mut revents = 0i16;
    let read_mask = (general::POLLIN | general::POLLPRI | general::POLLRDHUP) as i16;
    if events & read_mask != 0 && table.poll(process, fd, SelectMode::Read)? {
        revents |= general::POLLIN as i16;
    }
    if events & general::POLLOUT as i16 != 0 && table.poll(process, fd, SelectMode::Write)? {
        revents |= general::POLLOUT as i16;
    }
    Ok(revents)
}

fn fd_entry_ready_mask(process: &UserProcess, entry: &FdEntry) -> u32 {
    match entry {
        FdEntry::Pipe(pipe) => {
            let state = pipe.poll();
            let mut mask = 0u32;
            if state.readable {
                mask |= general::EPOLLIN;
            }
            if state.writable {
                mask |= general::EPOLLOUT;
            }
            mask
        }
        FdEntry::Event(event) => {
            let state = event.poll();
            let mut mask = 0u32;
            if state.readable {
                mask |= general::EPOLLIN;
            }
            if state.writable {
                mask |= general::EPOLLOUT;
            }
            mask
        }
        FdEntry::Timer(timer) => {
            let state = timer.poll(process);
            let mut mask = 0u32;
            if state.readable {
                mask |= general::EPOLLIN;
            }
            if state.writable {
                mask |= general::EPOLLOUT;
            }
            mask
        }
        FdEntry::Socket(socket) => socket.ready_mask(process),
        _ => 0,
    }
}

fn epoll_entry_supported(entry: &FdEntry) -> bool {
    matches!(
        entry,
        FdEntry::Pipe(_)
            | FdEntry::Event(_)
            | FdEntry::Timer(_)
            | FdEntry::Socket(_)
            | FdEntry::Epoll(_)
    )
}

fn epoll_contains_shared(shared: &Arc<EpollShared>, needle: usize) -> bool {
    let me = Arc::as_ptr(shared) as usize;
    if me == needle {
        return true;
    }
    let watches = shared.watches.lock();
    for watch in watches.values() {
        if let FdEntry::Epoll(epoll) = &watch.entry {
            if epoll_contains_shared(&epoll.shared, needle) {
                return true;
            }
        }
    }
    false
}

fn epoll_nested_depth(shared: &Arc<EpollShared>) -> usize {
    fn walk(shared: &Arc<EpollShared>, seen: &mut BTreeSet<usize>) -> usize {
        let me = Arc::as_ptr(shared) as usize;
        if !seen.insert(me) {
            return 0;
        }
        let watches = shared.watches.lock();
        let mut max_depth = 1usize;
        for watch in watches.values() {
            if let FdEntry::Epoll(epoll) = &watch.entry {
                max_depth = max_depth.max(1 + walk(&epoll.shared, seen));
            }
        }
        seen.remove(&me);
        max_depth
    }
    walk(shared, &mut BTreeSet::new())
}

fn read_sockaddr_in(
    process: &UserProcess,
    addr: usize,
    addrlen: usize,
) -> Result<(u32, u16), LinuxError> {
    if addrlen < size_of::<net::sockaddr_in>() {
        return Err(LinuxError::EINVAL);
    }
    let addr_in = read_user_value::<net::sockaddr_in>(process, addr)?;
    if addr_in.sin_family as u32 != net::AF_INET {
        return Err(LinuxError::EAFNOSUPPORT);
    }
    Ok((
        u32::from_be(addr_in.sin_addr.s_addr),
        u16::from_be(addr_in.sin_port),
    ))
}

fn write_sockaddr_in(
    process: &UserProcess,
    addr: usize,
    addrlen: usize,
    port: u16,
) -> Result<(), LinuxError> {
    let len = read_user_value::<net::socklen_t>(process, addrlen)?;
    let full_len = size_of::<net::sockaddr_in>() as net::socklen_t;
    if len < full_len {
        return Err(LinuxError::EINVAL);
    }
    let sockaddr = net::sockaddr_in {
        sin_family: net::AF_INET as _,
        sin_port: port.to_be(),
        sin_addr: net::in_addr { s_addr: 0 },
        __pad: [0; 8],
    };
    let ret = write_user_value(process, addr, &sockaddr);
    if ret != 0 {
        return Err(LinuxError::EFAULT);
    }
    if write_user_value(process, addrlen, &full_len) != 0 {
        return Err(LinuxError::EFAULT);
    }
    Ok(())
}

fn sys_epoll_wait_common(
    process: &UserProcess,
    epfd: i32,
    events: usize,
    maxevents: i32,
    deadline: Option<core::time::Duration>,
    sigmask: usize,
    sigsetsize: usize,
) -> Result<isize, LinuxError> {
    if maxevents <= 0 {
        return Err(LinuxError::EINVAL);
    }
    let new_mask = read_optional_sigmask(process, sigmask, sigsetsize)?;
    let Some(ext) = current_task_ext() else {
        return Err(LinuxError::EINVAL);
    };
    let saved_mask = ext.signal_mask.load(Ordering::Acquire);
    if let Some(mask) = new_mask {
        ext.signal_mask.store(mask, Ordering::Release);
    }
    loop {
        let ready = {
            let table = process.fds.lock();
            table.epoll_wait_ready(process, epfd, maxevents as usize)?
        };
        if !ready.is_empty() {
            write_epoll_events(process, events, &ready)?;
            ext.signal_mask.store(saved_mask, Ordering::Release);
            return Ok(ready.len() as isize);
        }
        if current_unblocked_signal_pending() {
            ext.signal_mask.store(saved_mask, Ordering::Release);
            return Err(LinuxError::EINTR);
        }
        if deadline.is_some_and(|ddl| axhal::time::wall_time() >= ddl) {
            ext.signal_mask.store(saved_mask, Ordering::Release);
            return Ok(0);
        }
        axtask::yield_now();
    }
}

fn write_epoll_events(
    process: &UserProcess,
    events: usize,
    ready: &[general::epoll_event],
) -> Result<(), LinuxError> {
    for (idx, event) in ready.iter().enumerate() {
        let ret = write_user_value(
            process,
            events + idx * size_of::<general::epoll_event>(),
            event,
        );
        if ret != 0 {
            return Err(LinuxError::EFAULT);
        }
    }
    Ok(())
}

fn write_fd_set(process: &UserProcess, ptr: usize, bits: &[usize; FD_SET_WORDS]) -> isize {
    if ptr == 0 {
        return 0;
    }
    write_user_value(process, ptr, &UserFdSet { fds_bits: *bits })
}

fn poll_fd_set(
    process: &UserProcess,
    table: &FdTable,
    nfds: usize,
    requested: &[usize; FD_SET_WORDS],
    ready: &mut [usize; FD_SET_WORDS],
    mode: SelectMode,
) -> Result<usize, LinuxError> {
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
            if table.poll(process, fd as i32, mode)? {
                ready[word_idx] |= 1usize << bit_idx;
                count += 1;
            }
            bits &= bits - 1;
        }
    }
    Ok(count)
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
    _tf: &TrapFrame,
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
    let key = futex_key(process, uaddr, op);
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
    let wait_on_state = |state: Arc<FutexState>,
                         expected: u32,
                         timeout_dur: Option<core::time::Duration>| {
        let seq = state.seq.load(Ordering::Acquire);
        state.waiters.fetch_add(1, Ordering::AcqRel);
        arm_current_futex_wait(uaddr, &state);
        let wait_cond = || {
            state.seq.load(Ordering::Acquire) != seq
                || read_user_value::<u32>(process, uaddr).map_or(true, |value| value != expected)
                || current_sigcancel_pending()
        };
        let ret = if let Some(dur) = timeout_dur {
            if state.queue.wait_timeout_until(dur, wait_cond) {
                Err(LinuxError::ETIMEDOUT)
            } else if current_sigcancel_pending() {
                Err(LinuxError::EINTR)
            } else {
                Ok(0)
            }
        } else {
            state.queue.wait_until(wait_cond);
            if current_sigcancel_pending() {
                Err(LinuxError::EINTR)
            } else {
                Ok(0)
            }
        };
        state.waiters.fetch_sub(1, Ordering::AcqRel);
        clear_current_futex_wait();
        ret
    };
    match cmd {
        general::FUTEX_WAIT => {
            let current = match read_user_value::<u32>(process, uaddr) {
                Ok(value) => value,
                Err(err) => return neg_errno(err),
            };
            if current != val as u32 {
                return neg_errno(LinuxError::EAGAIN);
            }
            let state = futex_state(key);
            let timeout_dur = if timeout == 0 {
                None
            } else {
                let ts = match read_user_value::<general::timespec>(process, timeout) {
                    Ok(value) => value,
                    Err(err) => return neg_errno(err),
                };
                match timespec_to_duration(&ts) {
                    Ok(dur) => Some(dur),
                    Err(err) => return neg_errno(err),
                }
            };
            match wait_on_state(state, val as u32, timeout_dur) {
                Ok(ret) => ret,
                Err(err) => neg_errno(err),
            }
        }
        general::FUTEX_WAIT_BITSET => {
            if _val3 == 0 {
                return neg_errno(LinuxError::EINVAL);
            }
            let current = match read_user_value::<u32>(process, uaddr) {
                Ok(value) => value,
                Err(err) => return neg_errno(err),
            };
            if current != val as u32 {
                return neg_errno(LinuxError::EAGAIN);
            }
            let timeout_dur = if timeout == 0 {
                None
            } else {
                let ts = match read_user_value::<general::timespec>(process, timeout) {
                    Ok(value) => value,
                    Err(err) => return neg_errno(err),
                };
                let deadline = match timespec_to_duration(&ts) {
                    Ok(dur) => dur,
                    Err(err) => return neg_errno(err),
                };
                let clockid = if op & general::FUTEX_CLOCK_REALTIME as u32 != 0 {
                    general::CLOCK_REALTIME
                } else {
                    general::CLOCK_MONOTONIC
                };
                let now = match base_clock_now_duration(clockid) {
                    Ok(now) => now,
                    Err(err) => return neg_errno(err),
                };
                if deadline <= now {
                    return neg_errno(LinuxError::ETIMEDOUT);
                }
                Some(deadline - now)
            };
            match wait_on_state(futex_state(key), val as u32, timeout_dur) {
                Ok(ret) => ret,
                Err(err) => neg_errno(err),
            }
        }
        general::FUTEX_WAKE => futex_wake(key, val) as isize,
        general::FUTEX_CMP_REQUEUE => {
            if _uaddr2 == 0 || _uaddr2 % size_of::<u32>() != 0 {
                return neg_errno(LinuxError::EINVAL);
            }
            let wake_count = val as isize;
            let requeue_count = timeout as isize;
            if wake_count < 0 || requeue_count < 0 {
                return neg_errno(LinuxError::EINVAL);
            }
            let current = match read_user_value::<u32>(process, uaddr) {
                Ok(value) => value,
                Err(err) => return neg_errno(err),
            };
            if current != _val3 as u32 {
                return neg_errno(LinuxError::EAGAIN);
            }
            let source = futex_state(key);
            let target = futex_state(futex_key(process, _uaddr2, op));
            source.seq.fetch_add(1, Ordering::Release);
            let mut total = 0usize;
            for _ in 0..wake_count as usize {
                if !source.queue.notify_one(true) {
                    break;
                }
                total += 1;
            }
            total += source.queue.requeue(requeue_count as usize, &target.queue);
            total as isize
        }
        _ => neg_errno(LinuxError::ENOSYS),
    }
}

fn sys_futex_waitv(
    process: &UserProcess,
    waiters: usize,
    nr_waiters: usize,
    flags: usize,
    timeout: usize,
    clockid: usize,
) -> isize {
    if waiters == 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    if nr_waiters == 0 || nr_waiters > FUTEX_WAITV_MAX {
        return neg_errno(LinuxError::EINVAL);
    }
    if flags != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let clockid = clockid as u32;
    if clockid != general::CLOCK_MONOTONIC && clockid != general::CLOCK_REALTIME {
        return neg_errno(LinuxError::EINVAL);
    }
    let deadline = if timeout == 0 {
        None
    } else {
        let ts = match read_user_value::<general::timespec>(process, timeout) {
            Ok(value) => value,
            Err(err) => return neg_errno(err),
        };
        match timespec_to_duration(&ts) {
            Ok(deadline) => Some(deadline),
            Err(err) => return neg_errno(err),
        }
    };

    let mut states = Vec::with_capacity(nr_waiters);
    for idx in 0..nr_waiters {
        let waiter = match read_user_value::<general::futex_waitv>(
            process,
            waiters + idx * size_of::<general::futex_waitv>(),
        ) {
            Ok(waiter) => waiter,
            Err(err) => return neg_errno(err),
        };
        if waiter.flags & general::FUTEX_32 == 0
            || waiter.flags & !(general::FUTEX_32 | general::FUTEX_PRIVATE_FLAG) != 0
            || waiter.__reserved != 0
        {
            return neg_errno(LinuxError::EINVAL);
        }
        if waiter.uaddr == 0 {
            return neg_errno(LinuxError::EFAULT);
        }
        let uaddr = waiter.uaddr as usize;
        if uaddr % size_of::<u32>() != 0 {
            return neg_errno(LinuxError::EINVAL);
        }
        let current = match read_user_value::<u32>(process, uaddr) {
            Ok(value) => value,
            Err(err) => return neg_errno(err),
        };
        if current != waiter.val as u32 {
            return neg_errno(LinuxError::EAGAIN);
        }
        let key = futex_key(process, uaddr, waiter.flags);
        let state = futex_state(key);
        let seq = state.seq.load(Ordering::Acquire);
        states.push((idx, state, seq));
    }

    loop {
        for (idx, state, seq) in &states {
            if state.seq.load(Ordering::Acquire) != *seq {
                return *idx as isize;
            }
        }
        if current_sigcancel_pending() {
            return neg_errno(LinuxError::EINTR);
        }
        if let Some(deadline) = deadline {
            let now = match base_clock_now_duration(clockid) {
                Ok(now) => now,
                Err(err) => return neg_errno(err),
            };
            if now >= deadline {
                return neg_errno(LinuxError::ETIMEDOUT);
            }
        }
        axtask::yield_now();
    }
}

fn sys_getgroups(process: &UserProcess, size: i32, list: usize) -> isize {
    if size < 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let groups = process.supplementary_groups();
    if size == 0 {
        return groups.len() as isize;
    }
    if (size as usize) < groups.len() {
        return neg_errno(LinuxError::EINVAL);
    }
    if list == 0 {
        return neg_errno(LinuxError::EFAULT);
    }
    let Some(dst) = user_bytes_mut(process, list, size as usize * size_of::<u32>(), true) else {
        return neg_errno(LinuxError::EFAULT);
    };
    let bytes = unsafe {
        core::slice::from_raw_parts(
            groups.as_ptr() as *const u8,
            groups.len() * size_of::<u32>(),
        )
    };
    dst[..bytes.len()].copy_from_slice(bytes);
    groups.len() as isize
}

fn sys_setgroups(process: &UserProcess, size: usize, list: usize) -> isize {
    if size > MAX_GROUPS {
        return neg_errno(LinuxError::EINVAL);
    }
    let mut groups = Vec::with_capacity(size);
    if size != 0 {
        if list == 0 {
            return neg_errno(LinuxError::EFAULT);
        }
        for index in 0..size {
            match read_user_value::<u32>(process, list + index * size_of::<u32>()) {
                Ok(gid) => groups.push(gid),
                Err(err) => return neg_errno(err),
            }
        }
    }
    match process.setgroups(groups) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_setuid(process: &UserProcess, uid: u32) -> isize {
    match process.setuid(uid) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_setgid(process: &UserProcess, gid: u32) -> isize {
    match process.setgid(gid) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_setreuid(process: &UserProcess, ruid: usize, euid: usize) -> isize {
    match process.setresuid(uid_arg(ruid), uid_arg(euid), None) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_setregid(process: &UserProcess, rgid: usize, egid: usize) -> isize {
    match process.setresgid(uid_arg(rgid), uid_arg(egid), None) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_setresuid(process: &UserProcess, ruid: usize, euid: usize, suid: usize) -> isize {
    match process.setresuid(uid_arg(ruid), uid_arg(euid), uid_arg(suid)) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_setresgid(process: &UserProcess, rgid: usize, egid: usize, sgid: usize) -> isize {
    match process.setresgid(uid_arg(rgid), uid_arg(egid), uid_arg(sgid)) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn next_msg_id(registry: &mut SysvRegistry) -> i32 {
    if let Some(id) = registry.msg.next_hint.take() {
        if id >= 0 && !registry.msg.by_id.contains_key(&id) {
            registry.msg.next_id = registry.msg.next_id.max(id.saturating_add(1));
            return id;
        }
    }
    while registry.msg.by_id.contains_key(&registry.msg.next_id) {
        registry.msg.next_id = registry.msg.next_id.saturating_add(1);
    }
    let id = registry.msg.next_id;
    registry.msg.next_id = registry.msg.next_id.saturating_add(1);
    id
}

fn msg_queue_from_id(msqid: i32) -> Result<Arc<MsgQueueRecord>, LinuxError> {
    sysv_registry()
        .lock()
        .msg
        .by_id
        .get(&msqid)
        .cloned()
        .ok_or(LinuxError::EINVAL)
}

fn msg_queue_from_index(index: i32) -> Result<Arc<MsgQueueRecord>, LinuxError> {
    if index < 0 {
        return Err(LinuxError::EINVAL);
    }
    sysv_registry()
        .lock()
        .msg
        .by_id
        .values()
        .nth(index as usize)
        .cloned()
        .ok_or(LinuxError::EINVAL)
}

fn notify_all_sysv_msg_waiters() {
    let registry = sysv_registry().lock();
    for queue in registry.msg.by_id.values() {
        queue.send_wait.notify_all(true);
        queue.recv_wait.notify_all(true);
    }
}

fn msg_build_ds(queue: &MsgQueueRecord) -> LinuxMsqidDs {
    let state = queue.state.lock();
    LinuxMsqidDs {
        msg_perm: state.perm,
        msg_stime: state.stime,
        msg_rtime: state.rtime,
        msg_ctime: state.ctime,
        msg_cbytes: state.cbytes,
        msg_qnum: state.messages.len(),
        msg_qbytes: state.qbytes,
        msg_lspid: state.lspid,
        msg_lrpid: state.lrpid,
        __unused4: 0,
        __unused5: 0,
    }
}

fn msg_info_snapshot() -> LinuxMsgInfo {
    let registry = sysv_registry().lock();
    let mut queue_cnt = 0i32;
    let mut msg_cnt = 0i32;
    let mut msg_bytes = 0i32;
    for queue in registry.msg.by_id.values() {
        let state = queue.state.lock();
        queue_cnt += 1;
        msg_cnt += state.messages.len() as i32;
        msg_bytes += state.cbytes as i32;
    }
    LinuxMsgInfo {
        msgpool: queue_cnt,
        msgmap: msg_cnt,
        msgmax: SYSV_MSGMAX as i32,
        msgmnb: SYSV_MSGMNB as i32,
        msgmni: registry.msg.max_queues as i32,
        msgssz: 16,
        msgtql: msg_bytes,
        msgseg: 0,
    }
}

fn parse_msg_next_id(src: &[u8]) -> Result<Option<i32>, LinuxError> {
    let text = core::str::from_utf8(src).map_err(|_| LinuxError::EINVAL)?;
    let value = text.trim().parse::<i32>().map_err(|_| LinuxError::EINVAL)?;
    if value < -1 {
        return Err(LinuxError::EINVAL);
    }
    Ok((value >= 0).then_some(value))
}

fn parse_msgmni_limit(src: &[u8]) -> Result<usize, LinuxError> {
    let text = core::str::from_utf8(src).map_err(|_| LinuxError::EINVAL)?;
    let value = text
        .trim()
        .parse::<usize>()
        .map_err(|_| LinuxError::EINVAL)?;
    if value == 0 {
        return Err(LinuxError::EINVAL);
    }
    Ok(value)
}

fn msg_select_index(
    state: &MsgQueueState,
    msgtyp: isize,
    msgflg: i32,
) -> Result<Option<usize>, LinuxError> {
    if msgflg & MSG_COPY_FLAG != 0 {
        if msgflg & IPC_NOWAIT_FLAG == 0 || msgflg & MSG_EXCEPT_FLAG != 0 || msgtyp < 0 {
            return Err(LinuxError::EINVAL);
        }
        return Ok(state.messages.get(msgtyp as usize).map(|_| msgtyp as usize));
    }
    if msgtyp == 0 {
        return Ok((!state.messages.is_empty()).then_some(0));
    }
    if msgtyp > 0 {
        if msgflg & MSG_EXCEPT_FLAG != 0 {
            return Ok(state
                .messages
                .iter()
                .position(|msg| msg.mtype != msgtyp as i64));
        }
        return Ok(state
            .messages
            .iter()
            .position(|msg| msg.mtype == msgtyp as i64));
    }
    let limit = (-msgtyp) as i64;
    let mut best_ty = i64::MAX;
    let mut best_idx = None;
    for (idx, msg) in state.messages.iter().enumerate() {
        if msg.mtype <= limit && msg.mtype < best_ty {
            best_ty = msg.mtype;
            best_idx = Some(idx);
        }
    }
    Ok(best_idx)
}

fn sys_msgget(process: &UserProcess, key: i32, msgflg: i32) -> isize {
    if msgflg & !((MODE_MASK as i32) | IPC_CREAT_FLAG | IPC_EXCL_FLAG) != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let mut registry = sysv_registry().lock();
    if key != IPC_PRIVATE_KEY {
        if let Some(&msqid) = registry.msg.by_key.get(&key) {
            let queue = registry.msg.by_id.get(&msqid).cloned().unwrap();
            if msgflg & IPC_CREAT_FLAG != 0 && msgflg & IPC_EXCL_FLAG != 0 {
                return neg_errno(LinuxError::EEXIST);
            }
            let perm = queue.state.lock().perm;
            let requested_mode = (msgflg as u32) & MODE_MASK;
            let want_read = requested_mode & 0o444 != 0;
            let want_write = requested_mode & 0o222 != 0;
            if !shm_has_perm(process, &perm, want_read, want_write) {
                return neg_errno(LinuxError::EACCES);
            }
            return msqid as isize;
        }
        if msgflg & IPC_CREAT_FLAG == 0 {
            return neg_errno(LinuxError::ENOENT);
        }
    }
    if registry.msg.by_id.len() >= registry.msg.max_queues {
        return neg_errno(LinuxError::ENOSPC);
    }
    let msqid = next_msg_id(&mut registry);
    let creds = process.creds();
    let now = now_unix_secs();
    let queue = Arc::new(MsgQueueRecord {
        id: msqid,
        key,
        state: Mutex::new(MsgQueueState {
            perm: LinuxIpcPerm {
                key,
                uid: creds.euid,
                gid: creds.egid,
                cuid: creds.euid,
                cgid: creds.egid,
                mode: (msgflg as u32) & MODE_MASK,
                seq: 0,
                ..LinuxIpcPerm::default()
            },
            stime: 0,
            rtime: 0,
            ctime: now,
            cbytes: 0,
            qbytes: SYSV_MSGMNB,
            lspid: 0,
            lrpid: 0,
            removed: false,
            messages: VecDeque::new(),
        }),
        send_wait: WaitQueue::new(),
        recv_wait: WaitQueue::new(),
    });
    registry.msg.by_id.insert(msqid, queue);
    if key != IPC_PRIVATE_KEY {
        registry.msg.by_key.insert(key, msqid);
    }
    msqid as isize
}

fn sys_msgsnd(process: &UserProcess, msqid: i32, msgp: usize, msgsz: usize, msgflg: i32) -> isize {
    if msgsz > isize::MAX as usize || msgsz > SYSV_MSGMAX {
        return neg_errno(LinuxError::EINVAL);
    }
    let queue = match msg_queue_from_id(msqid) {
        Ok(queue) => queue,
        Err(err) => return neg_errno(err),
    };
    let perm = queue.state.lock().perm;
    if !shm_has_perm(process, &perm, false, true) {
        return neg_errno(LinuxError::EACCES);
    }
    let mtype = match read_user_value::<i64>(process, msgp) {
        Ok(mtype) => mtype,
        Err(err) => return neg_errno(err),
    };
    if mtype <= 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let Some(src) = user_bytes(process, msgp + size_of::<i64>(), msgsz, false) else {
        return neg_errno(LinuxError::EFAULT);
    };
    let payload = src.to_vec();
    loop {
        {
            let mut state = queue.state.lock();
            if state.removed {
                return neg_errno(LinuxError::EIDRM);
            }
            if state.cbytes + payload.len() <= state.qbytes {
                state.cbytes += payload.len();
                state.stime = now_unix_secs();
                state.lspid = process.pid();
                state.messages.push_back(MsgMessage {
                    mtype,
                    data: payload.clone(),
                });
                drop(state);
                queue.recv_wait.notify_all(true);
                return 0;
            }
        }
        if msgflg & IPC_NOWAIT_FLAG != 0 {
            return neg_errno(LinuxError::EAGAIN);
        }
        if current_unblocked_signal_pending() {
            return neg_errno(LinuxError::EINTR);
        }
        let _ = queue.recv_wait.notify_one(true);
        let timeout = queue
            .send_wait
            .wait_timeout(core::time::Duration::from_millis(10));
        if timeout && current_unblocked_signal_pending() {
            return neg_errno(LinuxError::EINTR);
        }
    }
}

fn sys_msgrcv(
    process: &UserProcess,
    msqid: i32,
    msgp: usize,
    msgsz: usize,
    msgtyp: isize,
    msgflg: i32,
) -> isize {
    if msgsz > isize::MAX as usize {
        return neg_errno(LinuxError::EINVAL);
    }
    let queue = match msg_queue_from_id(msqid) {
        Ok(queue) => queue,
        Err(err) => return neg_errno(err),
    };
    let perm = queue.state.lock().perm;
    if !shm_has_perm(process, &perm, true, false) {
        return neg_errno(LinuxError::EACCES);
    }
    if user_bytes_mut(process, msgp, size_of::<i64>(), true).is_none() {
        return neg_errno(LinuxError::EFAULT);
    }
    loop {
        let mut selected = None;
        let mut copied = false;
        {
            let mut state = queue.state.lock();
            if state.removed {
                return neg_errno(LinuxError::EIDRM);
            }
            match msg_select_index(&state, msgtyp, msgflg) {
                Ok(Some(index)) => {
                    let msg = &state.messages[index];
                    if msg.data.len() > msgsz && msgflg & MSG_NOERROR_FLAG == 0 {
                        return neg_errno(LinuxError::E2BIG);
                    }
                    let copy_len = cmp::min(msg.data.len(), msgsz);
                    selected = Some((msg.mtype, msg.data[..copy_len].to_vec(), copy_len));
                    copied = msgflg & MSG_COPY_FLAG != 0;
                    if !copied {
                        let removed = state.messages.remove(index).unwrap();
                        state.cbytes = state.cbytes.saturating_sub(removed.data.len());
                        state.rtime = now_unix_secs();
                        state.lrpid = process.pid();
                    }
                }
                Ok(None) => {}
                Err(err) => return neg_errno(err),
            }
        }
        if let Some((mtype, data, copy_len)) = selected {
            let ret = write_user_value(process, msgp, &mtype);
            if ret != 0 {
                return ret;
            }
            let Some(dst) = user_bytes_mut(process, msgp + size_of::<i64>(), copy_len, true) else {
                return neg_errno(LinuxError::EFAULT);
            };
            dst.copy_from_slice(&data);
            if !copied {
                queue.send_wait.notify_all(true);
            }
            return copy_len as isize;
        }
        if msgflg & IPC_NOWAIT_FLAG != 0 || msgflg & MSG_COPY_FLAG != 0 {
            return neg_errno(LinuxError::ENOMSG);
        }
        if current_unblocked_signal_pending() {
            return neg_errno(LinuxError::EINTR);
        }
        let _ = queue.send_wait.notify_one(true);
        let timeout = queue
            .recv_wait
            .wait_timeout(core::time::Duration::from_millis(10));
        if timeout && current_unblocked_signal_pending() {
            return neg_errno(LinuxError::EINTR);
        }
    }
}

fn sys_msgctl(process: &UserProcess, msqid: i32, cmd: i32, buf: usize) -> isize {
    match cmd {
        IPC_INFO_CMD | MSG_INFO_CMD => {
            let info = msg_info_snapshot();
            let ret = write_user_value(process, buf, &info);
            if ret != 0 {
                return ret;
            }
            let used = sysv_registry().lock().msg.by_id.len();
            used.saturating_sub(1) as isize
        }
        IPC_RMID_CMD => {
            let queue = match msg_queue_from_id(msqid) {
                Ok(queue) => queue,
                Err(err) => return neg_errno(err),
            };
            let perm = queue.state.lock().perm;
            if !shm_is_owner(process, &perm) {
                return neg_errno(LinuxError::EPERM);
            }
            {
                let mut registry = sysv_registry().lock();
                registry.msg.by_id.remove(&msqid);
                if queue.key != IPC_PRIVATE_KEY {
                    registry.msg.by_key.remove(&queue.key);
                }
            }
            queue.state.lock().removed = true;
            queue.send_wait.notify_all(true);
            queue.recv_wait.notify_all(true);
            0
        }
        IPC_STAT_CMD | MSG_STAT_CMD | MSG_STAT_ANY_CMD => {
            let queue = match cmd {
                IPC_STAT_CMD => match msg_queue_from_id(msqid) {
                    Ok(queue) => queue,
                    Err(err) => return neg_errno(err),
                },
                MSG_STAT_CMD | MSG_STAT_ANY_CMD => match msg_queue_from_index(msqid) {
                    Ok(queue) => queue,
                    Err(err) => return neg_errno(err),
                },
                _ => unreachable!(),
            };
            if cmd != MSG_STAT_ANY_CMD {
                let perm = queue.state.lock().perm;
                if !shm_has_perm(process, &perm, true, false) {
                    return neg_errno(LinuxError::EACCES);
                }
            }
            let ds = msg_build_ds(&queue);
            let ret = write_user_value(process, buf, &ds);
            if ret != 0 {
                return ret;
            }
            if cmd == IPC_STAT_CMD {
                0
            } else {
                queue.id as isize
            }
        }
        IPC_SET_CMD => {
            let queue = match msg_queue_from_id(msqid) {
                Ok(queue) => queue,
                Err(err) => return neg_errno(err),
            };
            let perm = queue.state.lock().perm;
            if !shm_is_owner(process, &perm) {
                return neg_errno(LinuxError::EPERM);
            }
            let ds = match read_user_value::<LinuxMsqidDs>(process, buf) {
                Ok(ds) => ds,
                Err(err) => return neg_errno(err),
            };
            let mut state = queue.state.lock();
            state.perm.mode = (state.perm.mode & !MODE_MASK) | (ds.msg_perm.mode & MODE_MASK);
            state.qbytes = cmp::max(ds.msg_qbytes, state.cbytes);
            state.ctime = now_unix_secs();
            0
        }
        _ => neg_errno(LinuxError::EINVAL),
    }
}

fn next_sem_id(registry: &mut SysvRegistry) -> i32 {
    while registry.sem.by_id.contains_key(&registry.sem.next_id) {
        registry.sem.next_id = registry.sem.next_id.saturating_add(1);
    }
    let id = registry.sem.next_id;
    registry.sem.next_id = registry.sem.next_id.saturating_add(1);
    id
}

fn sem_set_from_id(semid: i32) -> Result<Arc<SemSetRecord>, LinuxError> {
    sysv_registry()
        .lock()
        .sem
        .by_id
        .get(&semid)
        .cloned()
        .ok_or(LinuxError::EINVAL)
}

fn sem_set_from_index(index: i32) -> Result<Arc<SemSetRecord>, LinuxError> {
    if index < 0 {
        return Err(LinuxError::EINVAL);
    }
    sysv_registry()
        .lock()
        .sem
        .by_id
        .values()
        .nth(index as usize)
        .cloned()
        .ok_or(LinuxError::EINVAL)
}

fn notify_all_sysv_sem_waiters() {
    let registry = sysv_registry().lock();
    for set in registry.sem.by_id.values() {
        set.wait.notify_all(true);
    }
}

fn sem_build_ds(set: &SemSetRecord) -> LinuxSemidDs {
    let state = set.state.lock();
    LinuxSemidDs {
        sem_perm: state.perm,
        sem_otime: state.otime,
        sem_ctime: state.ctime,
        sem_nsems: state.sems.len(),
        __unused3: 0,
        __unused4: 0,
    }
}

fn sem_info_snapshot() -> LinuxSemInfo {
    let registry = sysv_registry().lock();
    let mut set_cnt = 0i32;
    let mut sem_cnt = 0i32;
    for set in registry.sem.by_id.values() {
        let state = set.state.lock();
        set_cnt += 1;
        sem_cnt += state.sems.len() as i32;
    }
    LinuxSemInfo {
        semmap: sem_cnt,
        semmni: registry.sem.max_sets as i32,
        semmns: (registry.sem.max_sets * registry.sem.max_per_set) as i32,
        semmnu: sem_cnt,
        semmsl: registry.sem.max_per_set as i32,
        semopm: registry.sem.max_ops as i32,
        semume: registry.sem.max_ops as i32,
        semusz: set_cnt,
        semvmx: SYSV_SEMVMX,
        semaem: sem_cnt,
    }
}

fn parse_sem_limits(src: &[u8]) -> Result<(usize, usize, usize, usize), LinuxError> {
    let text = core::str::from_utf8(src).map_err(|_| LinuxError::EINVAL)?;
    let parts = text
        .split_whitespace()
        .map(|part| part.parse::<usize>().map_err(|_| LinuxError::EINVAL))
        .collect::<Result<Vec<_>, _>>()?;
    if parts.len() != 4 || parts.iter().any(|value| *value == 0) {
        return Err(LinuxError::EINVAL);
    }
    Ok((parts[0], parts[1], parts[2], parts[3]))
}

fn read_sem_ops(
    process: &UserProcess,
    sops: usize,
    nsops: usize,
) -> Result<Vec<LinuxSembuf>, LinuxError> {
    let mut ops = Vec::with_capacity(nsops);
    for idx in 0..nsops {
        ops.push(read_user_value::<LinuxSembuf>(
            process,
            sops + idx * size_of::<LinuxSembuf>(),
        )?);
    }
    Ok(ops)
}

fn sem_wait_deadline(
    process: &UserProcess,
    timeout: Option<usize>,
) -> Result<Option<axhal::time::TimeValue>, LinuxError> {
    let Some(timeout) = timeout else {
        return Ok(None);
    };
    if timeout == 0 {
        return Ok(None);
    }
    let ts = read_user_value::<general::timespec>(process, timeout)?;
    Ok(Some(axhal::time::wall_time() + timespec_to_duration(&ts)?))
}

fn sys_semget(process: &UserProcess, key: i32, nsems: i32, semflg: i32) -> isize {
    if nsems < 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    if semflg & !((MODE_MASK as i32) | IPC_CREAT_FLAG | IPC_EXCL_FLAG) != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let mut registry = sysv_registry().lock();
    if key != IPC_PRIVATE_KEY {
        if let Some(&semid) = registry.sem.by_key.get(&key) {
            let set = registry.sem.by_id.get(&semid).cloned().unwrap();
            if semflg & IPC_CREAT_FLAG != 0 && semflg & IPC_EXCL_FLAG != 0 {
                return neg_errno(LinuxError::EEXIST);
            }
            let state = set.state.lock();
            if nsems as usize > state.sems.len() {
                return neg_errno(LinuxError::EINVAL);
            }
            if !shm_has_perm(process, &state.perm, true, true) {
                return neg_errno(LinuxError::EACCES);
            }
            return semid as isize;
        }
        if semflg & IPC_CREAT_FLAG == 0 {
            return neg_errno(LinuxError::ENOENT);
        }
    }
    if nsems as usize > registry.sem.max_per_set {
        return neg_errno(LinuxError::EINVAL);
    }
    if registry.sem.by_id.len() >= registry.sem.max_sets {
        return neg_errno(LinuxError::ENOSPC);
    }
    let semid = next_sem_id(&mut registry);
    let creds = process.creds();
    let now = now_unix_secs();
    let set = Arc::new(SemSetRecord {
        id: semid,
        key,
        state: Mutex::new(SemSetState {
            perm: LinuxIpcPerm {
                key,
                uid: creds.euid,
                gid: creds.egid,
                cuid: creds.euid,
                cgid: creds.egid,
                mode: (semflg as u32) & MODE_MASK,
                seq: 0,
                ..LinuxIpcPerm::default()
            },
            otime: 0,
            ctime: now,
            removed: false,
            sems: vec![
                SemState {
                    val: 0,
                    pid: 0,
                    ncnt: 0,
                    zcnt: 0,
                };
                nsems as usize
            ],
        }),
        wait: WaitQueue::new(),
    });
    registry.sem.by_id.insert(semid, set);
    if key != IPC_PRIVATE_KEY {
        registry.sem.by_key.insert(key, semid);
    }
    semid as isize
}

fn sys_semctl(process: &UserProcess, semid: i32, semnum: i32, cmd: i32, arg: usize) -> isize {
    match cmd {
        IPC_INFO_CMD | SEM_INFO_CMD => {
            let info = sem_info_snapshot();
            let ret = write_user_value(process, arg, &info);
            if ret != 0 {
                return ret;
            }
            let used = sysv_registry().lock().sem.by_id.len();
            used.saturating_sub(1) as isize
        }
        IPC_RMID_CMD => {
            let set = match sem_set_from_id(semid) {
                Ok(set) => set,
                Err(err) => return neg_errno(err),
            };
            let perm = set.state.lock().perm;
            if !shm_is_owner(process, &perm) {
                return neg_errno(LinuxError::EPERM);
            }
            {
                let mut registry = sysv_registry().lock();
                registry.sem.by_id.remove(&semid);
                if set.key != IPC_PRIVATE_KEY {
                    registry.sem.by_key.remove(&set.key);
                }
            }
            set.state.lock().removed = true;
            set.wait.notify_all(true);
            0
        }
        IPC_STAT_CMD | SEM_STAT_CMD | SEM_STAT_ANY_CMD => {
            let set = match cmd {
                IPC_STAT_CMD => match sem_set_from_id(semid) {
                    Ok(set) => set,
                    Err(err) => return neg_errno(err),
                },
                _ => match sem_set_from_index(semid) {
                    Ok(set) => set,
                    Err(err) => return neg_errno(err),
                },
            };
            if cmd != SEM_STAT_ANY_CMD {
                let perm = set.state.lock().perm;
                if !shm_has_perm(process, &perm, true, false) {
                    return neg_errno(LinuxError::EACCES);
                }
            }
            let ds = sem_build_ds(&set);
            let ret = write_user_value(process, arg, &ds);
            if ret != 0 {
                return ret;
            }
            if cmd == IPC_STAT_CMD {
                0
            } else {
                set.id as isize
            }
        }
        IPC_SET_CMD => {
            let set = match sem_set_from_id(semid) {
                Ok(set) => set,
                Err(err) => return neg_errno(err),
            };
            let perm = set.state.lock().perm;
            if !shm_is_owner(process, &perm) {
                return neg_errno(LinuxError::EPERM);
            }
            let ds = match read_user_value::<LinuxSemidDs>(process, arg) {
                Ok(ds) => ds,
                Err(err) => return neg_errno(err),
            };
            let mut state = set.state.lock();
            state.perm.mode = (state.perm.mode & !MODE_MASK) | (ds.sem_perm.mode & MODE_MASK);
            state.ctime = now_unix_secs();
            0
        }
        GETPID_CMD | GETVAL_CMD | GETNCNT_CMD | GETZCNT_CMD | SETVAL_CMD => {
            let set = match sem_set_from_id(semid) {
                Ok(set) => set,
                Err(err) => return neg_errno(err),
            };
            let mut state = set.state.lock();
            let Some(sem) = state.sems.get_mut(semnum as usize) else {
                return neg_errno(LinuxError::EINVAL);
            };
            match cmd {
                GETPID_CMD => sem.pid as isize,
                GETVAL_CMD => sem.val as isize,
                GETNCNT_CMD => sem.ncnt as isize,
                GETZCNT_CMD => sem.zcnt as isize,
                SETVAL_CMD => {
                    let val = arg as i32;
                    if !(0..=SYSV_SEMVMX).contains(&val) {
                        return neg_errno(LinuxError::ERANGE);
                    }
                    sem.val = val;
                    sem.pid = process.pid();
                    state.ctime = now_unix_secs();
                    drop(state);
                    set.wait.notify_all(true);
                    0
                }
                _ => unreachable!(),
            }
        }
        GETALL_CMD => {
            let set = match sem_set_from_id(semid) {
                Ok(set) => set,
                Err(err) => return neg_errno(err),
            };
            let state = set.state.lock();
            for (idx, sem) in state.sems.iter().enumerate() {
                let ret = write_user_value::<u16>(
                    process,
                    arg + idx * size_of::<u16>(),
                    &(sem.val as u16),
                );
                if ret != 0 {
                    return ret;
                }
            }
            0
        }
        SETALL_CMD => {
            let set = match sem_set_from_id(semid) {
                Ok(set) => set,
                Err(err) => return neg_errno(err),
            };
            let values_len = set.state.lock().sems.len();
            let mut values = Vec::with_capacity(values_len);
            for idx in 0..values_len {
                match read_user_value::<u16>(process, arg + idx * size_of::<u16>()) {
                    Ok(val) => {
                        if val as i32 > SYSV_SEMVMX {
                            return neg_errno(LinuxError::ERANGE);
                        }
                        values.push(val as i32);
                    }
                    Err(err) => return neg_errno(err),
                }
            }
            let mut state = set.state.lock();
            for (sem, value) in state.sems.iter_mut().zip(values) {
                sem.val = value;
                sem.pid = process.pid();
            }
            state.ctime = now_unix_secs();
            drop(state);
            set.wait.notify_all(true);
            0
        }
        _ => neg_errno(LinuxError::EINVAL),
    }
}

fn sys_semop(
    process: &UserProcess,
    semid: i32,
    sops: usize,
    nsops: usize,
    timeout: Option<usize>,
) -> isize {
    if nsops == 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let max_ops = sysv_registry().lock().sem.max_ops;
    if nsops > max_ops {
        return neg_errno(LinuxError::E2BIG);
    }
    let ops = match read_sem_ops(process, sops, nsops) {
        Ok(ops) => ops,
        Err(err) => return neg_errno(err),
    };
    let deadline = match sem_wait_deadline(process, timeout) {
        Ok(deadline) => deadline,
        Err(err) => return neg_errno(err),
    };
    let set = match sem_set_from_id(semid) {
        Ok(set) => set,
        Err(err) => return neg_errno(err),
    };
    let perm = set.state.lock().perm;
    if !shm_has_perm(process, &perm, false, true) {
        return neg_errno(LinuxError::EACCES);
    }

    loop {
        let mut wait_kind = None;
        {
            let mut state = set.state.lock();
            if state.removed {
                return neg_errno(LinuxError::EIDRM);
            }
            let mut can_apply = true;
            for op in &ops {
                let Some(sem) = state.sems.get_mut(op.sem_num as usize) else {
                    return neg_errno(LinuxError::EFBIG);
                };
                if op.sem_op > 0 {
                    if sem.val.saturating_add(op.sem_op as i32) > SYSV_SEMVMX {
                        return neg_errno(LinuxError::ERANGE);
                    }
                    continue;
                }
                if op.sem_op == 0 {
                    if sem.val != 0 {
                        if op.sem_flg as i32 & IPC_NOWAIT_FLAG != 0 {
                            return neg_errno(LinuxError::EAGAIN);
                        }
                        sem.zcnt += 1;
                        wait_kind = Some((op.sem_num as usize, true));
                        can_apply = false;
                        break;
                    }
                    continue;
                }
                if sem.val < -(op.sem_op as i32) {
                    if op.sem_flg as i32 & IPC_NOWAIT_FLAG != 0 {
                        return neg_errno(LinuxError::EAGAIN);
                    }
                    sem.ncnt += 1;
                    wait_kind = Some((op.sem_num as usize, false));
                    can_apply = false;
                    break;
                }
            }
            if can_apply {
                for op in &ops {
                    let sem = &mut state.sems[op.sem_num as usize];
                    sem.val += op.sem_op as i32;
                    sem.pid = process.pid();
                }
                state.otime = now_unix_secs();
                drop(state);
                set.wait.notify_all(true);
                return 0;
            }
        }
        if current_unblocked_signal_pending() {
            if let Some((idx, zero_wait)) = wait_kind {
                let mut state = set.state.lock();
                let sem = &mut state.sems[idx];
                if zero_wait {
                    sem.zcnt = sem.zcnt.saturating_sub(1);
                } else {
                    sem.ncnt = sem.ncnt.saturating_sub(1);
                }
            }
            return neg_errno(LinuxError::EINTR);
        }
        let timed_out = if let Some(deadline) = deadline {
            let now = axhal::time::wall_time();
            if now >= deadline {
                true
            } else {
                set.wait.wait_timeout(deadline - now)
            }
        } else {
            set.wait.wait();
            false
        };
        if let Some((idx, zero_wait)) = wait_kind {
            let mut state = set.state.lock();
            let sem = &mut state.sems[idx];
            if zero_wait {
                sem.zcnt = sem.zcnt.saturating_sub(1);
            } else {
                sem.ncnt = sem.ncnt.saturating_sub(1);
            }
            if state.removed {
                return neg_errno(LinuxError::EIDRM);
            }
        }
        if current_unblocked_signal_pending() {
            return neg_errno(LinuxError::EINTR);
        }
        if timed_out && deadline.is_some() {
            return neg_errno(LinuxError::EAGAIN);
        }
    }
}

fn now_unix_secs() -> i64 {
    axhal::time::wall_time().as_secs() as i64
}

fn reclaim_shm_segment(segment: &Arc<ShmSegment>) {
    global_allocator().dealloc_pages(segment.start_vaddr, segment.num_pages);
}

fn shm_segment_from_id(shmid: i32) -> Result<Arc<ShmSegment>, LinuxError> {
    sysv_registry()
        .lock()
        .shm
        .by_id
        .get(&shmid)
        .cloned()
        .ok_or(LinuxError::EINVAL)
}

fn shm_allowed_addr(addr: usize) -> bool {
    addr >= USER_MMAP_BASE && addr + PAGE_SIZE_4K < USER_STACK_TOP
}

fn choose_shm_addr(
    process: &UserProcess,
    requested: usize,
    size: usize,
    shmflg: i32,
) -> Result<usize, LinuxError> {
    if requested == 0 {
        let mut brk = process.brk.lock();
        let start = align_up(brk.next_mmap, PAGE_SIZE_4K);
        brk.next_mmap = start + size + PAGE_SIZE_4K;
        return Ok(start);
    }
    if requested % PAGE_SIZE_4K != 0 && shmflg & SHM_RND_FLAG == 0 {
        return Err(LinuxError::EINVAL);
    }
    let start = if shmflg & SHM_RND_FLAG != 0 {
        align_down(requested, PAGE_SIZE_4K)
    } else {
        requested
    };
    if shmflg & SHM_REMAP_FLAG != 0 && start < PAGE_SIZE_4K {
        return Err(LinuxError::EINVAL);
    }
    if !shm_allowed_addr(start) || start + size >= USER_STACK_TOP {
        return Err(LinuxError::EINVAL);
    }
    Ok(start)
}

fn shm_is_range_mapped(aspace: &AddrSpace, start: usize, size: usize) -> bool {
    PageIter4K::new(VirtAddr::from(start), VirtAddr::from(start + size))
        .unwrap()
        .any(|page| aspace.page_table().query(page).is_ok())
}

fn shm_build_ds(segment: &ShmSegment) -> LinuxShmIdDs {
    let meta = segment.meta.lock();
    let mut perm = meta.perm;
    if meta.removed {
        perm.mode |= SHM_DEST_MODE;
    }
    LinuxShmIdDs {
        shm_perm: perm,
        shm_segsz: segment.size,
        shm_atime: meta.atime,
        shm_dtime: meta.dtime,
        shm_ctime: meta.ctime,
        shm_cpid: meta.cpid,
        shm_lpid: meta.lpid,
        shm_nattch: meta.nattch,
        __unused4: 0,
        __unused5: 0,
    }
}

fn shm_info_snapshot() -> LinuxShmInfo {
    let registry = sysv_registry().lock();
    let mut shm_tot = 0usize;
    let mut used_ids = 0i32;
    for segment in registry.shm.by_id.values() {
        shm_tot += segment.num_pages;
        used_ids += 1;
    }
    LinuxShmInfo {
        used_ids,
        shm_tot,
        shm_rss: shm_tot,
        shm_swp: 0,
        swap_attempts: 0,
        swap_successes: 0,
    }
}

fn next_shm_id(registry: &mut SysvRegistry) -> i32 {
    if let Some(id) = registry.shm.next_hint.take() {
        if id >= 0 && !registry.shm.by_id.contains_key(&id) {
            registry.shm.next_id = registry.shm.next_id.max(id.saturating_add(1));
            return id;
        }
    }
    while registry.shm.by_id.contains_key(&registry.shm.next_id) {
        registry.shm.next_id = registry.shm.next_id.saturating_add(1);
    }
    let id = registry.shm.next_id;
    registry.shm.next_id = registry.shm.next_id.saturating_add(1);
    id
}

fn sys_shmget(process: &UserProcess, key: i32, size: usize, shmflg: i32) -> isize {
    if shmflg & !((MODE_MASK as i32) | IPC_CREAT_FLAG | IPC_EXCL_FLAG | SHM_HUGETLB_FLAG) != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    if shmflg & SHM_HUGETLB_FLAG != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    if size < SYSV_SHMMIN || size > SYSV_SHMMAX {
        return neg_errno(LinuxError::EINVAL);
    }
    let mut registry = sysv_registry().lock();
    if key != IPC_PRIVATE_KEY {
        if let Some(&shmid) = registry.shm.by_key.get(&key) {
            let segment = registry.shm.by_id.get(&shmid).cloned().unwrap();
            if shmflg & IPC_CREAT_FLAG != 0 && shmflg & IPC_EXCL_FLAG != 0 {
                return neg_errno(LinuxError::EEXIST);
            }
            if size > segment.size {
                return neg_errno(LinuxError::EINVAL);
            }
            let perm = segment.meta.lock().perm;
            let requested_mode = (shmflg as u32) & MODE_MASK;
            let want_read = requested_mode & 0o444 != 0;
            let want_write = requested_mode & 0o222 != 0;
            if !shm_has_perm(process, &perm, want_read, want_write) {
                return neg_errno(LinuxError::EACCES);
            }
            return shmid as isize;
        }
        if shmflg & IPC_CREAT_FLAG == 0 {
            return neg_errno(LinuxError::ENOENT);
        }
    }
    if registry.shm.by_id.len() >= SYSV_SHMMNI {
        return neg_errno(LinuxError::ENOSPC);
    }
    let num_pages = size.div_ceil(PAGE_SIZE_4K);
    let start_vaddr = match global_allocator().alloc_pages(num_pages, PAGE_SIZE_4K) {
        Ok(vaddr) => vaddr,
        Err(_) => return neg_errno(LinuxError::ENOMEM),
    };
    let start_paddr = virt_to_phys(VirtAddr::from(start_vaddr));
    unsafe {
        core::ptr::write_bytes(
            phys_to_virt(start_paddr).as_mut_ptr(),
            0,
            num_pages * PAGE_SIZE_4K,
        );
    }
    let shmid = next_shm_id(&mut registry);
    let now = now_unix_secs();
    let creds = process.creds();
    let segment = Arc::new(ShmSegment {
        id: shmid,
        key,
        size,
        map_size: num_pages * PAGE_SIZE_4K,
        start_vaddr,
        start_paddr,
        num_pages,
        meta: Mutex::new(ShmMeta {
            perm: LinuxIpcPerm {
                key,
                uid: creds.euid,
                gid: creds.egid,
                cuid: creds.euid,
                cgid: creds.egid,
                mode: (shmflg as u32) & MODE_MASK,
                seq: 0,
                ..LinuxIpcPerm::default()
            },
            atime: 0,
            dtime: 0,
            ctime: now,
            cpid: process.pid(),
            lpid: 0,
            nattch: 0,
            removed: false,
        }),
    });
    registry.shm.by_id.insert(shmid, segment);
    if key != IPC_PRIVATE_KEY {
        registry.shm.by_key.insert(key, shmid);
    }
    shmid as isize
}

fn sys_shmat(process: &UserProcess, shmid: i32, shmaddr: usize, shmflg: i32) -> isize {
    let segment = match shm_segment_from_id(shmid) {
        Ok(segment) => segment,
        Err(err) => return neg_errno(err),
    };
    let perm = segment.meta.lock().perm;
    let readonly = shmflg & SHM_RDONLY_FLAG != 0;
    if !shm_has_perm(process, &perm, true, !readonly) {
        return neg_errno(LinuxError::EACCES);
    }
    let start = match choose_shm_addr(process, shmaddr, segment.map_size, shmflg) {
        Ok(start) => start,
        Err(err) => return neg_errno(err),
    };
    {
        let mut aspace = process.aspace.lock();
        if shm_is_range_mapped(&aspace, start, segment.map_size) {
            if shmflg & SHM_REMAP_FLAG == 0 {
                return neg_errno(LinuxError::EINVAL);
            }
            let _ = aspace.unmap(VirtAddr::from(start), segment.map_size);
        }
    }
    match process.map_shm_attachment(segment, start, readonly, shmflg & SHM_EXEC_FLAG != 0, false) {
        Ok(addr) => addr as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_shmdt(process: &UserProcess, shmaddr: usize) -> isize {
    match process.shmdt(shmaddr) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_shmctl(process: &UserProcess, shmid: i32, cmd: i32, buf: usize) -> isize {
    match cmd {
        IPC_INFO_CMD => {
            let info = LinuxShmInfoParams {
                shmmax: SYSV_SHMMAX,
                shmmin: SYSV_SHMMIN,
                shmmni: SYSV_SHMMNI,
                shmseg: SYSV_SHMMNI,
                shmall: SYSV_SHMMAX.div_ceil(PAGE_SIZE_4K),
                __unused: [0; 4],
            };
            write_user_value(process, buf, &info)
        }
        SHM_INFO_CMD => {
            let info = shm_info_snapshot();
            if write_user_value(process, buf, &info) != 0 {
                return neg_errno(LinuxError::EFAULT);
            }
            let used = sysv_registry().lock().shm.by_id.len();
            used.saturating_sub(1) as isize
        }
        IPC_RMID_CMD => {
            let segment = match shm_segment_from_id(shmid) {
                Ok(segment) => segment,
                Err(err) => return neg_errno(err),
            };
            let perm = segment.meta.lock().perm;
            if !shm_is_owner(process, &perm) {
                return neg_errno(LinuxError::EPERM);
            }
            {
                let mut registry = sysv_registry().lock();
                registry.shm.by_id.remove(&shmid);
                if segment.key != IPC_PRIVATE_KEY {
                    registry.shm.by_key.remove(&segment.key);
                }
            }
            let mut meta = segment.meta.lock();
            meta.removed = true;
            let reclaim = meta.nattch == 0;
            drop(meta);
            if reclaim {
                reclaim_shm_segment(&segment);
            }
            0
        }
        IPC_STAT_CMD | SHM_STAT_CMD | SHM_STAT_ANY_CMD => {
            let segment = match cmd {
                IPC_STAT_CMD => match shm_segment_from_id(shmid) {
                    Ok(segment) => segment,
                    Err(err) => return neg_errno(err),
                },
                SHM_STAT_CMD | SHM_STAT_ANY_CMD => match read_shm_segment_by_index(shmid) {
                    Ok(segment) => segment,
                    Err(err) => return neg_errno(err),
                },
                _ => unreachable!(),
            };
            if cmd != SHM_STAT_ANY_CMD {
                let perm = segment.meta.lock().perm;
                if !shm_has_perm(process, &perm, true, false) {
                    return neg_errno(LinuxError::EACCES);
                }
            }
            let ds = shm_build_ds(&segment);
            if write_user_value(process, buf, &ds) != 0 {
                return neg_errno(LinuxError::EFAULT);
            }
            if cmd == IPC_STAT_CMD {
                0
            } else {
                segment.id as isize
            }
        }
        IPC_SET_CMD => {
            let segment = match shm_segment_from_id(shmid) {
                Ok(segment) => segment,
                Err(err) => return neg_errno(err),
            };
            let perm = segment.meta.lock().perm;
            if !shm_is_owner(process, &perm) {
                return neg_errno(LinuxError::EPERM);
            }
            let ds = match read_user_value::<LinuxShmIdDs>(process, buf) {
                Ok(ds) => ds,
                Err(err) => return neg_errno(err),
            };
            let mut meta = segment.meta.lock();
            meta.perm.mode = (meta.perm.mode & !MODE_MASK) | (ds.shm_perm.mode & MODE_MASK);
            meta.ctime = now_unix_secs();
            0
        }
        SHM_LOCK_CMD => {
            let segment = match shm_segment_from_id(shmid) {
                Ok(segment) => segment,
                Err(err) => return neg_errno(err),
            };
            let perm = segment.meta.lock().perm;
            if !shm_is_owner(process, &perm) {
                return neg_errno(LinuxError::EPERM);
            }
            segment.meta.lock().perm.mode |= SHM_LOCKED_MODE;
            0
        }
        SHM_UNLOCK_CMD => {
            let segment = match shm_segment_from_id(shmid) {
                Ok(segment) => segment,
                Err(err) => return neg_errno(err),
            };
            let perm = segment.meta.lock().perm;
            if !shm_is_owner(process, &perm) {
                return neg_errno(LinuxError::EPERM);
            }
            segment.meta.lock().perm.mode &= !SHM_LOCKED_MODE;
            0
        }
        _ => neg_errno(LinuxError::EINVAL),
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
        if signum >= 32 {
            let _handler = new_action
                .sa_handler_kernel
                .map(|func| func as usize)
                .unwrap_or(0);
            user_trace!(
                "sigdbg: rt_sigaction tid={} sig={} handler={_handler:#x} flags={:#x} mask={:#x}",
                current_tid(),
                signum,
                new_action.sa_flags,
                new_action.sa_mask.sig[0],
            );
        }
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
        ext.signal_mask
            .store(frame.ucontext.sigmask.sig[0], Ordering::Release);
        if ext.pending_signal.load(Ordering::Acquire) == 0 {
            user_trace!(
                "sigdbg: rt_sigreturn tid={} frame={frame_addr:#x} restore_sp={:#x} restore_tp={:#x} restore_pc={:#x}",
                current_tid(),
                restored.regs.sp,
                restored.regs.tp,
                restored.sepc,
            );
        }
        ext.signal_frame.store(0, Ordering::Release);
        *ext.pending_sigreturn.lock() = Some(restored);
        0
    }
    #[cfg(not(target_arch = "riscv64"))]
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
        if (current_mask | set_mask | next_mask) & signal_mask_bit(SIGCANCEL_NUM) != 0 {
            user_trace!(
                "sigdbg: rt_sigprocmask tid={} how={} set={set_mask:#x} old={current_mask:#x} new={next_mask:#x}",
                current_tid(),
                how,
            );
        }
        ext.signal_mask.store(next_mask, Ordering::Release);
    }
    0
}

fn sys_rt_sigtimedwait(
    process: &UserProcess,
    _set: usize,
    info: usize,
    timeout: usize,
    _sigsetsize: usize,
) -> isize {
    if timeout != 0 {
        if let Err(err) = read_user_value::<general::timespec>(process, timeout) {
            return neg_errno(err);
        }
    }
    if info != 0 {
        let Some(dst) = user_bytes_mut(process, info, 128, true) else {
            return neg_errno(LinuxError::EFAULT);
        };
        dst.fill(0);
    }
    SIGCHLD_NUM
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
    if pid == 0 || pid == process.pid() || pid == current_tid() {
        return 0;
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
    if sig >= 32 {
        user_trace!(
            "sigdbg: tkill from tid={} to tid={tid} sig={sig}",
            current_tid()
        );
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
    if sig >= 32 {
        user_trace!(
            "sigdbg: tgkill from tid={} tgid={} to tid={tid} sig={sig}",
            current_tid(),
            tgid,
        );
    }
    if let Err(err) = deliver_user_signal(&entry, sig) {
        return neg_errno(err);
    }
    0
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
    }

    0
}

fn sys_exit(process: &UserProcess, _tf: &TrapFrame, code: i32) -> ! {
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

fn sys_exit_group(process: &UserProcess, _tf: &TrapFrame, code: i32) -> ! {
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
    std::env::current_dir().unwrap_or_else(|_| "/".into())
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
        .find(|candidate| matches!(std::fs::metadata(candidate), Ok(meta) if meta.is_file()))
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
    st
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

#[cfg(feature = "auto-run-tests")]
pub(crate) fn run_internal_ipc_regressions() -> Result<(), String> {
    println!("#### INTERNAL REGRESSION START ipc-sync ####");
    let process = regression_process()?;
    let result_slot = Arc::new(Mutex::new(None::<Result<(), String>>));
    let runner_process = process.clone();
    let runner_slot = result_slot.clone();
    let runner = spawn_regression_task(process.clone(), "ipc-sync-regression", move || {
        let result = run_futex_regressions(runner_process.clone())
            .and_then(|_| run_sysv_ipc_regressions(runner_process));
        *runner_slot.lock() = Some(result);
    });
    process.set_pid(runner.id().as_u64() as i32);
    let _ = runner.join();
    process.teardown();
    let result = result_slot
        .lock()
        .take()
        .unwrap_or_else(|| Err("internal regression task did not report a result".into()));
    match &result {
        Ok(()) => println!("#### INTERNAL REGRESSION END ipc-sync PASS ####"),
        Err(err) => println!("#### INTERNAL REGRESSION END ipc-sync FAIL: {err} ####"),
    }
    result
}

#[cfg(feature = "auto-run-tests")]
fn regression_process() -> Result<Arc<UserProcess>, String> {
    let aspace = axmm::new_user_aspace(VirtAddr::from(USER_ASPACE_BASE), USER_ASPACE_SIZE)
        .map_err(|err| format!("create regression aspace failed: {err}"))?;
    Ok(Arc::new(UserProcess {
        aspace: Mutex::new(aspace),
        brk: Mutex::new(BrkState {
            start: USER_ASPACE_BASE,
            end: USER_ASPACE_BASE,
            limit: USER_ASPACE_BASE,
            next_mmap: USER_MMAP_BASE,
        }),
        fds: Mutex::new(FdTable::new()),
        aio_contexts: Mutex::new(BTreeMap::new()),
        creds: Mutex::new(UserCreds::root()),
        shm_attachments: Mutex::new(BTreeMap::new()),
        time_offsets: Mutex::new(BTreeMap::new()),
        child_time_offsets: Mutex::new(None),
        cwd: Mutex::new("/".into()),
        exec_root: Mutex::new("/".into()),
        children: Mutex::new(Vec::new()),
        rlimits: Mutex::new(BTreeMap::new()),
        signal_actions: Mutex::new(BTreeMap::new()),
        next_aio_context: AtomicU64::new(1),
        pid: AtomicI32::new(0),
        ppid: 0,
        live_threads: AtomicUsize::new(1),
        exit_group_code: AtomicI32::new(NO_EXIT_GROUP_CODE),
        exit_code: AtomicI32::new(0),
        exit_wait: WaitQueue::new(),
    }))
}

#[cfg(feature = "auto-run-tests")]
fn spawn_regression_task<F>(process: Arc<UserProcess>, name: &str, f: F) -> AxTaskRef
where
    F: FnOnce() + Send + 'static,
{
    let mut task = TaskInner::new(f, name.to_string(), 64 * 1024);
    let root = process.aspace.lock().page_table_root();
    task.ctx_mut().set_page_table_root(root);
    task.init_task_ext(UserTaskExt {
        process,
        clear_child_tid: AtomicUsize::new(0),
        pending_signal: AtomicI32::new(0),
        signal_mask: AtomicU64::new(0),
        futex_wait: AtomicUsize::new(0),
        futex_wait_state: Mutex::new(None),
        robust_list_head: AtomicUsize::new(0),
        robust_list_len: AtomicUsize::new(0),
        deferred_unmap_start: AtomicUsize::new(0),
        deferred_unmap_len: AtomicUsize::new(0),
        signal_frame: AtomicUsize::new(0),
        pending_sigreturn: Mutex::new(None),
    });
    axtask::spawn_task(task)
}

#[cfg(feature = "auto-run-tests")]
fn regression_map_rw(process: &UserProcess, size: usize) -> Result<usize, String> {
    let addr = sys_mmap(
        process,
        0,
        size,
        (general::PROT_READ | general::PROT_WRITE) as usize,
        (general::MAP_PRIVATE | general::MAP_ANONYMOUS) as usize,
        usize::MAX,
        0,
    );
    if addr < 0 {
        Err(format!("mmap({size:#x}) failed with {addr}"))
    } else {
        Ok(addr as usize)
    }
}

#[cfg(feature = "auto-run-tests")]
fn regression_write_bytes(process: &UserProcess, addr: usize, data: &[u8]) -> Result<(), String> {
    let Some(dst) = user_bytes_mut(process, addr, data.len(), true) else {
        return Err(format!("user buffer {addr:#x}+{} is not writable", data.len()));
    };
    dst.copy_from_slice(data);
    Ok(())
}

#[cfg(feature = "auto-run-tests")]
fn regression_write_value<T: Copy>(
    process: &UserProcess,
    addr: usize,
    value: &T,
) -> Result<(), String> {
    let ret = write_user_value(process, addr, value);
    if ret == 0 {
        Ok(())
    } else {
        Err(format!("write_user_value({addr:#x}) failed with {ret}"))
    }
}

#[cfg(feature = "auto-run-tests")]
fn regression_read_value<T: Copy>(process: &UserProcess, addr: usize) -> Result<T, String> {
    read_user_value(process, addr).map_err(|err| format!("read_user_value({addr:#x}) failed: {err:?}"))
}

#[cfg(feature = "auto-run-tests")]
fn regression_wait_until<F>(what: &str, mut cond: F) -> Result<(), String>
where
    F: FnMut() -> bool,
{
    for _ in 0..200 {
        if cond() {
            return Ok(());
        }
        axtask::yield_now();
        axtask::sleep(core::time::Duration::from_millis(1));
    }
    Err(format!("timed out while waiting for {what}"))
}

#[cfg(feature = "auto-run-tests")]
fn regression_next_key() -> i32 {
    static NEXT_KEY: AtomicI32 = AtomicI32::new(0x4300);
    NEXT_KEY.fetch_add(1, Ordering::AcqRel)
}

#[cfg(feature = "auto-run-tests")]
fn regression_trap_frame() -> TrapFrame {
    unsafe { core::mem::zeroed() }
}

#[cfg(feature = "auto-run-tests")]
fn run_futex_regressions(process: Arc<UserProcess>) -> Result<(), String> {
    const PENDING: isize = isize::MIN;

    let futex_addr = regression_map_rw(process.as_ref(), PAGE_SIZE_4K)?;
    regression_write_value(process.as_ref(), futex_addr, &1u32)?;
    let tf = regression_trap_frame();
    let wait_private = (general::FUTEX_WAIT | general::FUTEX_PRIVATE_FLAG) as usize;
    let wake_private = (general::FUTEX_WAKE | general::FUTEX_PRIVATE_FLAG) as usize;
    let wait_bitset_private =
        (general::FUTEX_WAIT_BITSET | general::FUTEX_PRIVATE_FLAG) as usize;
    let cmp_requeue_private =
        (general::FUTEX_CMP_REQUEUE | general::FUTEX_PRIVATE_FLAG) as usize;

    let mismatch = sys_futex(process.as_ref(), &tf, futex_addr, wait_private, 2, 0, 0, 0);
    if mismatch != neg_errno(LinuxError::EAGAIN) {
        return Err(format!("futex mismatch regression returned {mismatch}, expected EAGAIN"));
    }

    let rel_timeout = regression_map_rw(process.as_ref(), size_of::<general::timespec>())?;
    regression_write_value(
        process.as_ref(),
        rel_timeout,
        &general::timespec {
            tv_sec: 0,
            tv_nsec: 1_000_000,
        },
    )?;
    let timeout_ret = sys_futex(
        process.as_ref(),
        &tf,
        futex_addr,
        wait_private,
        1,
        rel_timeout,
        0,
        0,
    );
    if timeout_ret != neg_errno(LinuxError::ETIMEDOUT) {
        return Err(format!(
            "futex timeout regression returned {timeout_ret}, expected ETIMEDOUT"
        ));
    }

    let abs_timeout = regression_map_rw(process.as_ref(), size_of::<general::timespec>())?;
    let deadline = base_clock_now_duration(general::CLOCK_MONOTONIC)
        .map_err(|err| format!("futex wait_bitset clock read failed: {err:?}"))?
        .saturating_add(core::time::Duration::from_millis(2));
    regression_write_value(
        process.as_ref(),
        abs_timeout,
        &general::timespec {
            tv_sec: deadline.as_secs() as i64,
            tv_nsec: deadline.subsec_nanos() as i64,
        },
    )?;
    let bitset_timeout = sys_futex(
        process.as_ref(),
        &tf,
        futex_addr,
        wait_bitset_private,
        1,
        abs_timeout,
        0,
        u32::MAX as usize,
    );
    if bitset_timeout != neg_errno(LinuxError::ETIMEDOUT) {
        return Err(format!(
            "futex wait_bitset regression returned {bitset_timeout}, expected ETIMEDOUT"
        ));
    }

    regression_write_value(process.as_ref(), futex_addr, &0u32)?;
    let wake_key = futex_key(process.as_ref(), futex_addr, general::FUTEX_PRIVATE_FLAG as u32);
    let wake_result_1 = Arc::new(AtomicIsize::new(PENDING));
    let wake_result_2 = Arc::new(AtomicIsize::new(PENDING));
    let waiter1_process = process.clone();
    let waiter1_result = wake_result_1.clone();
    let waiter1 = spawn_regression_task(process.clone(), "futex-waiter-1", move || {
        let ret = sys_futex(
            waiter1_process.as_ref(),
            &regression_trap_frame(),
            futex_addr,
            wait_private,
            0,
            0,
            0,
            0,
        );
        waiter1_result.store(ret, Ordering::Release);
    });
    let waiter2_process = process.clone();
    let waiter2_result = wake_result_2.clone();
    let waiter2 = spawn_regression_task(process.clone(), "futex-waiter-2", move || {
        let ret = sys_futex(
            waiter2_process.as_ref(),
            &regression_trap_frame(),
            futex_addr,
            wait_private,
            0,
            0,
            0,
            0,
        );
        waiter2_result.store(ret, Ordering::Release);
    });
    regression_wait_until("two futex waiters", || {
        futex_state(wake_key).waiters.load(Ordering::Acquire) == 2
    })?;
    let woken_once = sys_futex(process.as_ref(), &tf, futex_addr, wake_private, 1, 0, 0, 0);
    if woken_once != 1 {
        return Err(format!("futex wake(1) returned {woken_once}, expected 1"));
    }
    regression_wait_until("one futex waiter to complete", || {
        (wake_result_1.load(Ordering::Acquire) == 0) ^ (wake_result_2.load(Ordering::Acquire) == 0)
    })?;
    let woken_twice = sys_futex(process.as_ref(), &tf, futex_addr, wake_private, 1, 0, 0, 0);
    if woken_twice != 1 {
        return Err(format!("second futex wake(1) returned {woken_twice}, expected 1"));
    }
    let _ = waiter1.join();
    let _ = waiter2.join();
    if wake_result_1.load(Ordering::Acquire) != 0 || wake_result_2.load(Ordering::Acquire) != 0 {
        return Err("futex wake count regression left a waiter blocked".into());
    }

    let futex_addr_2 = regression_map_rw(process.as_ref(), PAGE_SIZE_4K)?;
    regression_write_value(process.as_ref(), futex_addr_2, &0u32)?;
    let requeue_results = Arc::new([
        AtomicIsize::new(PENDING),
        AtomicIsize::new(PENDING),
        AtomicIsize::new(PENDING),
    ]);
    let source_key = futex_key(process.as_ref(), futex_addr, general::FUTEX_PRIVATE_FLAG as u32);
    let target_key =
        futex_key(process.as_ref(), futex_addr_2, general::FUTEX_PRIVATE_FLAG as u32);
    let mut requeue_tasks = Vec::new();
    for index in 0..3 {
        let waiter_process = process.clone();
        let results = requeue_results.clone();
        requeue_tasks.push(spawn_regression_task(
            process.clone(),
            "futex-requeue-waiter",
            move || {
                let ret = sys_futex(
                    waiter_process.as_ref(),
                    &regression_trap_frame(),
                    futex_addr,
                    wait_private,
                    0,
                    0,
                    0,
                    0,
                );
                results[index].store(ret, Ordering::Release);
            },
        ));
    }
    regression_wait_until("three cmp_requeue waiters", || {
        futex_state(source_key).waiters.load(Ordering::Acquire) == 3
    })?;
    let requeue_ret = sys_futex(
        process.as_ref(),
        &tf,
        futex_addr,
        cmp_requeue_private,
        1,
        2,
        futex_addr_2,
        0,
    );
    if requeue_ret != 3 {
        return Err(format!(
            "futex cmp_requeue regression returned {requeue_ret}, expected 3"
        ));
    }
    let _ = target_key;
    axtask::sleep(core::time::Duration::from_millis(2));
    let target_wake = sys_futex(process.as_ref(), &tf, futex_addr_2, wake_private, 2, 0, 0, 0);
    if target_wake != 2 {
        return Err(format!(
            "futex requeue target wake returned {target_wake}, expected 2"
        ));
    }
    for task in requeue_tasks {
        let _ = task.join();
    }
    if requeue_results
        .iter()
        .any(|ret| ret.load(Ordering::Acquire) != 0)
    {
        return Err("futex cmp_requeue regression did not release all waiters".into());
    }

    let waitv_addr_1 = regression_map_rw(process.as_ref(), PAGE_SIZE_4K)?;
    let waitv_addr_2 = regression_map_rw(process.as_ref(), PAGE_SIZE_4K)?;
    regression_write_value(process.as_ref(), waitv_addr_1, &0u32)?;
    regression_write_value(process.as_ref(), waitv_addr_2, &0u32)?;
    let waitv_vec_addr =
        regression_map_rw(process.as_ref(), 2 * size_of::<general::futex_waitv>())?;
    let waitv_timeout = regression_map_rw(process.as_ref(), size_of::<general::timespec>())?;
    let waitv_deadline = base_clock_now_duration(general::CLOCK_MONOTONIC)
        .map_err(|err| format!("futex waitv clock read failed: {err:?}"))?
        .saturating_add(core::time::Duration::from_millis(100));
    regression_write_value(
        process.as_ref(),
        waitv_timeout,
        &general::timespec {
            tv_sec: waitv_deadline.as_secs() as i64,
            tv_nsec: waitv_deadline.subsec_nanos() as i64,
        },
    )?;
    let waiters = [
        general::futex_waitv {
            val: 0,
            uaddr: waitv_addr_1 as u64,
            flags: general::FUTEX_32 | general::FUTEX_PRIVATE_FLAG,
            __reserved: 0,
        },
        general::futex_waitv {
            val: 0,
            uaddr: waitv_addr_2 as u64,
            flags: general::FUTEX_32 | general::FUTEX_PRIVATE_FLAG,
            __reserved: 0,
        },
    ];
    for (index, waiter) in waiters.iter().enumerate() {
        regression_write_value(
            process.as_ref(),
            waitv_vec_addr + index * size_of::<general::futex_waitv>(),
            waiter,
        )?;
    }
    let wake_second_result = Arc::new(AtomicIsize::new(PENDING));
    let wake_second_process = process.clone();
    let wake_second_result_cloned = wake_second_result.clone();
    let wake_second = spawn_regression_task(process.clone(), "futex-waitv-waker", move || {
        axtask::sleep(core::time::Duration::from_millis(5));
        let ret = sys_futex(
            wake_second_process.as_ref(),
            &regression_trap_frame(),
            waitv_addr_2,
            wake_private,
            1,
            0,
            0,
            0,
        );
        wake_second_result_cloned.store(ret, Ordering::Release);
    });
    let waitv_ret = sys_futex_waitv(
        process.as_ref(),
        waitv_vec_addr,
        2,
        0,
        waitv_timeout,
        general::CLOCK_MONOTONIC as usize,
    );
    let _ = wake_second.join();
    if wake_second_result.load(Ordering::Acquire) != 1 {
        return Err("futex waitv regression failed to wake the second futex".into());
    }
    if waitv_ret != 1 {
        return Err(format!("futex waitv regression returned {waitv_ret}, expected 1"));
    }

    Ok(())
}

#[cfg(feature = "auto-run-tests")]
fn run_sysv_ipc_regressions(process: Arc<UserProcess>) -> Result<(), String> {
    const PENDING: isize = isize::MIN;

    let msg_key = regression_next_key();
    let msqid = sys_msgget(
        process.as_ref(),
        msg_key,
        IPC_CREAT_FLAG | IPC_EXCL_FLAG | 0o600,
    );
    if msqid < 0 {
        return Err(format!("msgget create regression failed with {msqid}"));
    }
    let msqid = msqid as i32;
    let msg_dup = sys_msgget(
        process.as_ref(),
        msg_key,
        IPC_CREAT_FLAG | IPC_EXCL_FLAG | 0o600,
    );
    if msg_dup != neg_errno(LinuxError::EEXIST) {
        return Err(format!("msgget duplicate regression returned {msg_dup}, expected EEXIST"));
    }
    let msg_lookup = sys_msgget(process.as_ref(), msg_key, 0);
    if msg_lookup != msqid as isize {
        return Err(format!("msgget lookup regression returned {msg_lookup}, expected {msqid}"));
    }
    if sys_msgctl(process.as_ref(), msqid, IPC_RMID_CMD, 0) != 0 {
        return Err("msgctl IPC_RMID cleanup failed".into());
    }

    let sem_key = regression_next_key();
    let semid = sys_semget(
        process.as_ref(),
        sem_key,
        1,
        IPC_CREAT_FLAG | IPC_EXCL_FLAG | 0o600,
    );
    if semid < 0 {
        return Err(format!("semget create regression failed with {semid}"));
    }
    let semid = semid as i32;
    let sem_dup = sys_semget(
        process.as_ref(),
        sem_key,
        1,
        IPC_CREAT_FLAG | IPC_EXCL_FLAG | 0o600,
    );
    if sem_dup != neg_errno(LinuxError::EEXIST) {
        return Err(format!("semget duplicate regression returned {sem_dup}, expected EEXIST"));
    }
    let sem_lookup = sys_semget(process.as_ref(), sem_key, 1, 0);
    if sem_lookup != semid as isize {
        return Err(format!("semget lookup regression returned {sem_lookup}, expected {semid}"));
    }
    if sys_semctl(process.as_ref(), semid, 0, IPC_RMID_CMD, 0) != 0 {
        return Err("semctl IPC_RMID cleanup failed".into());
    }

    let shm_missing = sys_shmget(process.as_ref(), regression_next_key(), PAGE_SIZE_4K, 0);
    if shm_missing != neg_errno(LinuxError::ENOENT) {
        return Err(format!(
            "shmget missing-key regression returned {shm_missing}, expected ENOENT"
        ));
    }

    let shm_key = regression_next_key();
    let shmid = sys_shmget(
        process.as_ref(),
        shm_key,
        PAGE_SIZE_4K,
        IPC_CREAT_FLAG | IPC_EXCL_FLAG | 0o600,
    );
    if shmid < 0 {
        return Err(format!("shmget create regression failed with {shmid}"));
    }
    let shmid = shmid as i32;
    let shm_addr_a = sys_shmat(process.as_ref(), shmid, 0, 0);
    let shm_addr_b = sys_shmat(process.as_ref(), shmid, 0, 0);
    if shm_addr_a < 0 || shm_addr_b < 0 {
        return Err(format!(
            "shmat visibility regression failed: addr_a={shm_addr_a}, addr_b={shm_addr_b}"
        ));
    }
    let shm_addr_a = shm_addr_a as usize;
    let shm_addr_b = shm_addr_b as usize;
    regression_write_value(process.as_ref(), shm_addr_a, &0xfeed_beefu32)?;
    let mirrored = regression_read_value::<u32>(process.as_ref(), shm_addr_b)?;
    if mirrored != 0xfeed_beefu32 {
        return Err(format!(
            "shared memory visibility regression saw {mirrored:#x}, expected 0xfeedbeef"
        ));
    }
    let saved_creds = process.creds.lock().clone();
    *process.creds.lock() = UserCreds {
        ruid: 1000,
        euid: 1000,
        suid: 1000,
        rgid: 1000,
        egid: 1000,
        sgid: 1000,
        groups: vec![1000],
    };
    let denied = sys_shmat(process.as_ref(), shmid, 0, 0);
    *process.creds.lock() = saved_creds;
    if denied != neg_errno(LinuxError::EACCES) {
        return Err(format!("shmat permission regression returned {denied}, expected EACCES"));
    }
    if sys_shmctl(process.as_ref(), shmid, IPC_RMID_CMD, 0) != 0 {
        return Err("shmctl IPC_RMID regression failed".into());
    }
    if sys_shmdt(process.as_ref(), shm_addr_a) != 0 || sys_shmdt(process.as_ref(), shm_addr_b) != 0
    {
        return Err("shmdt cleanup regression failed".into());
    }

    let wake_msqid = sys_msgget(process.as_ref(), regression_next_key(), IPC_CREAT_FLAG | 0o600);
    if wake_msqid < 0 {
        return Err(format!("msgget wake regression failed with {wake_msqid}"));
    }
    let wake_msqid = wake_msqid as i32;
    let recv_buf = regression_map_rw(process.as_ref(), PAGE_SIZE_4K)?;
    let recv_result = Arc::new(AtomicIsize::new(PENDING));
    let recv_process = process.clone();
    let recv_result_slot = recv_result.clone();
    let recv_task = spawn_regression_task(process.clone(), "sysv-msgrcv-wait", move || {
        let ret = sys_msgrcv(recv_process.as_ref(), wake_msqid, recv_buf, 8, 0, 0);
        recv_result_slot.store(ret, Ordering::Release);
    });
    axtask::sleep(core::time::Duration::from_millis(2));
    let send_buf = regression_map_rw(process.as_ref(), PAGE_SIZE_4K)?;
    regression_write_value(process.as_ref(), send_buf, &1i64)?;
    regression_write_bytes(process.as_ref(), send_buf + size_of::<i64>(), b"wake-msg")?;
    let send_ret = sys_msgsnd(process.as_ref(), wake_msqid, send_buf, 8, 0);
    if send_ret != 0 {
        return Err(format!("msgsnd wake regression failed with {send_ret}"));
    }
    let _ = recv_task.join();
    if recv_result.load(Ordering::Acquire) != 8 {
        return Err(format!(
            "msgrcv wake regression returned {}, expected 8",
            recv_result.load(Ordering::Acquire)
        ));
    }
    let recv_type = regression_read_value::<i64>(process.as_ref(), recv_buf)?;
    if recv_type != 1 {
        return Err(format!("msgrcv wake regression type {recv_type}, expected 1"));
    }
    let payload = user_bytes(process.as_ref(), recv_buf + size_of::<i64>(), 8, false)
        .ok_or_else(|| "msgrcv payload buffer missing".to_string())?;
    if payload != b"wake-msg" {
        return Err(format!("msgrcv wake regression payload {:?}, expected wake-msg", payload));
    }
    if sys_msgctl(process.as_ref(), wake_msqid, IPC_RMID_CMD, 0) != 0 {
        return Err("msgctl IPC_RMID after wake regression failed".into());
    }

    let rm_msqid = sys_msgget(process.as_ref(), regression_next_key(), IPC_CREAT_FLAG | 0o600);
    if rm_msqid < 0 {
        return Err(format!("msgget removal regression failed with {rm_msqid}"));
    }
    let rm_msqid = rm_msqid as i32;
    let rm_recv_buf = regression_map_rw(process.as_ref(), PAGE_SIZE_4K)?;
    let rm_result = Arc::new(AtomicIsize::new(PENDING));
    let rm_process = process.clone();
    let rm_result_slot = rm_result.clone();
    let rm_task = spawn_regression_task(process.clone(), "sysv-msgrcv-rmid", move || {
        let ret = sys_msgrcv(rm_process.as_ref(), rm_msqid, rm_recv_buf, 4, 0, 0);
        rm_result_slot.store(ret, Ordering::Release);
    });
    axtask::sleep(core::time::Duration::from_millis(2));
    if sys_msgctl(process.as_ref(), rm_msqid, IPC_RMID_CMD, 0) != 0 {
        return Err("msgctl IPC_RMID removal regression failed".into());
    }
    let _ = rm_task.join();
    if rm_result.load(Ordering::Acquire) != neg_errno(LinuxError::EIDRM) {
        return Err(format!(
            "msgrcv removal regression returned {}, expected EIDRM",
            rm_result.load(Ordering::Acquire)
        ));
    }

    let wake_semid = sys_semget(process.as_ref(), regression_next_key(), 1, IPC_CREAT_FLAG | 0o600);
    if wake_semid < 0 {
        return Err(format!("semget wake regression failed with {wake_semid}"));
    }
    let wake_semid = wake_semid as i32;
    let semops_addr = regression_map_rw(process.as_ref(), PAGE_SIZE_4K)?;
    regression_write_value(
        process.as_ref(),
        semops_addr,
        &LinuxSembuf {
            sem_num: 0,
            sem_op: -1,
            sem_flg: 0,
        },
    )?;
    let sem_wait_result = Arc::new(AtomicIsize::new(PENDING));
    let sem_wait_process = process.clone();
    let sem_wait_slot = sem_wait_result.clone();
    let sem_wait_task = spawn_regression_task(process.clone(), "sysv-semop-wake", move || {
        let ret = sys_semop(sem_wait_process.as_ref(), wake_semid, semops_addr, 1, None);
        sem_wait_slot.store(ret, Ordering::Release);
    });
    regression_wait_until("semop waiter", || {
        sem_set_from_id(wake_semid)
            .ok()
            .and_then(|set| set.state.lock().sems.first().map(|sem| sem.ncnt == 1))
            .unwrap_or(false)
    })?;
    let setval_ret = sys_semctl(process.as_ref(), wake_semid, 0, SETVAL_CMD, 1);
    if setval_ret != 0 {
        return Err(format!("semctl SETVAL wake regression failed with {setval_ret}"));
    }
    let _ = sem_wait_task.join();
    if sem_wait_result.load(Ordering::Acquire) != 0 {
        return Err(format!(
            "semop wake regression returned {}, expected 0",
            sem_wait_result.load(Ordering::Acquire)
        ));
    }
    if sys_semctl(process.as_ref(), wake_semid, 0, IPC_RMID_CMD, 0) != 0 {
        return Err("semctl IPC_RMID wake cleanup failed".into());
    }

    let rm_semid = sys_semget(process.as_ref(), regression_next_key(), 1, IPC_CREAT_FLAG | 0o600);
    if rm_semid < 0 {
        return Err(format!("semget removal regression failed with {rm_semid}"));
    }
    let rm_semid = rm_semid as i32;
    let rm_semops = regression_map_rw(process.as_ref(), PAGE_SIZE_4K)?;
    regression_write_value(
        process.as_ref(),
        rm_semops,
        &LinuxSembuf {
            sem_num: 0,
            sem_op: -1,
            sem_flg: 0,
        },
    )?;
    let rm_sem_result = Arc::new(AtomicIsize::new(PENDING));
    let rm_sem_process = process.clone();
    let rm_sem_slot = rm_sem_result.clone();
    let rm_sem_task = spawn_regression_task(process.clone(), "sysv-semop-rmid", move || {
        let ret = sys_semop(rm_sem_process.as_ref(), rm_semid, rm_semops, 1, None);
        rm_sem_slot.store(ret, Ordering::Release);
    });
    regression_wait_until("semop removal waiter", || {
        sem_set_from_id(rm_semid)
            .ok()
            .and_then(|set| set.state.lock().sems.first().map(|sem| sem.ncnt == 1))
            .unwrap_or(false)
    })?;
    if sys_semctl(process.as_ref(), rm_semid, 0, IPC_RMID_CMD, 0) != 0 {
        return Err("semctl IPC_RMID removal regression failed".into());
    }
    let _ = rm_sem_task.join();
    if rm_sem_result.load(Ordering::Acquire) != neg_errno(LinuxError::EIDRM) {
        return Err(format!(
            "semop removal regression returned {}, expected EIDRM",
            rm_sem_result.load(Ordering::Acquire)
        ));
    }

    Ok(())
}

fn uid_arg(value: usize) -> Option<u32> {
    let value = value as u32;
    if value == u32::MAX {
        None
    } else {
        Some(value)
    }
}

fn ipc_can_read(mode: u32) -> bool {
    mode & 0o4 != 0
}

fn ipc_can_write(mode: u32) -> bool {
    mode & 0o2 != 0
}

fn ipc_access_mode(creds: &UserCreds, perm: &LinuxIpcPerm) -> u32 {
    if creds.euid == perm.uid || creds.euid == perm.cuid {
        (perm.mode >> 6) & 0o7
    } else if creds.egid == perm.gid
        || creds.egid == perm.cgid
        || creds
            .groups
            .iter()
            .any(|gid| *gid == perm.gid || *gid == perm.cgid)
    {
        (perm.mode >> 3) & 0o7
    } else {
        perm.mode & 0o7
    }
}

fn shm_has_perm(
    process: &UserProcess,
    perm: &LinuxIpcPerm,
    want_read: bool,
    want_write: bool,
) -> bool {
    if process.is_superuser() {
        return true;
    }
    let mode = ipc_access_mode(&process.creds(), perm);
    (!want_read || ipc_can_read(mode)) && (!want_write || ipc_can_write(mode))
}

fn shm_is_owner(process: &UserProcess, perm: &LinuxIpcPerm) -> bool {
    if process.is_superuser() {
        return true;
    }
    let creds = process.creds();
    creds.euid == perm.uid || creds.euid == perm.cuid
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
            Self::Event(event) => Ok(Self::Event(event.clone())),
            Self::Timer(timer) => Ok(Self::Timer(timer.clone())),
            Self::Socket(socket) => Ok(Self::Socket(socket.clone())),
            Self::Epoll(epoll) => Ok(Self::Epoll(epoll.clone())),
            Self::TimeNsOffsets(file) => Ok(Self::TimeNsOffsets(file.clone())),
            Self::ProcPseudo(file) => Ok(Self::ProcPseudo(file.clone())),
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
        Ok(Self { entries })
    }

    fn is_stdio(&self, fd: i32) -> bool {
        matches!(fd, 0..=2)
    }

    fn poll(&self, process: &UserProcess, fd: i32, mode: SelectMode) -> Result<bool, LinuxError> {
        let entry = self.entry(fd)?;
        let ready_mask = fd_entry_ready_mask(process, entry);
        match mode {
            SelectMode::Read => Ok(match entry {
                FdEntry::Stdin => false,
                FdEntry::Stdout | FdEntry::Stderr => false,
                FdEntry::DevNull | FdEntry::File(_) | FdEntry::Directory(_) => true,
                FdEntry::Pipe(_) | FdEntry::Event(_) | FdEntry::Timer(_) | FdEntry::Socket(_) => {
                    ready_mask & general::EPOLLIN != 0
                }
                FdEntry::Epoll(_) => false,
                FdEntry::TimeNsOffsets(_) => false,
                FdEntry::ProcPseudo(_) => true,
            }),
            SelectMode::Write => Ok(match entry {
                FdEntry::Stdin => false,
                FdEntry::Stdout | FdEntry::Stderr | FdEntry::DevNull => true,
                FdEntry::File(_) => true,
                FdEntry::Directory(_) => false,
                FdEntry::Pipe(_) | FdEntry::Event(_) | FdEntry::Timer(_) | FdEntry::Socket(_) => {
                    ready_mask & general::EPOLLOUT != 0
                }
                FdEntry::Epoll(_) => false,
                FdEntry::TimeNsOffsets(_) => true,
                FdEntry::ProcPseudo(file) => matches!(
                    file.kind,
                    ProcPseudoKind::KernelShmNextId
                        | ProcPseudoKind::KernelMsgMni
                        | ProcPseudoKind::KernelMsgNextId
                        | ProcPseudoKind::KernelSem
                ),
            }),
            SelectMode::Except => Ok(false),
        }
    }

    fn read(
        &mut self,
        process: &UserProcess,
        fd: i32,
        dst: &mut [u8],
    ) -> Result<usize, LinuxError> {
        match self.entry_mut(fd)? {
            FdEntry::Stdin => Ok(0),
            FdEntry::DevNull => Ok(0),
            FdEntry::File(file) => file.file.read(dst).map_err(LinuxError::from),
            FdEntry::Directory(_) => Err(LinuxError::EISDIR),
            FdEntry::Pipe(pipe) => pipe.read(dst),
            FdEntry::Event(event) => event.read(dst),
            FdEntry::Timer(timer) => timer.read(process, dst),
            FdEntry::Socket(socket) => socket.read(dst),
            FdEntry::TimeNsOffsets(_) => Err(LinuxError::EBADF),
            FdEntry::ProcPseudo(file) => file.read(dst),
            _ => Err(LinuxError::EBADF),
        }
    }

    fn write(&mut self, process: &UserProcess, fd: i32, src: &[u8]) -> Result<usize, LinuxError> {
        match self.entry_mut(fd)? {
            FdEntry::Stdout | FdEntry::Stderr => {
                axhal::console::write_bytes(src);
                Ok(src.len())
            }
            FdEntry::DevNull => Ok(src.len()),
            FdEntry::File(file) => file.file.write(src).map_err(LinuxError::from),
            FdEntry::Pipe(pipe) => pipe.write(src),
            FdEntry::Event(event) => event.write(src),
            FdEntry::Timer(_) => Err(LinuxError::EINVAL),
            FdEntry::Socket(socket) => socket.write(src),
            FdEntry::TimeNsOffsets(_) => write_timens_offsets(process, src),
            FdEntry::ProcPseudo(file) => file.write(src),
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
        self.insert(entry)
    }

    fn socket(&mut self, domain: u32, ty: u32, protocol: u32) -> Result<i32, LinuxError> {
        let sock_type = ty & !(general::O_NONBLOCK | general::O_CLOEXEC);
        if protocol != 0 {
            return Err(LinuxError::EPROTONOSUPPORT);
        }
        let entry = match (domain, sock_type) {
            (net::AF_INET, net::SOCK_STREAM) => FdEntry::Socket(SocketEntry::new_inet_stream(ty)?),
            _ => return Err(LinuxError::EAFNOSUPPORT),
        };
        self.insert(entry)
    }

    fn socketpair(&mut self, domain: u32, ty: u32, protocol: u32) -> Result<[i32; 2], LinuxError> {
        let sock_type = ty & !(general::O_NONBLOCK | general::O_CLOEXEC);
        if protocol != 0 {
            return Err(LinuxError::EPROTONOSUPPORT);
        }
        if domain != net::AF_UNIX || sock_type != net::SOCK_STREAM {
            return Err(LinuxError::EAFNOSUPPORT);
        }
        let (left, right) = SocketEntry::new_socketpair(ty)?;
        let left_fd = self.insert(FdEntry::Socket(left))?;
        let right_fd = match self.insert(FdEntry::Socket(right)) {
            Ok(fd) => fd,
            Err(err) => {
                let _ = self.close(left_fd);
                return Err(err);
            }
        };
        Ok([left_fd, right_fd])
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

    fn mknodat(
        &mut self,
        process: &UserProcess,
        dirfd: i32,
        path: &str,
        mode: u32,
    ) -> Result<(), LinuxError> {
        if mode & general::S_IFMT != general::S_IFIFO {
            return Err(LinuxError::EINVAL);
        }
        let abs_path = if path.starts_with('/') || dirfd == general::AT_FDCWD {
            let cwd = process.cwd();
            resolve_host_path(cwd, path).map_err(|_| LinuxError::EINVAL)?
        } else {
            resolve_dirfd_path(process, self, dirfd, path)?
        };
        create_fifo_placeholder(abs_path.as_str())
    }

    fn bind(
        &mut self,
        process: &UserProcess,
        fd: i32,
        addr: usize,
        addrlen: usize,
    ) -> Result<(), LinuxError> {
        let FdEntry::Socket(socket) = self.entry_mut(fd)? else {
            return Err(LinuxError::ENOTSOCK);
        };
        socket.bind(process, addr, addrlen)
    }

    fn listen(&mut self, fd: i32, backlog: usize) -> Result<(), LinuxError> {
        let FdEntry::Socket(socket) = self.entry_mut(fd)? else {
            return Err(LinuxError::ENOTSOCK);
        };
        socket.listen(backlog)
    }

    fn connect(
        &mut self,
        process: &UserProcess,
        fd: i32,
        addr: usize,
        addrlen: usize,
    ) -> Result<(), LinuxError> {
        let FdEntry::Socket(socket) = self.entry_mut(fd)? else {
            return Err(LinuxError::ENOTSOCK);
        };
        socket.connect(process, addr, addrlen)
    }

    fn getsockname(
        &mut self,
        process: &UserProcess,
        fd: i32,
        addr: usize,
        addrlen: usize,
    ) -> Result<(), LinuxError> {
        let FdEntry::Socket(socket) = self.entry_mut(fd)? else {
            return Err(LinuxError::ENOTSOCK);
        };
        socket.getsockname(process, addr, addrlen)
    }

    fn shutdown(&mut self, fd: i32, how: u32) -> Result<(), LinuxError> {
        let FdEntry::Socket(socket) = self.entry_mut(fd)? else {
            return Err(LinuxError::ENOTSOCK);
        };
        socket.shutdown(how)
    }

    fn epoll_ctl(
        &mut self,
        _process: &UserProcess,
        epfd: i32,
        op: u32,
        fd: i32,
        event: Option<general::epoll_event>,
    ) -> Result<(), LinuxError> {
        let epoll = match self.entry(epfd)? {
            FdEntry::Epoll(epoll) => epoll.clone(),
            _ => return Err(LinuxError::EINVAL),
        };
        if fd == epfd {
            return Err(LinuxError::EINVAL);
        }
        let target = if op == general::EPOLL_CTL_DEL {
            None
        } else {
            let entry = self.entry(fd)?;
            if !epoll_entry_supported(entry) {
                return Err(LinuxError::EPERM);
            }
            if let FdEntry::Epoll(target_epoll) = entry {
                if epoll_contains_shared(&target_epoll.shared, Arc::as_ptr(&epoll.shared) as usize)
                {
                    return Err(LinuxError::ELOOP);
                }
                if epoll_nested_depth(&target_epoll.shared) >= 5 {
                    return Err(LinuxError::EINVAL);
                }
            }
            Some(entry.duplicate_for_fork()?)
        };
        let mut watches = epoll.shared.watches.lock();
        match op {
            general::EPOLL_CTL_ADD => {
                if watches.contains_key(&fd) {
                    return Err(LinuxError::EEXIST);
                }
                let event = event.ok_or(LinuxError::EFAULT)?;
                watches.insert(
                    fd,
                    EpollWatch {
                        entry: target.unwrap(),
                        events: event.events,
                        data: event.data,
                        last_mask: 0,
                        oneshot_disabled: false,
                    },
                );
                Ok(())
            }
            general::EPOLL_CTL_MOD => {
                let event = event.ok_or(LinuxError::EFAULT)?;
                let watch = watches.get_mut(&fd).ok_or(LinuxError::ENOENT)?;
                watch.entry = target.unwrap();
                watch.events = event.events;
                watch.data = event.data;
                watch.last_mask = 0;
                watch.oneshot_disabled = false;
                Ok(())
            }
            general::EPOLL_CTL_DEL => {
                if watches.remove(&fd).is_some() {
                    Ok(())
                } else {
                    Err(LinuxError::ENOENT)
                }
            }
            _ => Err(LinuxError::EINVAL),
        }
    }

    fn epoll_wait_ready(
        &self,
        process: &UserProcess,
        epfd: i32,
        maxevents: usize,
    ) -> Result<Vec<general::epoll_event>, LinuxError> {
        let epoll = match self.entry(epfd)? {
            FdEntry::Epoll(epoll) => epoll.clone(),
            _ => return Err(LinuxError::EINVAL),
        };
        let mut ready = Vec::new();
        let mut watches = epoll.shared.watches.lock();
        for watch in watches.values_mut() {
            if ready.len() >= maxevents {
                break;
            }
            let edge = watch.events & general::EPOLLET != 0;
            let oneshot = watch.events & general::EPOLLONESHOT != 0;
            let interest = watch.events & !(general::EPOLLET | general::EPOLLONESHOT);
            let current = fd_entry_ready_mask(process, &watch.entry) & interest;
            let mut deliver = current;
            if edge {
                deliver &= !watch.last_mask;
            }
            if oneshot && watch.oneshot_disabled {
                deliver = 0;
            }
            watch.last_mask = current;
            if deliver == 0 {
                continue;
            }
            if oneshot {
                watch.oneshot_disabled = true;
            }
            ready.push(general::epoll_event {
                events: deliver,
                data: watch.data,
            });
        }
        Ok(ready)
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
            let abs_path = resolve_host_path(cwd, path).map_err(|_| LinuxError::EINVAL)?;
            return if remove_dir {
                directory_remove_dir(abs_path.as_str())
            } else {
                directory_remove_file(abs_path.as_str())
            };
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
        if fd <= 2 {
            return Ok(());
        }
        self.entries[fd as usize] = None;
        Ok(())
    }

    fn stat(&mut self, fd: i32) -> Result<general::stat, LinuxError> {
        match self.entry_mut(fd)? {
            FdEntry::Stdin => Ok(stdio_stat(true)),
            FdEntry::Stdout | FdEntry::Stderr => Ok(stdio_stat(false)),
            FdEntry::DevNull => Ok(stdio_stat(false)),
            FdEntry::File(file) => Ok(file_attr_to_stat(
                &file.file.get_attr().map_err(LinuxError::from)?,
                Some(file.path.as_str()),
            )),
            FdEntry::Directory(dir) => Ok(file_attr_to_stat(&dir.attr, Some(dir.path.as_str()))),
            FdEntry::Pipe(pipe) => Ok(pipe.stat()),
            FdEntry::Event(event) => Ok(event.stat()),
            FdEntry::Timer(timer) => Ok(timer.stat()),
            FdEntry::Socket(socket) => Ok(socket.stat()),
            FdEntry::Epoll(epoll) => Ok(epoll.stat()),
            FdEntry::TimeNsOffsets(_) => Ok(procfs_pseudo_stat()),
            FdEntry::ProcPseudo(_) => Ok(procfs_pseudo_stat()),
        }
    }

    fn stat_path(
        &mut self,
        process: &UserProcess,
        dirfd: i32,
        path: &str,
    ) -> Result<general::stat, LinuxError> {
        match open_fd_entry(process, self, dirfd, path, general::O_RDONLY) {
            Ok(FdEntry::File(file)) => Ok(file_attr_to_stat(
                &file.file.get_attr().map_err(LinuxError::from)?,
                Some(file.path.as_str()),
            )),
            Ok(FdEntry::Directory(dir)) => {
                Ok(file_attr_to_stat(&dir.attr, Some(dir.path.as_str())))
            }
            Ok(FdEntry::TimeNsOffsets(_) | FdEntry::ProcPseudo(_)) => Ok(procfs_pseudo_stat()),
            Ok(_) => Err(LinuxError::EINVAL),
            Err(err) => Err(err),
        }
    }

    fn truncate(&mut self, fd: i32, size: u64) -> Result<(), LinuxError> {
        match self.entry_mut(fd)? {
            FdEntry::File(file) => file.file.truncate(size).map_err(LinuxError::from),
            FdEntry::DevNull => Ok(()),
            _ => Err(LinuxError::EINVAL),
        }
    }

    fn fcntl(&mut self, fd: i32, cmd: u32, _arg: usize) -> Result<i32, LinuxError> {
        match cmd {
            general::F_DUPFD | general::F_DUPFD_CLOEXEC => self.dup_min(fd, _arg as i32),
            general::F_GETFD => match self.entry(fd)? {
                FdEntry::Pipe(pipe) => Ok(pipe.getfd()),
                FdEntry::Event(event) => Ok(event.getfd()),
                FdEntry::Timer(timer) => Ok(timer.getfd()),
                FdEntry::Socket(socket) => Ok(socket.getfd()),
                FdEntry::Epoll(epoll) => Ok(epoll.getfd()),
                _ => Ok(0),
            },
            general::F_SETFD => match self.entry_mut(fd)? {
                FdEntry::Pipe(pipe) => Ok(pipe.setfd(_arg as u32)),
                FdEntry::Event(event) => Ok(event.setfd(_arg as u32)),
                FdEntry::Timer(timer) => Ok(timer.setfd(_arg as u32)),
                FdEntry::Socket(socket) => Ok(socket.setfd(_arg as u32)),
                FdEntry::Epoll(epoll) => Ok(epoll.setfd(_arg as u32)),
                _ => Ok(0),
            },
            general::F_GETFL => match self.entry(fd)? {
                FdEntry::Pipe(pipe) => Ok(pipe.getfl()),
                FdEntry::Event(event) => Ok(event.getfl()),
                FdEntry::Timer(timer) => Ok(timer.getfl()),
                FdEntry::Socket(socket) => Ok(socket.getfl()),
                _ => Ok(0),
            },
            general::F_SETFL => match self.entry_mut(fd)? {
                FdEntry::Pipe(pipe) => Ok(pipe.setfl(_arg as u32)),
                FdEntry::Event(event) => Ok(event.setfl(_arg as u32)),
                FdEntry::Timer(timer) => Ok(timer.setfl(_arg as u32)),
                FdEntry::Socket(socket) => Ok(socket.setfl(_arg as u32)),
                _ => Ok(0),
            },
            general::F_SETPIPE_SZ => match self.entry(fd)? {
                FdEntry::Pipe(_) => Ok(PIPE_BUF_SIZE as i32),
                _ => Err(LinuxError::EBADF),
            },
            general::F_GETPIPE_SZ => match self.entry(fd)? {
                FdEntry::Pipe(_) => Ok(PIPE_BUF_SIZE as i32),
                _ => Err(LinuxError::EBADF),
            },
            _ => {
                let _ = self.entry(fd)?;
                Ok(0)
            }
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
            FdEntry::Event(_) => Err(LinuxError::ESPIPE),
            FdEntry::Timer(_) => Err(LinuxError::ESPIPE),
            FdEntry::TimeNsOffsets(_) => Err(LinuxError::ESPIPE),
            FdEntry::ProcPseudo(file) => file.lseek(offset, whence),
            _ => Err(LinuxError::ESPIPE),
        }
    }

    fn dup(&mut self, fd: i32) -> Result<i32, LinuxError> {
        self.dup_min(fd, 0)
    }

    fn dup_min(&mut self, fd: i32, min_fd: i32) -> Result<i32, LinuxError> {
        if min_fd < 0 {
            return Err(LinuxError::EINVAL);
        }
        let entry = self.entry(fd)?.duplicate_for_fork()?;
        self.insert_min(entry, min_fd as usize)
    }

    fn dup3(&mut self, oldfd: i32, newfd: i32, _flags: u32) -> Result<i32, LinuxError> {
        if oldfd == newfd {
            return Err(LinuxError::EINVAL);
        }
        let entry = self.entry(oldfd)?.duplicate_for_fork()?;
        if newfd < 0 {
            return Err(LinuxError::EBADF);
        }
        let newfd = newfd as usize;
        if self.entries.len() <= newfd {
            self.entries.resize_with(newfd + 1, || None);
        }
        self.entries[newfd] = Some(entry);
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

    fn write_file_at(&mut self, fd: i32, offset: u64, src: &[u8]) -> Result<usize, LinuxError> {
        let FdEntry::File(file) = self.entry_mut(fd)? else {
            return Err(LinuxError::EBADF);
        };
        file.file.write_at(offset, src).map_err(LinuxError::from)
    }

    fn insert(&mut self, entry: FdEntry) -> Result<i32, LinuxError> {
        self.insert_min(entry, 0)
    }

    fn insert_min(&mut self, entry: FdEntry, min_fd: usize) -> Result<i32, LinuxError> {
        if self.entries.len() < min_fd {
            self.entries.resize_with(min_fd, || None);
        }
        if let Some((idx, slot)) = self
            .entries
            .iter_mut()
            .enumerate()
            .skip(min_fd)
            .find(|(_, slot)| slot.is_none())
        {
            *slot = Some(entry);
            return Ok(idx as i32);
        }
        self.entries.push(Some(entry));
        Ok((self.entries.len() - 1) as i32)
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
                return open_fd_candidates(&[path], prefer_dir, &opts);
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
        open_fd_candidates(&candidates, prefer_dir, &opts)
    } else {
        let FdEntry::Directory(dir) = table.entry(dirfd)? else {
            return Err(LinuxError::ENOTDIR);
        };
        let primary = normalize_path(dir.path.as_str(), path).ok_or(LinuxError::EINVAL)?;
        let mut candidates = vec![primary];
        for extra in runtime_library_name_candidates(exec_root.as_str(), path) {
            push_runtime_candidate(&mut candidates, Some(extra));
        }
        open_fd_candidates(&candidates, prefer_dir, &opts)
    }
}

fn open_fd_candidates(
    candidates: &[String],
    prefer_dir: bool,
    opts: &OpenOptions,
) -> Result<FdEntry, LinuxError> {
    let mut last_err = LinuxError::ENOENT;
    for path in candidates {
        if path == "/dev/null" {
            if prefer_dir {
                return Err(LinuxError::ENOTDIR);
            }
            return Ok(FdEntry::DevNull);
        }
        if path == "/proc/self/timens_offsets" {
            if prefer_dir {
                return Err(LinuxError::ENOTDIR);
            }
            return Ok(FdEntry::TimeNsOffsets(TimeNsOffsetsFile));
        }
        if let Some(kind) = match path.as_str() {
            "/proc/sys/kernel/shmmax" => Some(ProcPseudoKind::KernelShmMax),
            "/proc/sys/kernel/shmmin" => Some(ProcPseudoKind::KernelShmMin),
            "/proc/sys/kernel/shmmni" => Some(ProcPseudoKind::KernelShmMni),
            "/proc/sys/kernel/shmall" => Some(ProcPseudoKind::KernelShmAll),
            "/proc/sys/kernel/shm_next_id" => Some(ProcPseudoKind::KernelShmNextId),
            "/proc/sysvipc/shm" => Some(ProcPseudoKind::SysvipcShm),
            "/proc/sys/kernel/msgmni" => Some(ProcPseudoKind::KernelMsgMni),
            "/proc/sys/kernel/msg_next_id" => Some(ProcPseudoKind::KernelMsgNextId),
            "/proc/sysvipc/msg" => Some(ProcPseudoKind::SysvipcMsg),
            "/proc/sys/kernel/sem" => Some(ProcPseudoKind::KernelSem),
            "/proc/sysvipc/sem" => Some(ProcPseudoKind::SysvipcSem),
            _ => None,
        } {
            if prefer_dir {
                return Err(LinuxError::ENOTDIR);
            }
            return Ok(FdEntry::ProcPseudo(ProcPseudoFile { kind, offset: 0 }));
        }
        if prefer_dir {
            match open_dir_entry(path.as_str()) {
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
                return Ok(FdEntry::File(FileEntry {
                    file,
                    path: path.clone(),
                }));
            }
            Err(err) => {
                let err = LinuxError::from(err);
                if err == LinuxError::EISDIR {
                    return open_dir_entry(path.as_str());
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

fn open_dir_entry(path: &str) -> Result<FdEntry, LinuxError> {
    let mut opts = OpenOptions::new();
    opts.read(true);
    let dir = Directory::open_dir(path, &opts).map_err(LinuxError::from)?;
    let file = File::open(path, &opts).map_err(LinuxError::from)?;
    let attr = file.get_attr().map_err(LinuxError::from)?;
    Ok(FdEntry::Directory(DirectoryEntry {
        dir,
        attr,
        path: path.into(),
    }))
}

fn directory_create_dir(path: &str) -> Result<(), LinuxError> {
    axfs::api::create_dir(path).map_err(LinuxError::from)
}

fn create_fifo_placeholder(path: &str) -> Result<(), LinuxError> {
    let mut opts = OpenOptions::new();
    opts.write(true);
    opts.create_new(true);
    let _file = File::open(path, &opts).map_err(LinuxError::from)?;
    Ok(())
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

fn parse_timens_offsets(src: &[u8]) -> Result<Vec<(u32, TimeOffset)>, LinuxError> {
    let text = core::str::from_utf8(src).map_err(|_| LinuxError::EINVAL)?;
    let mut updates = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let clockid = parts
            .next()
            .ok_or(LinuxError::EINVAL)?
            .parse::<u32>()
            .map_err(|_| LinuxError::EINVAL)?;
        let secs = parts
            .next()
            .ok_or(LinuxError::EINVAL)?
            .parse::<i64>()
            .map_err(|_| LinuxError::EINVAL)?;
        let nanos = parts
            .next()
            .ok_or(LinuxError::EINVAL)?
            .parse::<i32>()
            .map_err(|_| LinuxError::EINVAL)?;
        if parts.next().is_some() || nanos.abs() >= 1_000_000_000 {
            return Err(LinuxError::EINVAL);
        }
        match clockid {
            general::CLOCK_MONOTONIC | general::CLOCK_BOOTTIME => {}
            _ => return Err(LinuxError::EINVAL),
        }
        updates.push((clockid, TimeOffset { secs, nanos }));
    }
    Ok(updates)
}

fn write_timens_offsets(process: &UserProcess, src: &[u8]) -> Result<usize, LinuxError> {
    let updates = parse_timens_offsets(src)?;
    for (clockid, offset) in updates {
        process.write_child_time_offset(clockid, offset)?;
    }
    Ok(src.len())
}

fn parse_shm_next_id(src: &[u8]) -> Result<Option<i32>, LinuxError> {
    let text = core::str::from_utf8(src).map_err(|_| LinuxError::EINVAL)?;
    let value = text.trim().parse::<i32>().map_err(|_| LinuxError::EINVAL)?;
    if value < -1 {
        return Err(LinuxError::EINVAL);
    }
    Ok((value >= 0).then_some(value))
}

fn read_shm_segment_by_index(index: i32) -> Result<Arc<ShmSegment>, LinuxError> {
    if index < 0 {
        return Err(LinuxError::EINVAL);
    }
    sysv_registry()
        .lock()
        .shm
        .by_id
        .values()
        .nth(index as usize)
        .cloned()
        .ok_or(LinuxError::EINVAL)
}

fn proc_shm_text(path: ProcPseudoKind) -> String {
    match path {
        ProcPseudoKind::KernelShmMax => format!("{SYSV_SHMMAX}\n"),
        ProcPseudoKind::KernelShmMin => format!("{SYSV_SHMMIN}\n"),
        ProcPseudoKind::KernelShmMni => format!("{SYSV_SHMMNI}\n"),
        ProcPseudoKind::KernelShmAll => format!("{}\n", SYSV_SHMMAX.div_ceil(PAGE_SIZE_4K)),
        ProcPseudoKind::KernelShmNextId => {
            let registry = sysv_registry().lock();
            match registry.shm.next_hint {
                Some(id) => format!("{id}\n"),
                None => "-1\n".into(),
            }
        }
        ProcPseudoKind::SysvipcShm => {
            let registry = sysv_registry().lock();
            let mut text =
                String::from("       key      shmid perms                  size  cpid  lpid nattch   uid   gid  cuid  cgid      atime      dtime      ctime       rss       swap\n");
            for segment in registry.shm.by_id.values() {
                let meta = segment.meta.lock();
                let _ = writeln!(
                    text,
                    "{:10} {:10} {:5o} {:21} {:5} {:5} {:6} {:5} {:5} {:5} {:5} {:10} {:10} {:10} {:9} {:10}",
                    segment.key,
                    segment.id,
                    meta.perm.mode & MODE_MASK,
                    segment.size,
                    meta.cpid,
                    meta.lpid,
                    meta.nattch,
                    meta.perm.uid,
                    meta.perm.gid,
                    meta.perm.cuid,
                    meta.perm.cgid,
                    meta.atime,
                    meta.dtime,
                    meta.ctime,
                    segment.map_size,
                    0,
                );
            }
            text
        }
        ProcPseudoKind::KernelMsgMni => {
            let registry = sysv_registry().lock();
            format!("{}\n", registry.msg.max_queues)
        }
        ProcPseudoKind::KernelMsgNextId => {
            let registry = sysv_registry().lock();
            match registry.msg.next_hint {
                Some(id) => format!("{id}\n"),
                None => "-1\n".into(),
            }
        }
        ProcPseudoKind::SysvipcMsg => {
            let registry = sysv_registry().lock();
            let mut text = String::from(
                "       key      msqid perms      cbytes       qnum  lspid  lrpid   uid   gid  cuid  cgid      stime      rtime      ctime\n",
            );
            for queue in registry.msg.by_id.values() {
                let state = queue.state.lock();
                let _ = writeln!(
                    text,
                    "{:10} {:10} {:5o} {:11} {:10} {:6} {:6} {:5} {:5} {:5} {:5} {:10} {:10} {:10}",
                    queue.key,
                    queue.id,
                    state.perm.mode & MODE_MASK,
                    state.cbytes,
                    state.messages.len(),
                    state.lspid,
                    state.lrpid,
                    state.perm.uid,
                    state.perm.gid,
                    state.perm.cuid,
                    state.perm.cgid,
                    state.stime,
                    state.rtime,
                    state.ctime,
                );
            }
            text
        }
        ProcPseudoKind::KernelSem => {
            let registry = sysv_registry().lock();
            format!(
                "{} {} {} {}\n",
                registry.sem.max_per_set,
                registry.sem.max_sets * registry.sem.max_per_set,
                registry.sem.max_ops,
                registry.sem.max_sets
            )
        }
        ProcPseudoKind::SysvipcSem => {
            let registry = sysv_registry().lock();
            let mut text = String::from(
                "       key      semid perms      nsems   uid   gid  cuid  cgid      otime      ctime\n",
            );
            for set in registry.sem.by_id.values() {
                let state = set.state.lock();
                let _ = writeln!(
                    text,
                    "{:10} {:10} {:5o} {:10} {:5} {:5} {:5} {:5} {:10} {:10}",
                    set.key,
                    set.id,
                    state.perm.mode & MODE_MASK,
                    state.sems.len(),
                    state.perm.uid,
                    state.perm.gid,
                    state.perm.cuid,
                    state.perm.cgid,
                    state.otime,
                    state.ctime,
                );
            }
            text
        }
    }
}

impl ProcPseudoFile {
    fn read(&mut self, dst: &mut [u8]) -> Result<usize, LinuxError> {
        let content = proc_shm_text(self.kind);
        let src = content.as_bytes();
        if self.offset >= src.len() {
            return Ok(0);
        }
        let len = cmp::min(dst.len(), src.len() - self.offset);
        dst[..len].copy_from_slice(&src[self.offset..self.offset + len]);
        self.offset += len;
        Ok(len)
    }

    fn write(&mut self, src: &[u8]) -> Result<usize, LinuxError> {
        match self.kind {
            ProcPseudoKind::KernelShmNextId => {
                let mut registry = sysv_registry().lock();
                registry.shm.next_hint = parse_shm_next_id(src)?;
                self.offset += src.len();
                Ok(src.len())
            }
            ProcPseudoKind::KernelMsgMni => {
                let mut registry = sysv_registry().lock();
                registry.msg.max_queues = parse_msgmni_limit(src)?;
                self.offset += src.len();
                Ok(src.len())
            }
            ProcPseudoKind::KernelMsgNextId => {
                let mut registry = sysv_registry().lock();
                registry.msg.next_hint = parse_msg_next_id(src)?;
                self.offset += src.len();
                Ok(src.len())
            }
            ProcPseudoKind::KernelSem => {
                let (semmsl, _semmns, semopm, semmni) = parse_sem_limits(src)?;
                let mut registry = sysv_registry().lock();
                registry.sem.max_per_set = semmsl;
                registry.sem.max_ops = semopm;
                registry.sem.max_sets = semmni;
                self.offset += src.len();
                Ok(src.len())
            }
            _ => Err(LinuxError::EBADF),
        }
    }

    fn lseek(&mut self, offset: i64, whence: u32) -> Result<u64, LinuxError> {
        let len = proc_shm_text(self.kind).len() as i64;
        let new_pos = match whence {
            general::SEEK_SET => offset,
            general::SEEK_CUR => self.offset as i64 + offset,
            general::SEEK_END => len + offset,
            _ => return Err(LinuxError::EINVAL),
        };
        if new_pos < 0 {
            return Err(LinuxError::EINVAL);
        }
        self.offset = new_pos as usize;
        Ok(self.offset as u64)
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

fn procfs_pseudo_stat() -> general::stat {
    let mut st: general::stat = unsafe { core::mem::zeroed() };
    st.st_ino = 4;
    st.st_mode = ST_MODE_FILE | 0o600;
    st.st_nlink = 1;
    st.st_blksize = 256;
    st
}
