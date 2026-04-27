use core::cmp;
use core::ffi::{CStr, c_char, c_long};
use core::mem::{offset_of, size_of};
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use axerrno::LinuxError;
use axfs::fops::{self, Directory, File, FileAttr, FileType, OpenOptions};
use axhal::context::{TrapFrame, UspaceContext};
use axhal::paging::MappingFlags;
use axhal::trap::{
    PAGE_FAULT, PageFaultFlags, SYSCALL, register_trap_handler, register_user_return_handler,
};
use axio::PollState;
use axmm::AddrSpace;
use axns::AxNamespace;
use axsync::Mutex;
use axtask::{AxTaskRef, TaskInner, WaitQueue};
use lazyinit::LazyInit;
use linux_raw_sys::{auxvec, general, ioctl, system};
use memory_addr::{PAGE_SIZE_4K, PageIter4K, VirtAddr};
use std::collections::BTreeMap;
use std::string::{String, ToString};
use std::sync::Arc;
use std::vec::Vec;
use xmas_elf::ElfFile;
use xmas_elf::header::{Machine, Type as ElfType};
use xmas_elf::program::{Flags as PhFlags, ProgramHeader, Type as PhType};

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
const AT_FDCWD_I32: i32 = -100;
#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
const SYS_UMOUNT2: u32 = 39;
#[cfg(not(any(target_arch = "riscv64", target_arch = "loongarch64")))]
const SYS_UMOUNT2: u32 = general::__NR_umount2;
#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
const SYS_MOUNT: u32 = 40;
#[cfg(not(any(target_arch = "riscv64", target_arch = "loongarch64")))]
const SYS_MOUNT: u32 = general::__NR_mount;
#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
const SYS_RT_SIGSUSPEND: u32 = 133;
#[cfg(not(any(target_arch = "riscv64", target_arch = "loongarch64")))]
const SYS_RT_SIGSUSPEND: u32 = general::__NR_rt_sigsuspend;
const AUX_CLOCK_TICKS: usize = 100;
const SIGCHLD_NUM: isize = 17;
const SIGCANCEL_NUM: i32 = 33;
#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
const SI_TKILL_CODE: i32 = -6;
#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
const SA_NODEFER_FLAG: u64 = 0x4000_0000;
const KERNEL_SIGSET_BYTES: usize = size_of::<u64>();
const SIG_BLOCK_HOW: usize = 0;
const SIG_UNBLOCK_HOW: usize = 1;
const SIG_SETMASK_HOW: usize = 2;
const RLIMIT_STACK_RESOURCE: u32 = 3;
const RLIMIT_NOFILE_RESOURCE: u32 = 7;
const DEFAULT_NOFILE_LIMIT: u64 = 1024;
const FD_SETSIZE: usize = 1024;
const IOV_MAX: usize = 1024;
const BITS_PER_USIZE: usize = usize::BITS as usize;
const FD_SET_WORDS: usize = FD_SETSIZE.div_ceil(BITS_PER_USIZE);
const IPC_PRIVATE: usize = 0;
const IPC_CREAT: u32 = 0o1000;
const IPC_EXCL: u32 = 0o2000;
const IPC_RMID: usize = 0;
const SHM_RDONLY: u32 = 0o10000;
const SHM_RND: u32 = 0o20000;
const SHM_REMAP: u32 = 0o40000;
#[cfg(target_arch = "riscv64")]
const RISCV_SIGNAL_SIGSET_RESERVED_BYTES: usize = 120;
#[cfg(target_arch = "riscv64")]
const RISCV_SIGNAL_FPSTATE_BYTES: usize = 528;
#[cfg(target_arch = "riscv64")]
const SS_DISABLE: i32 = 2;
#[cfg(target_arch = "riscv64")]
const RISCV_SIGTRAMP_CODE: [u32; 3] = [0x08b0_0893, 0x0000_0073, 0x0010_0073];
#[cfg(target_arch = "loongarch64")]
const LOONGARCH_SIGNAL_UCONTEXT_BYTES: usize = 256;
#[cfg(target_arch = "loongarch64")]
const LOONGARCH_SIGTRAMP_CODE: [u32; 3] = [0x0382_2c0b, 0x002b_0000, 0x002a_0000];

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
    signal_wait: WaitQueue,
    sigsuspend_active: AtomicBool,
    futex_wait: AtomicUsize,
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
    cwd: Mutex<String>,
    exec_root: Mutex<String>,
    mount_table: Mutex<crate::linux_fs::MountTable>,
    shm_attachments: Mutex<BTreeMap<usize, i32>>,
    children: Mutex<Vec<ChildTask>>,
    rlimits: Mutex<BTreeMap<u32, UserRlimit>>,
    signal_actions: Mutex<BTreeMap<usize, general::kernel_sigaction>>,
    itimer_real_deadline_us: AtomicU64,
    itimer_real_interval_us: AtomicU64,
    child_exit_seq: AtomicUsize,
    pid: AtomicI32,
    ppid: i32,
    live_threads: AtomicUsize,
    exit_group_code: AtomicI32,
    exit_code: AtomicI32,
    parent_exit_signal: i32,
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
    entries: Vec<Option<FdSlot>>,
}

enum FdEntry {
    Stdin,
    Stdout,
    Stderr,
    DevNull,
    File(crate::linux_fs::SharedOpenFileDescription),
    Directory(crate::linux_fs::SharedOpenFileDescription),
    Pipe(PipeEndpoint),
}

struct FdSlot {
    fd_flags: crate::linux_fs::FdFlags,
    entry: FdEntry,
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

struct PipeRingBuffer {
    data: [u8; PIPE_BUF_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
    readers: usize,
    writers: usize,
}

struct PipeEndpoint {
    readable: bool,
    buffer: Arc<Mutex<PipeRingBuffer>>,
    read_wait: Arc<WaitQueue>,
    write_wait: Arc<WaitQueue>,
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
    queue: WaitQueue,
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

#[cfg(target_arch = "loongarch64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct LoongArchSignalInfo {
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

#[cfg(target_arch = "loongarch64")]
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct LoongArchSignalFrame {
    saved_mask: u64,
    info: LoongArchSignalInfo,
    ucontext: [u8; LOONGARCH_SIGNAL_UCONTEXT_BYTES],
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
const _: [(); 416] = [(); size_of::<LoongArchSignalFrame>()];

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
    const fn new(readers: usize, writers: usize) -> Self {
        Self {
            data: [0; PIPE_BUF_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::Empty,
            readers,
            writers,
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
        let buffer = Arc::new(Mutex::new(PipeRingBuffer::new(1, 1)));
        let read_wait = Arc::new(WaitQueue::new());
        let write_wait = Arc::new(WaitQueue::new());
        (
            Self {
                readable: true,
                buffer: buffer.clone(),
                read_wait: read_wait.clone(),
                write_wait: write_wait.clone(),
            },
            Self {
                readable: false,
                buffer,
                read_wait,
                write_wait,
            },
        )
    }

    const fn writable(&self) -> bool {
        !self.readable
    }

    fn peer_closed_locked(&self, ring: &PipeRingBuffer) -> bool {
        if self.readable {
            ring.writers == 0
        } else {
            ring.readers == 0
        }
    }

    fn read(&self, dst: &mut [u8]) -> Result<usize, LinuxError> {
        if !self.readable {
            return Err(LinuxError::EBADF);
        }
        let mut read_len = 0usize;
        while read_len < dst.len() {
            let mut ring = self.buffer.lock();
            let available = ring.available_read();
            if available == 0 {
                if read_len > 0 || self.peer_closed_locked(&ring) {
                    return Ok(read_len);
                }
                drop(ring);
                self.read_wait.wait_until(|| {
                    let ring = self.buffer.lock();
                    ring.available_read() > 0 || self.peer_closed_locked(&ring)
                });
                continue;
            }
            let to_read = cmp::min(available, dst.len() - read_len);
            for _ in 0..to_read {
                dst[read_len] = ring.read_byte();
                read_len += 1;
            }
            drop(ring);
            self.write_wait.notify_all(true);
            if read_len > 0 {
                return Ok(read_len);
            }
        }
        Ok(read_len)
    }

    fn write(&self, src: &[u8]) -> Result<usize, LinuxError> {
        if !self.writable() {
            return Err(LinuxError::EBADF);
        }
        let mut written = 0usize;
        while written < src.len() {
            let mut ring = self.buffer.lock();
            if self.peer_closed_locked(&ring) {
                return if written > 0 {
                    Ok(written)
                } else {
                    Err(LinuxError::EPIPE)
                };
            }
            let available = ring.available_write();
            if available == 0 {
                drop(ring);
                self.write_wait.wait_until(|| {
                    let ring = self.buffer.lock();
                    ring.available_write() > 0 || self.peer_closed_locked(&ring)
                });
                continue;
            }
            let to_write = cmp::min(available, src.len() - written);
            for _ in 0..to_write {
                ring.write_byte(src[written]);
                written += 1;
            }
            drop(ring);
            self.read_wait.notify_all(true);
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
        let ring = self.buffer.lock();
        let peer_closed = self.peer_closed_locked(&ring);
        PollState {
            readable: self.readable && (ring.available_read() > 0 || peer_closed),
            writable: self.writable() && (ring.available_write() > 0 || peer_closed),
        }
    }
}

impl Clone for PipeEndpoint {
    fn clone(&self) -> Self {
        {
            let mut ring = self.buffer.lock();
            if self.readable {
                ring.readers += 1;
            } else {
                ring.writers += 1;
            }
        }
        Self {
            readable: self.readable,
            buffer: Arc::clone(&self.buffer),
            read_wait: Arc::clone(&self.read_wait),
            write_wait: Arc::clone(&self.write_wait),
        }
    }
}

impl Drop for PipeEndpoint {
    fn drop(&mut self) {
        {
            let mut ring = self.buffer.lock();
            if self.readable {
                ring.readers = ring.readers.saturating_sub(1);
            } else {
                ring.writers = ring.writers.saturating_sub(1);
            }
        }
        if self.readable {
            self.write_wait.notify_all(true);
        } else {
            self.read_wait.notify_all(true);
        }
    }
}

// compat(Phase 1B iozone): minimal SysV shared memory registry for private
// process-shared benchmark coordination segments.
// delete-when: axmm owns real shared-memory VM objects and SysV IPC registry.
struct CompatShmSegment {
    size: usize,
    pages: usize,
    kernel_vaddr: usize,
    phys_start: usize,
    marked_removed: bool,
    attachments: usize,
}

struct CompatShmRegistry {
    next_id: i32,
    segments: BTreeMap<i32, CompatShmSegment>,
}

impl CompatShmRegistry {
    const fn new() -> Self {
        Self {
            next_id: 1,
            segments: BTreeMap::new(),
        }
    }

    fn allocate_private(&mut self, size: usize) -> Result<i32, LinuxError> {
        if size == 0 {
            return Err(LinuxError::EINVAL);
        }
        let pages = align_up(size, PAGE_SIZE_4K) / PAGE_SIZE_4K;
        let kernel_vaddr = axalloc::global_allocator()
            .alloc_pages(pages, PAGE_SIZE_4K)
            .map_err(|_| LinuxError::ENOMEM)?;
        unsafe {
            core::ptr::write_bytes(kernel_vaddr as *mut u8, 0, pages * PAGE_SIZE_4K);
        }
        let phys_start = axhal::mem::virt_to_phys(VirtAddr::from(kernel_vaddr)).as_usize();
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).unwrap_or(1).max(1);
        self.segments.insert(
            id,
            CompatShmSegment {
                size: pages * PAGE_SIZE_4K,
                pages,
                kernel_vaddr,
                phys_start,
                marked_removed: false,
                attachments: 0,
            },
        );
        Ok(id)
    }
}

fn compat_shm_table() -> &'static Mutex<CompatShmRegistry> {
    static SHM: LazyInit<Mutex<CompatShmRegistry>> = LazyInit::new();
    if !SHM.is_inited() {
        SHM.init_once(Mutex::new(CompatShmRegistry::new()));
    }
    &SHM
}

fn compat_shm_free_segment(segment: CompatShmSegment) {
    axalloc::global_allocator().dealloc_pages(segment.kernel_vaddr, segment.pages);
}

fn compat_shm_prepare_attach(shmid: i32) -> Result<(usize, usize), LinuxError> {
    let mut registry = compat_shm_table().lock();
    let segment = registry
        .segments
        .get_mut(&shmid)
        .ok_or(LinuxError::EINVAL)?;
    if segment.marked_removed {
        return Err(LinuxError::EINVAL);
    }
    segment.attachments = segment
        .attachments
        .checked_add(1)
        .ok_or(LinuxError::EINVAL)?;
    Ok((segment.phys_start, segment.size))
}

fn compat_shm_detach(shmid: i32) {
    let mut registry = compat_shm_table().lock();
    let Some(segment) = registry.segments.get_mut(&shmid) else {
        return;
    };
    segment.attachments = segment.attachments.saturating_sub(1);
    if segment.marked_removed
        && segment.attachments == 0
        && let Some(segment) = registry.segments.remove(&shmid)
    {
        compat_shm_free_segment(segment);
    }
}

fn compat_shm_segment_size(shmid: i32) -> Option<usize> {
    compat_shm_table()
        .lock()
        .segments
        .get(&shmid)
        .map(|segment| segment.size)
}

fn compat_shm_mark_removed(shmid: i32) -> Result<(), LinuxError> {
    let mut registry = compat_shm_table().lock();
    let segment = registry
        .segments
        .get_mut(&shmid)
        .ok_or(LinuxError::EINVAL)?;
    segment.marked_removed = true;
    if segment.attachments == 0
        && let Some(segment) = registry.segments.remove(&shmid)
    {
        compat_shm_free_segment(segment);
    }
    Ok(())
}

fn compat_shm_clone_attachments(attachments: &BTreeMap<usize, i32>) -> Result<(), LinuxError> {
    let mut registry = compat_shm_table().lock();
    for shmid in attachments.values() {
        let segment = registry.segments.get_mut(shmid).ok_or(LinuxError::EINVAL)?;
        segment.attachments = segment
            .attachments
            .checked_add(1)
            .ok_or(LinuxError::EINVAL)?;
    }
    Ok(())
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
        sigsuspend_active: AtomicBool::new(false),
        futex_wait: AtomicUsize::new(0),
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
    let image = load_program_image(&mut aspace, cwd, argv, &[])?;

    let process = Arc::new(UserProcess {
        aspace: Mutex::new(aspace),
        brk: Mutex::new(image.brk),
        fds: Mutex::new(FdTable::new()),
        cwd: Mutex::new(cwd.into()),
        exec_root: Mutex::new(image.exec_root.clone()),
        mount_table: Mutex::new(crate::linux_fs::MountTable::new()),
        shm_attachments: Mutex::new(BTreeMap::new()),
        children: Mutex::new(Vec::new()),
        rlimits: Mutex::new(BTreeMap::new()),
        signal_actions: Mutex::new(BTreeMap::new()),
        itimer_real_deadline_us: AtomicU64::new(0),
        itimer_real_interval_us: AtomicU64::new(0),
        child_exit_seq: AtomicUsize::new(0),
        pid: AtomicI32::new(0),
        ppid: 1,
        live_threads: AtomicUsize::new(1),
        exit_group_code: AtomicI32::new(NO_EXIT_GROUP_CODE),
        exit_code: AtomicI32::new(0),
        parent_exit_signal: 0,
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
    envp: &[&str],
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
        envp,
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
    envp: &[&str],
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

    let mut env_ptrs = Vec::with_capacity(envp.len());
    for env in envp.iter().rev() {
        let mut bytes = env.as_bytes().to_vec();
        bytes.push(0);
        let ptr = push_stack_bytes(aspace, stack_base, &mut sp, &bytes, 1)?;
        env_ptrs.push(ptr);
    }
    env_ptrs.reverse();

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

    let mut words = Vec::with_capacity(1 + arg_ptrs.len() + 1 + env_ptrs.len() + 1 + aux.len() * 2);
    words.push(argv.len());
    words.extend(arg_ptrs.iter().copied());
    words.push(0);
    words.extend(env_ptrs.iter().copied());
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
    envp: &[String],
) -> Result<(usize, usize, usize), String> {
    let argv_refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
    let envp_refs = envp.iter().map(String::as_str).collect::<Vec<_>>();
    let image = {
        let mut aspace = process.aspace.lock();
        load_program_image(&mut aspace, cwd, &argv_refs, &envp_refs)?
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

    fn visible_cwd(&self) -> String {
        let cwd = self.cwd();
        if let Some(rest) = cwd.strip_prefix(TESTSUITE_STAGE_ROOT) {
            if rest.is_empty() {
                "/".into()
            } else if rest.starts_with('/') {
                rest.into()
            } else {
                cwd
            }
        } else {
            cwd
        }
    }

    fn normalize_user_path(&self, path: &str) -> Result<String, LinuxError> {
        let cwd = self.cwd();
        crate::linux_fs::resolve_cwd_path(cwd.as_str(), path).ok_or(LinuxError::EINVAL)
    }

    fn teardown(&self) {
        compat_itimer_real_disarm(self);
        self.detach_all_compat_shm();
        self.aspace.lock().clear();
        self.fds.lock().close_all();
        self.mount_table.lock().clear();
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
            compat_itimer_real_disarm(self);
            self.detach_all_compat_shm();
            self.fds.lock().close_all();
            self.exit_wait.notify_all(false);
            self.notify_parent_exit_signal();
        }
    }

    fn notify_parent_exit_signal(&self) {
        if self.parent_exit_signal == 0 {
            return;
        }
        if let Some(parent) = user_thread_entry_by_tid(self.ppid) {
            if self.parent_exit_signal == SIGCHLD_NUM as i32 {
                parent
                    .process
                    .child_exit_seq
                    .fetch_add(1, Ordering::Release);
                if let Some(ext) = task_ext(&parent.task) {
                    if ext.sigsuspend_active.load(Ordering::Acquire)
                        && !signal_is_blocked(ext, SIGCHLD_NUM as i32)
                    {
                        ext.pending_signal
                            .store(SIGCHLD_NUM as i32, Ordering::Release);
                    }
                    ext.signal_wait.notify_all(true);
                }
            } else {
                let _ = deliver_user_signal(&parent, self.parent_exit_signal);
            }
        }
    }

    fn detach_all_compat_shm(&self) {
        let attachments = core::mem::take(&mut *self.shm_attachments.lock());
        for (addr, shmid) in attachments {
            let size = compat_shm_segment_size(shmid).unwrap_or(0);
            if size != 0 {
                let _ = self.aspace.lock().unmap(VirtAddr::from(addr), size);
            }
            compat_shm_detach(shmid);
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

    fn fork(&self, parent_exit_signal: i32) -> Result<Arc<UserProcess>, LinuxError> {
        let mut aspace = axmm::new_user_aspace(VirtAddr::from(USER_ASPACE_BASE), USER_ASPACE_SIZE)
            .map_err(LinuxError::from)?;
        {
            let parent_aspace = self.aspace.lock();
            aspace
                .clone_user_mappings_from(&parent_aspace)
                .map_err(LinuxError::from)?;
        }
        let shm_attachments = self.shm_attachments.lock().clone();
        compat_shm_clone_attachments(&shm_attachments)?;

        Ok(Arc::new(UserProcess {
            aspace: Mutex::new(aspace),
            brk: Mutex::new(*self.brk.lock()),
            fds: Mutex::new(self.fds.lock().fork_copy()?),
            cwd: Mutex::new(self.cwd()),
            exec_root: Mutex::new(self.exec_root()),
            mount_table: Mutex::new(self.mount_table.lock().clone()),
            shm_attachments: Mutex::new(shm_attachments),
            children: Mutex::new(Vec::new()),
            rlimits: Mutex::new(self.rlimits.lock().clone()),
            signal_actions: Mutex::new(self.signal_actions.lock().clone()),
            itimer_real_deadline_us: AtomicU64::new(0),
            itimer_real_interval_us: AtomicU64::new(0),
            child_exit_seq: AtomicUsize::new(0),
            pid: AtomicI32::new(0),
            ppid: axtask::current().id().as_u64() as i32,
            live_threads: AtomicUsize::new(1),
            exit_group_code: AtomicI32::new(NO_EXIT_GROUP_CODE),
            exit_code: AtomicI32::new(0),
            parent_exit_signal,
            exit_wait: WaitQueue::new(),
        }))
    }

    fn add_child(&self, task: AxTaskRef, process: Arc<UserProcess>) -> i32 {
        let pid = task.id().as_u64() as i32;
        self.children.lock().push(ChildTask { pid, task, process });
        pid
    }

    fn has_exited_child(&self) -> bool {
        self.children
            .lock()
            .iter()
            .any(|child| child.process.live_threads.load(Ordering::Acquire) == 0)
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

fn futex_table() -> &'static Mutex<BTreeMap<usize, Arc<FutexState>>> {
    static FUTEXES: LazyInit<Mutex<BTreeMap<usize, Arc<FutexState>>>> = LazyInit::new();
    if !FUTEXES.is_inited() {
        FUTEXES.init_once(Mutex::new(BTreeMap::new()));
    }
    &FUTEXES
}

fn user_thread_table() -> &'static Mutex<BTreeMap<i32, UserThreadEntry>> {
    static USER_THREADS: LazyInit<Mutex<BTreeMap<i32, UserThreadEntry>>> = LazyInit::new();
    if !USER_THREADS.is_inited() {
        USER_THREADS.init_once(Mutex::new(BTreeMap::new()));
    }
    &USER_THREADS
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
    ext.signal_wait.notify_all(true);
    if sig == SIGCANCEL_NUM {
        user_trace!(
            "sigdbg: deliver tid={} blocked={} futex_wait={:#x}",
            entry.task.id().as_u64(),
            signal_is_blocked(ext, sig),
            ext.futex_wait.load(Ordering::Acquire),
        );
    }
    if sig == SIGCANCEL_NUM && !signal_is_blocked(ext, sig) {
        let futex_wait = ext.futex_wait.load(Ordering::Acquire);
        if futex_wait != 0 {
            if let Some(state) = futex_table().lock().get(&futex_wait).cloned() {
                state.seq.fetch_add(1, Ordering::Release);
                let _ = state.queue.notify_task(true, &entry.task);
            }
        }
    }
    Ok(())
}

fn futex_state(uaddr: usize) -> Arc<FutexState> {
    let mut table = futex_table().lock();
    table
        .entry(uaddr)
        .or_insert_with(|| {
            Arc::new(FutexState {
                seq: AtomicU32::new(0),
                queue: WaitQueue::new(),
            })
        })
        .clone()
}

fn futex_wake_addr(uaddr: usize, count: usize) -> usize {
    let Some(state) = futex_table().lock().get(&uaddr).cloned() else {
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
    let _ = futex_wake_addr(clear_tid, 1);
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

fn has_unblocked_pending_signal(ext: &UserTaskExt) -> bool {
    let sig = ext.pending_signal.load(Ordering::Acquire);
    sig != 0 && !signal_is_blocked(ext, sig)
}

fn current_sigcancel_pending() -> bool {
    current_task_ext().is_some_and(|ext| {
        ext.pending_signal.load(Ordering::Acquire) == SIGCANCEL_NUM
            && !signal_is_blocked(ext, SIGCANCEL_NUM)
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
    compat_itimer_real_poll(ext);
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

#[cfg(target_arch = "loongarch64")]
fn make_loongarch_siginfo(sig: i32, code: i32, tid: i32) -> LoongArchSignalInfo {
    let mut info = LoongArchSignalInfo { bytes: [0; 128] };
    info.bytes[0..4].copy_from_slice(&sig.to_ne_bytes());
    info.bytes[4..8].copy_from_slice(&0i32.to_ne_bytes());
    info.bytes[8..12].copy_from_slice(&code.to_ne_bytes());
    info.bytes[16..20].copy_from_slice(&tid.to_ne_bytes());
    info.bytes[20..24].copy_from_slice(&0u32.to_ne_bytes());
    info
}

#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
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

#[cfg(target_arch = "loongarch64")]
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
    if handler <= 1 {
        ext.pending_signal.store(0, Ordering::Release);
        return Ok(());
    }

    let current_mask = ext.signal_mask.load(Ordering::Acquire);
    let frame_size = size_of::<LoongArchSignalFrame>();
    let frame_addr = align_down(tf.regs.sp.saturating_sub(frame_size), 16);
    ensure_signal_frame_pages(ext.process.as_ref(), frame_addr, frame_size)?;

    let frame = LoongArchSignalFrame {
        saved_mask: current_mask,
        info: make_loongarch_siginfo(sig, SI_TKILL_CODE, current_tid()),
        ucontext: [0; LOONGARCH_SIGNAL_UCONTEXT_BYTES],
        trampoline: LOONGARCH_SIGTRAMP_CODE,
    };
    if write_user_value(ext.process.as_ref(), frame_addr, &frame) != 0 {
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

    tf.regs.sp = frame_addr;
    tf.regs.ra = frame_addr + offset_of!(LoongArchSignalFrame, trampoline);
    tf.regs.a0 = sig as usize;
    tf.regs.a1 = frame_addr + offset_of!(LoongArchSignalFrame, info);
    tf.regs.a2 = frame_addr + offset_of!(LoongArchSignalFrame, ucontext);
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
        general::__NR_write => sys_write(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_pwrite64 => {
            sys_pwrite64(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_writev => sys_writev(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_readv => sys_readv(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_preadv => sys_preadv(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3()),
        general::__NR_pwritev => sys_pwritev(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3()),
        general::__NR_getcwd => sys_getcwd(&process, tf.arg0(), tf.arg1()),
        general::__NR_chdir => sys_chdir(&process, tf.arg0()),
        general::__NR_openat => sys_openat(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3()),
        general::__NR_mkdirat => sys_mkdirat(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_unlinkat => sys_unlinkat(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        SYS_MOUNT => sys_mount(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
        SYS_UMOUNT2 => sys_umount2(&process, tf.arg0(), tf.arg1()),
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
        general::__NR_statx => sys_statx(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
        general::__NR_getdents64 => sys_getdents64(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_lseek => sys_lseek(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_fsync => sys_fsync(&process, tf.arg0()),
        general::__NR_fdatasync => sys_fdatasync(&process, tf.arg0()),
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
        general::__NR_shmget => sys_shmget(tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_shmctl => sys_shmctl(tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_shmat => sys_shmat(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_shmdt => sys_shmdt(&process, tf.arg0()),
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
        general::__NR_getgid => 0,
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
        SYS_RT_SIGSUSPEND => sys_rt_sigsuspend(&process, tf.arg0(), tf.arg1()),
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
        process.fds.lock().read(fd as i32, dst)
    })
}

fn sys_pread64(process: &UserProcess, fd: usize, buf: usize, count: usize, offset: usize) -> isize {
    let offset = match explicit_file_offset(offset) {
        Ok(offset) => offset,
        Err(err) => return neg_errno(err),
    };
    with_writable_slice(process, buf, count, |dst| {
        let table = process.fds.lock();
        let FdEntry::File(desc) = table.entry(fd as i32)? else {
            return Err(LinuxError::EBADF);
        };
        let desc = Arc::clone(desc);
        drop(table);

        let mut filled = 0usize;
        let mut current_offset = offset;
        while filled < dst.len() {
            let read = desc.pread_file(&mut dst[filled..], current_offset)?;
            if read == 0 {
                break;
            }
            current_offset = crate::linux_fs::advance_explicit_offset(current_offset, read)?;
            filled += read;
        }
        Ok(filled)
    })
}

fn sys_write(process: &UserProcess, fd: usize, buf: usize, count: usize) -> isize {
    with_readable_slice(process, buf, count, |src| {
        process.fds.lock().write(fd as i32, src)
    })
}

fn sys_pwrite64(
    process: &UserProcess,
    fd: usize,
    buf: usize,
    count: usize,
    offset: usize,
) -> isize {
    let offset = match explicit_file_offset(offset) {
        Ok(offset) => offset,
        Err(err) => return neg_errno(err),
    };
    with_readable_slice(process, buf, count, |src| {
        let table = process.fds.lock();
        let FdEntry::File(desc) = table.entry(fd as i32)? else {
            return Err(LinuxError::EBADF);
        };
        let desc = Arc::clone(desc);
        drop(table);

        let mut written = 0usize;
        let mut current_offset = offset;
        while written < src.len() {
            let n = desc.pwrite_file(&src[written..], current_offset)?;
            if n == 0 {
                break;
            }
            current_offset = crate::linux_fs::advance_explicit_offset(current_offset, n)?;
            written += n;
        }
        Ok(written)
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
    if flags != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let (read_end, write_end) = PipeEndpoint::new_pair();
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

fn explicit_file_offset(offset: usize) -> Result<u64, LinuxError> {
    if offset > i64::MAX as usize {
        return Err(LinuxError::EINVAL);
    }
    Ok(offset as u64)
}

fn checked_io_total(total: usize, delta: usize) -> Result<usize, LinuxError> {
    total
        .checked_add(delta)
        .filter(|value| *value <= isize::MAX as usize)
        .ok_or(LinuxError::EINVAL)
}

fn read_iovec_entries(
    process: &UserProcess,
    iov: usize,
    iovcnt: usize,
) -> Result<Vec<general::iovec>, LinuxError> {
    if iovcnt > IOV_MAX {
        return Err(LinuxError::EINVAL);
    }
    let bytes_len = iovcnt
        .checked_mul(size_of::<general::iovec>())
        .ok_or(LinuxError::EINVAL)?;
    let Some(iov_bytes) = user_bytes(process, iov, bytes_len, false) else {
        return Err(LinuxError::EFAULT);
    };
    let mut entries = Vec::with_capacity(iovcnt);
    for chunk in iov_bytes.chunks_exact(size_of::<general::iovec>()) {
        entries.push(unsafe { ptr::read_unaligned(chunk.as_ptr() as *const general::iovec) });
    }
    Ok(entries)
}

fn sys_writev(process: &UserProcess, fd: usize, iov: usize, iovcnt: usize) -> isize {
    if let Err(err) = process.fds.lock().entry(fd as i32).map(|_| ()) {
        return neg_errno(err);
    };

    let entries = match read_iovec_entries(process, iov, iovcnt) {
        Ok(entries) => entries,
        Err(err) => return neg_errno(err),
    };
    let mut written = 0usize;
    for entry in entries {
        let len = entry.iov_len as usize;
        if len == 0 {
            continue;
        }
        let Some(src) = user_bytes(process, entry.iov_base as usize, len, false) else {
            return neg_errno(LinuxError::EFAULT);
        };
        let n = match process.fds.lock().write(fd as i32, src) {
            Ok(v) => v,
            Err(err) => return neg_errno(err),
        };
        written = match checked_io_total(written, n) {
            Ok(total) => total,
            Err(err) => return neg_errno(err),
        };
        if n < len {
            break;
        }
    }
    written as isize
}

fn sys_readv(process: &UserProcess, fd: usize, iov: usize, iovcnt: usize) -> isize {
    if let Err(err) = process.fds.lock().entry(fd as i32).map(|_| ()) {
        return neg_errno(err);
    };

    let entries = match read_iovec_entries(process, iov, iovcnt) {
        Ok(entries) => entries,
        Err(err) => return neg_errno(err),
    };
    let mut total = 0usize;
    for entry in entries {
        let len = entry.iov_len as usize;
        if len == 0 {
            continue;
        }
        let Some(dst) = user_bytes_mut(process, entry.iov_base as usize, len, true) else {
            return neg_errno(LinuxError::EFAULT);
        };
        let n = match process.fds.lock().read(fd as i32, dst) {
            Ok(v) => v,
            Err(err) => return neg_errno(err),
        };
        total = match checked_io_total(total, n) {
            Ok(total) => total,
            Err(err) => return neg_errno(err),
        };
        if n < len {
            break;
        }
    }
    total as isize
}

fn sys_preadv(process: &UserProcess, fd: usize, iov: usize, iovcnt: usize, offset: usize) -> isize {
    let mut current_offset = match explicit_file_offset(offset) {
        Ok(offset) => offset,
        Err(err) => return neg_errno(err),
    };
    let desc = {
        let table = process.fds.lock();
        match table.entry(fd as i32) {
            Ok(FdEntry::File(desc)) => Arc::clone(desc),
            Ok(_) => return neg_errno(LinuxError::EBADF),
            Err(err) => return neg_errno(err),
        }
    };
    let entries = match read_iovec_entries(process, iov, iovcnt) {
        Ok(entries) => entries,
        Err(err) => return neg_errno(err),
    };

    let mut total = 0usize;
    for entry in entries {
        let len = entry.iov_len as usize;
        if len == 0 {
            continue;
        }
        let Some(dst) = user_bytes_mut(process, entry.iov_base as usize, len, true) else {
            return neg_errno(LinuxError::EFAULT);
        };
        let n = match desc.pread_file(dst, current_offset) {
            Ok(v) => v,
            Err(err) => return neg_errno(err),
        };
        total = match checked_io_total(total, n) {
            Ok(total) => total,
            Err(err) => return neg_errno(err),
        };
        current_offset = match crate::linux_fs::advance_explicit_offset(current_offset, n) {
            Ok(offset) => offset,
            Err(err) => return neg_errno(err),
        };
        if n < len {
            break;
        }
    }
    total as isize
}

fn sys_pwritev(
    process: &UserProcess,
    fd: usize,
    iov: usize,
    iovcnt: usize,
    offset: usize,
) -> isize {
    let mut current_offset = match explicit_file_offset(offset) {
        Ok(offset) => offset,
        Err(err) => return neg_errno(err),
    };
    let desc = {
        let table = process.fds.lock();
        match table.entry(fd as i32) {
            Ok(FdEntry::File(desc)) => Arc::clone(desc),
            Ok(_) => return neg_errno(LinuxError::EBADF),
            Err(err) => return neg_errno(err),
        }
    };
    let entries = match read_iovec_entries(process, iov, iovcnt) {
        Ok(entries) => entries,
        Err(err) => return neg_errno(err),
    };

    let mut total = 0usize;
    for entry in entries {
        let len = entry.iov_len as usize;
        if len == 0 {
            continue;
        }
        let Some(src) = user_bytes(process, entry.iov_base as usize, len, false) else {
            return neg_errno(LinuxError::EFAULT);
        };
        let n = match desc.pwrite_file(src, current_offset) {
            Ok(v) => v,
            Err(err) => return neg_errno(err),
        };
        total = match checked_io_total(total, n) {
            Ok(total) => total,
            Err(err) => return neg_errno(err),
        };
        current_offset = match crate::linux_fs::advance_explicit_offset(current_offset, n) {
            Ok(offset) => offset,
            Err(err) => return neg_errno(err),
        };
        if n < len {
            break;
        }
    }
    total as isize
}

fn sys_getcwd(process: &UserProcess, buf: usize, size: usize) -> isize {
    let cwd = process.cwd();
    let mut bytes = cwd.into_bytes();
    bytes.push(0);
    if bytes.len() > size {
        let visible_cwd = process.visible_cwd();
        bytes = visible_cwd.into_bytes();
        bytes.push(0);
        if bytes.len() > size {
            return neg_errno(LinuxError::ERANGE);
        }
    }
    let Some(dst) = user_bytes_mut(process, buf, bytes.len(), true) else {
        return neg_errno(LinuxError::EFAULT);
    };
    dst.copy_from_slice(&bytes);
    buf as isize
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
    envp: usize,
) -> isize {
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    let argv = match read_execve_argv(process, argv, path.as_str()) {
        Ok(argv) => argv,
        Err(err) => return neg_errno(err),
    };
    let envp = match read_execve_envp(process, envp) {
        Ok(envp) => envp,
        Err(err) => return neg_errno(err),
    };
    let cwd = process.cwd();
    let (entry, stack_ptr, argc) = match exec_program(process, cwd.as_str(), &argv, &envp) {
        Ok(image) => image,
        Err(_) => return neg_errno(LinuxError::ENOEXEC),
    };
    process.fds.lock().close_cloexec();
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

        let child_process = match process.fork(exit_signal as i32) {
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
            signal_wait: WaitQueue::new(),
            sigsuspend_active: AtomicBool::new(false),
            futex_wait: AtomicUsize::new(0),
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
        signal_wait: WaitQueue::new(),
        sigsuspend_active: AtomicBool::new(false),
        futex_wait: AtomicUsize::new(0),
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
    let abs_path = {
        let table = process.fds.lock();
        match resolve_dirfd_path(process, &table, dirfd as i32, path.as_str()) {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        }
    };
    match directory_create_dir(abs_path.as_str()) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_unlinkat(process: &UserProcess, dirfd: usize, pathname: usize, flags: usize) -> isize {
    let flags = flags as u32;
    if flags & !general::AT_REMOVEDIR != 0 {
        return neg_errno(LinuxError::EINVAL);
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
    let result = if flags & general::AT_REMOVEDIR != 0 {
        directory_remove_dir(abs_path.as_str())
    } else {
        directory_remove_file(abs_path.as_str())
    };
    match result {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_mount(
    process: &UserProcess,
    source: usize,
    target: usize,
    fstype: usize,
    flags: usize,
    data: usize,
) -> isize {
    let source = match read_cstr(process, source) {
        Ok(source) => source,
        Err(err) => return neg_errno(err),
    };
    let target = match read_cstr(process, target) {
        Ok(target) => target,
        Err(err) => return neg_errno(err),
    };
    let fstype = match read_cstr(process, fstype) {
        Ok(fstype) => fstype,
        Err(err) => return neg_errno(err),
    };
    let target = match process.normalize_user_path(target.as_str()) {
        Ok(target) => target,
        Err(err) => return neg_errno(err),
    };
    if let Err(err) = open_dir_entry(target.as_str()) {
        return neg_errno(err);
    }
    let request = crate::linux_fs::MountRequest {
        source: source.as_str(),
        target: target.as_str(),
        fstype: fstype.as_str(),
        flags,
        data,
    };
    match process.mount_table.lock().mount(request) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_umount2(process: &UserProcess, target: usize, flags: usize) -> isize {
    let target = match read_cstr(process, target) {
        Ok(target) => target,
        Err(err) => return neg_errno(err),
    };
    let target = match process.normalize_user_path(target.as_str()) {
        Ok(target) => target,
        Err(err) => return neg_errno(err),
    };
    let request = crate::linux_fs::UmountRequest {
        target: target.as_str(),
        flags,
    };
    match process.mount_table.lock().umount(request) {
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
    let abs_path = {
        let table = process.fds.lock();
        match resolve_dirfd_path(process, &table, dirfd as i32, path.as_str()) {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        }
    };
    match stat_path_abs(abs_path.as_str()) {
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
    match rename_path_abs(old_abs_path.as_str(), new_abs_path.as_str()) {
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
    let abs_path = {
        let table = process.fds.lock();
        match resolve_dirfd_path(process, &table, dirfd as i32, path.as_str()) {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        }
    };
    let st = match stat_path_abs(abs_path.as_str()) {
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
    mask: usize,
    statxbuf: usize,
) -> isize {
    let flags = flags as u32;
    if let Err(err) = crate::linux_fs::validate_statx_flags(flags) {
        return neg_errno(err);
    }
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    let st = if path.is_empty() {
        if !crate::linux_fs::statx_accepts_empty_path(flags) {
            return neg_errno(LinuxError::ENOENT);
        }
        match process.fds.lock().stat(dirfd as i32) {
            Ok(st) => st,
            Err(err) => return neg_errno(err),
        }
    } else {
        let abs_path = {
            let table = process.fds.lock();
            match resolve_dirfd_path(process, &table, dirfd as i32, path.as_str()) {
                Ok(path) => path,
                Err(err) => return neg_errno(err),
            }
        };
        match stat_path_abs(abs_path.as_str()) {
            Ok(st) => st,
            Err(err) => return neg_errno(err),
        }
    };
    let stx = crate::linux_fs::stat_to_statx(&st, mask as u32);
    write_user_value(process, statxbuf, &stx)
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
    let now = match clock_now_duration(clk_id as u32) {
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

fn timeval_to_duration(tv: general::timeval) -> Result<core::time::Duration, LinuxError> {
    if tv.tv_sec < 0 || tv.tv_usec < 0 || tv.tv_usec >= 1_000_000 {
        return Err(LinuxError::EINVAL);
    }
    Ok(core::time::Duration::new(
        tv.tv_sec as u64,
        (tv.tv_usec as u32) * 1_000,
    ))
}

fn duration_to_timeval(duration: core::time::Duration) -> general::timeval {
    general::timeval {
        tv_sec: duration.as_secs() as _,
        tv_usec: duration.subsec_micros() as _,
    }
}

fn duration_to_micros(duration: core::time::Duration) -> u64 {
    duration.as_micros().min(u64::MAX as u128) as u64
}

fn compat_itimer_real_remaining(process: &UserProcess) -> general::itimerval {
    let deadline_us = process.itimer_real_deadline_us.load(Ordering::Acquire);
    let interval_us = process.itimer_real_interval_us.load(Ordering::Acquire);
    let remaining = deadline_us
        .checked_sub(duration_to_micros(axhal::time::wall_time()))
        .map(core::time::Duration::from_micros)
        .unwrap_or_default();
    general::itimerval {
        it_interval: duration_to_timeval(core::time::Duration::from_micros(interval_us)),
        it_value: duration_to_timeval(remaining),
    }
}

fn compat_itimer_real_disarm(process: &UserProcess) {
    process.itimer_real_deadline_us.store(0, Ordering::Release);
    process.itimer_real_interval_us.store(0, Ordering::Release);
}

fn compat_itimer_real_arm(
    process: &UserProcess,
    initial: core::time::Duration,
    interval: core::time::Duration,
) {
    // compat(unixbench-fstime): provide the narrow ITIMER_REAL/SIGALRM path
    // used by alarm(2)-driven benchmark loops.
    // delete-when: timer/signal subsystem owns POSIX interval timers.
    process.itimer_real_deadline_us.store(
        duration_to_micros(axhal::time::wall_time()).saturating_add(duration_to_micros(initial)),
        Ordering::Release,
    );
    process
        .itimer_real_interval_us
        .store(duration_to_micros(interval), Ordering::Release);
}

fn compat_itimer_real_poll(ext: &UserTaskExt) {
    let process = ext.process.as_ref();
    let deadline_us = process.itimer_real_deadline_us.load(Ordering::Acquire);
    if deadline_us == 0 {
        return;
    }
    let now_us = duration_to_micros(axhal::time::wall_time());
    if now_us < deadline_us {
        return;
    }

    let interval_us = process.itimer_real_interval_us.load(Ordering::Acquire);
    if interval_us == 0 {
        process.itimer_real_deadline_us.store(0, Ordering::Release);
        process.itimer_real_interval_us.store(0, Ordering::Release);
    } else {
        process
            .itimer_real_deadline_us
            .store(now_us.saturating_add(interval_us), Ordering::Release);
    }
    ext.pending_signal
        .store(general::SIGALRM as i32, Ordering::Release);
}

fn sys_setitimer(process: &UserProcess, which: i32, new_value: usize, old_value: usize) -> isize {
    if which != general::ITIMER_REAL as i32 {
        return neg_errno(LinuxError::EINVAL);
    }
    if old_value != 0 {
        let value = compat_itimer_real_remaining(process);
        let ret = write_user_value(process, old_value, &value);
        if ret != 0 {
            return ret;
        }
    }
    let value = match read_user_value::<general::itimerval>(process, new_value) {
        Ok(value) => value,
        Err(err) => return neg_errno(err),
    };
    let initial = match timeval_to_duration(value.it_value) {
        Ok(duration) => duration,
        Err(err) => return neg_errno(err),
    };
    let interval = match timeval_to_duration(value.it_interval) {
        Ok(duration) => duration,
        Err(err) => return neg_errno(err),
    };
    if initial.is_zero() {
        compat_itimer_real_disarm(process);
    } else {
        compat_itimer_real_arm(process, initial, interval);
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
        let now = match clock_now_duration(clockid as u32) {
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

fn sys_shmget(key: usize, size: usize, flags: usize) -> isize {
    let flags = flags as u32;
    let allowed_flags = IPC_CREAT | IPC_EXCL | 0o777;
    if flags & !allowed_flags != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    if key != IPC_PRIVATE {
        return neg_errno(LinuxError::EOPNOTSUPP);
    }
    match compat_shm_table().lock().allocate_private(size) {
        Ok(id) => id as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_shmat(process: &UserProcess, shmid: usize, shmaddr: usize, shmflg: usize) -> isize {
    let shmid = shmid as i32;
    let shmflg = shmflg as u32;
    if shmflg & !(SHM_RDONLY | SHM_RND | SHM_REMAP) != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    if shmaddr != 0 || shmflg != 0 {
        return neg_errno(LinuxError::EOPNOTSUPP);
    }
    let (phys_start, size) = match compat_shm_prepare_attach(shmid) {
        Ok(segment) => segment,
        Err(err) => return neg_errno(err),
    };
    let target = {
        let mut brk = process.brk.lock();
        let start = align_up(brk.next_mmap, PAGE_SIZE_4K);
        brk.next_mmap = start + size + PAGE_SIZE_4K;
        if start < USER_MMAP_BASE || start + size >= USER_STACK_TOP - USER_STACK_SIZE {
            compat_shm_detach(shmid);
            return neg_errno(LinuxError::ENOMEM);
        }
        start
    };
    let map_flags = MappingFlags::USER | MappingFlags::READ | MappingFlags::WRITE;
    if let Err(err) =
        process
            .aspace
            .lock()
            .map_linear(VirtAddr::from(target), phys_start.into(), size, map_flags)
    {
        compat_shm_detach(shmid);
        return neg_errno(LinuxError::from(err));
    }
    process.shm_attachments.lock().insert(target, shmid);
    target as isize
}

fn sys_shmdt(process: &UserProcess, shmaddr: usize) -> isize {
    let shmid = {
        let mut attachments = process.shm_attachments.lock();
        let Some(shmid) = attachments.remove(&shmaddr) else {
            return neg_errno(LinuxError::EINVAL);
        };
        shmid
    };
    let Some(size) = compat_shm_segment_size(shmid) else {
        return neg_errno(LinuxError::EINVAL);
    };
    if let Err(err) = process.aspace.lock().unmap(VirtAddr::from(shmaddr), size) {
        process.shm_attachments.lock().insert(shmaddr, shmid);
        return neg_errno(LinuxError::from(err));
    }
    compat_shm_detach(shmid);
    0
}

fn sys_shmctl(shmid: usize, cmd: usize, _buf: usize) -> isize {
    if cmd != IPC_RMID {
        return neg_errno(LinuxError::EINVAL);
    }
    match compat_shm_mark_removed(shmid as i32) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
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
        general::FUTEX_WAIT => {
            let current = match read_user_value::<u32>(process, uaddr) {
                Ok(value) => value,
                Err(err) => return neg_errno(err),
            };
            if current != val as u32 {
                return neg_errno(LinuxError::EAGAIN);
            }
            let state = futex_state(uaddr);
            let seq = state.seq.load(Ordering::Acquire);
            if let Some(ext) = current_task_ext() {
                ext.futex_wait.store(uaddr, Ordering::Release);
            }
            let wait_cond = || {
                state.seq.load(Ordering::Acquire) != seq
                    || read_user_value::<u32>(process, uaddr)
                        .map_or(true, |value| value != val as u32)
                    || current_sigcancel_pending()
            };
            if timeout != 0 {
                let ts = match read_user_value::<general::timespec>(process, timeout) {
                    Ok(value) => value,
                    Err(err) => return neg_errno(err),
                };
                let dur = core::time::Duration::new(
                    ts.tv_sec.max(0) as u64,
                    ts.tv_nsec.clamp(0, 999_999_999) as u32,
                );
                if state.queue.wait_timeout_until(dur, wait_cond) {
                    if let Some(ext) = current_task_ext() {
                        ext.futex_wait.store(0, Ordering::Release);
                    }
                    return neg_errno(LinuxError::ETIMEDOUT);
                }
                if let Some(ext) = current_task_ext() {
                    ext.futex_wait.store(0, Ordering::Release);
                }
                if current_sigcancel_pending() {
                    return neg_errno(LinuxError::EINTR);
                }
                return 0;
            }
            state.queue.wait_until(wait_cond);
            if let Some(ext) = current_task_ext() {
                ext.futex_wait.store(0, Ordering::Release);
            }
            if current_sigcancel_pending() {
                return neg_errno(LinuxError::EINTR);
            }
            0
        }
        general::FUTEX_WAKE => futex_wake_addr(uaddr, val) as isize,
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
    #[cfg(target_arch = "loongarch64")]
    {
        let Some(ext) = current_task_ext() else {
            return neg_errno(LinuxError::EINVAL);
        };
        let frame_addr = ext.signal_frame.load(Ordering::Acquire);
        if frame_addr == 0 {
            return neg_errno(LinuxError::EINVAL);
        }
        let frame = match read_user_value::<LoongArchSignalFrame>(process, frame_addr) {
            Ok(frame) => frame,
            Err(err) => return neg_errno(err),
        };
        let Some(restored) = ext.pending_sigreturn.lock().take() else {
            return neg_errno(LinuxError::EINVAL);
        };
        ext.signal_mask.store(frame.saved_mask, Ordering::Release);
        ext.signal_frame.store(0, Ordering::Release);
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

fn sys_rt_sigsuspend(process: &UserProcess, mask: usize, sigsetsize: usize) -> isize {
    let Some(ext) = current_task_ext() else {
        return neg_errno(LinuxError::EINVAL);
    };
    if mask == 0 {
        return neg_errno(LinuxError::EFAULT);
    }
    if sigsetsize != 0 && sigsetsize < KERNEL_SIGSET_BYTES {
        return neg_errno(LinuxError::EINVAL);
    }
    let Some(src) = user_bytes(process, mask, KERNEL_SIGSET_BYTES, false) else {
        return neg_errno(LinuxError::EFAULT);
    };
    let mut set_bytes = [0u8; KERNEL_SIGSET_BYTES];
    set_bytes.copy_from_slice(src);
    let child_exit_seq = process.child_exit_seq.load(Ordering::Acquire);
    let old_mask = ext
        .signal_mask
        .swap(u64::from_ne_bytes(set_bytes), Ordering::AcqRel);
    ext.sigsuspend_active.store(true, Ordering::Release);

    ext.signal_wait.wait_until(|| {
        has_unblocked_pending_signal(ext)
            || (!signal_is_blocked(ext, SIGCHLD_NUM as i32)
                && (process.has_exited_child()
                    || process.child_exit_seq.load(Ordering::Acquire) != child_exit_seq))
    });
    if ext.pending_signal.load(Ordering::Acquire) == 0
        && !signal_is_blocked(ext, SIGCHLD_NUM as i32)
        && (process.has_exited_child()
            || process.child_exit_seq.load(Ordering::Acquire) != child_exit_seq)
    {
        ext.pending_signal
            .store(SIGCHLD_NUM as i32, Ordering::Release);
    }
    ext.sigsuspend_active.store(false, Ordering::Release);
    ext.signal_mask.store(old_mask, Ordering::Release);
    neg_errno(LinuxError::EINTR)
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

fn read_execve_envp(process: &UserProcess, envp_ptr: usize) -> Result<Vec<String>, LinuxError> {
    const MAX_ENVC: usize = 256;

    if envp_ptr == 0 {
        return Ok(Vec::new());
    }

    let mut envp = Vec::new();
    for idx in 0..MAX_ENVC {
        let item_ptr = read_user_word(process, envp_ptr + idx * size_of::<usize>())?;
        if item_ptr == 0 {
            break;
        }
        envp.push(read_cstr(process, item_ptr)?);
    }
    Ok(envp)
}

fn current_cwd() -> String {
    std::env::current_dir().unwrap_or_else(|_| "/".into())
}

fn resolve_host_path(cwd: String, path: &str) -> Result<String, String> {
    crate::linux_fs::normalize_path(cwd.as_str(), path)
        .ok_or_else(|| format!("invalid path: {path}"))
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
        vec![
            crate::linux_fs::normalize_path("/", path)
                .ok_or_else(|| format!("invalid path: {path}"))?,
        ]
    };
    candidates
        .into_iter()
        .find(|candidate| matches!(std::fs::metadata(candidate), Ok(meta) if meta.is_file()))
        .ok_or_else(|| format!("runtime support file not found: {path}"))
}

fn runtime_absolute_path_candidates(exec_root: &str, path: &str) -> Vec<String> {
    let Some(normalized) = crate::linux_fs::normalize_path("/", path) else {
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
    let normalized = crate::linux_fs::normalize_path("/", path)?;
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
            Self::File(desc) => Ok(Self::File(Arc::clone(desc))),
            Self::Directory(desc) => Ok(Self::Directory(Arc::clone(desc))),
            Self::Pipe(pipe) => Ok(Self::Pipe(pipe.clone())),
        }
    }
}

impl FdSlot {
    fn new(entry: FdEntry, fd_flags: crate::linux_fs::FdFlags) -> Self {
        Self { fd_flags, entry }
    }

    fn duplicate_for_fork(&self) -> Result<Self, LinuxError> {
        Ok(Self {
            fd_flags: self.fd_flags,
            entry: self.entry.duplicate_for_fork()?,
        })
    }
}

impl FdTable {
    fn new() -> Self {
        Self {
            entries: vec![
                Some(FdSlot::new(
                    FdEntry::Stdin,
                    crate::linux_fs::FdFlags::empty(),
                )),
                Some(FdSlot::new(
                    FdEntry::Stdout,
                    crate::linux_fs::FdFlags::empty(),
                )),
                Some(FdSlot::new(
                    FdEntry::Stderr,
                    crate::linux_fs::FdFlags::empty(),
                )),
            ],
        }
    }

    fn fork_copy(&self) -> Result<Self, LinuxError> {
        let mut entries = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            entries.push(match entry {
                Some(slot) => Some(slot.duplicate_for_fork()?),
                None => None,
            });
        }
        Ok(Self { entries })
    }

    fn dirfd_base_path(&self, dirfd: i32) -> Result<Option<String>, LinuxError> {
        if dirfd == AT_FDCWD_I32 {
            return Ok(None);
        }
        match self.entry(dirfd)? {
            FdEntry::Directory(desc) => Ok(Some(desc.path().to_string())),
            _ => Err(LinuxError::ENOTDIR),
        }
    }

    fn is_stdio(&self, fd: i32) -> bool {
        if !matches!(fd, 0..=2) {
            return false;
        }
        matches!(
            self.entries.get(fd as usize),
            Some(Some(FdSlot {
                entry: FdEntry::Stdin | FdEntry::Stdout | FdEntry::Stderr,
                ..
            }))
        )
    }

    fn poll(&self, fd: i32, mode: SelectMode) -> bool {
        let Ok(entry) = self.entry(fd) else {
            return matches!(mode, SelectMode::Except);
        };
        match mode {
            SelectMode::Read => match entry {
                FdEntry::Stdin => false,
                FdEntry::Stdout | FdEntry::Stderr => false,
                FdEntry::DevNull | FdEntry::File(_) | FdEntry::Directory(_) => true,
                FdEntry::Pipe(pipe) => pipe.poll().readable,
            },
            SelectMode::Write => match entry {
                FdEntry::Stdin => false,
                FdEntry::Stdout | FdEntry::Stderr | FdEntry::DevNull => true,
                FdEntry::File(_) => true,
                FdEntry::Directory(_) => false,
                FdEntry::Pipe(pipe) => pipe.poll().writable,
            },
            SelectMode::Except => false,
        }
    }

    fn read(&mut self, fd: i32, dst: &mut [u8]) -> Result<usize, LinuxError> {
        match self.entry(fd)? {
            FdEntry::Stdin => Ok(0),
            FdEntry::DevNull => Ok(0),
            FdEntry::File(desc) => desc.read_file(dst),
            FdEntry::Directory(_) => Err(LinuxError::EISDIR),
            FdEntry::Pipe(pipe) => pipe.read(dst),
            _ => Err(LinuxError::EBADF),
        }
    }

    fn write(&mut self, fd: i32, src: &[u8]) -> Result<usize, LinuxError> {
        match self.entry(fd)? {
            FdEntry::Stdout | FdEntry::Stderr => {
                axhal::console::write_bytes(src);
                Ok(src.len())
            }
            FdEntry::DevNull => Ok(src.len()),
            FdEntry::File(desc) => desc.write_file(src),
            FdEntry::Pipe(pipe) => pipe.write(src),
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
        let mut fd_flags = crate::linux_fs::FdFlags::empty();
        fd_flags.set_cloexec(flags & general::O_CLOEXEC != 0);
        self.insert_with_flags(entry, fd_flags)
    }

    fn close(&mut self, fd: i32) -> Result<(), LinuxError> {
        if !(0..self.entries.len() as i32).contains(&fd) || self.entries[fd as usize].is_none() {
            return Err(LinuxError::EBADF);
        }
        self.entries[fd as usize] = None;
        Ok(())
    }

    fn close_all(&mut self) {
        self.entries.clear();
    }

    fn close_cloexec(&mut self) {
        for slot in &mut self.entries {
            if slot.as_ref().is_some_and(|slot| slot.fd_flags.cloexec()) {
                *slot = None;
            }
        }
    }

    fn stat(&mut self, fd: i32) -> Result<general::stat, LinuxError> {
        match self.entry_mut(fd)? {
            FdEntry::Stdin => Ok(stdio_stat(true)),
            FdEntry::Stdout | FdEntry::Stderr => Ok(stdio_stat(false)),
            FdEntry::DevNull => Ok(stdio_stat(false)),
            FdEntry::File(desc) | FdEntry::Directory(desc) => {
                Ok(file_attr_to_stat(&desc.attr()?, Some(desc.path())))
            }
            FdEntry::Pipe(pipe) => Ok(pipe.stat()),
        }
    }

    fn truncate(&mut self, fd: i32, size: u64) -> Result<(), LinuxError> {
        match self.entry(fd)? {
            FdEntry::File(desc) => desc.truncate_file(size),
            FdEntry::DevNull => Ok(()),
            _ => Err(LinuxError::EINVAL),
        }
    }

    fn sync(&mut self, fd: i32) -> Result<(), LinuxError> {
        match self.entry(fd)? {
            FdEntry::File(desc) => desc.sync_file(),
            FdEntry::DevNull => Ok(()),
            _ => Err(LinuxError::EINVAL),
        }
    }

    fn file_status_flags(&self, fd: i32) -> Result<u32, LinuxError> {
        match self.entry(fd)? {
            FdEntry::File(desc) | FdEntry::Directory(desc) => Ok(desc.status_flags.lock().raw()),
            FdEntry::Pipe(pipe) => Ok(if pipe.readable {
                general::O_RDONLY
            } else {
                general::O_WRONLY
            }),
            FdEntry::Stdin => Ok(general::O_RDONLY),
            FdEntry::Stdout | FdEntry::Stderr => Ok(general::O_WRONLY),
            FdEntry::DevNull => Ok(general::O_RDWR),
        }
    }

    fn set_file_status_flags(&mut self, fd: i32, flags: u32) -> Result<(), LinuxError> {
        match self.entry(fd)? {
            FdEntry::File(desc) | FdEntry::Directory(desc) => {
                desc.status_flags.lock().set_raw(flags);
                Ok(())
            }
            FdEntry::Pipe(_)
            | FdEntry::Stdin
            | FdEntry::Stdout
            | FdEntry::Stderr
            | FdEntry::DevNull => Ok(()),
        }
    }

    fn fcntl(&mut self, fd: i32, cmd: u32, arg: usize) -> Result<i32, LinuxError> {
        let _ = self.slot(fd)?;
        match cmd {
            general::F_DUPFD => {
                self.dup_min_with_flags(fd, arg as i32, crate::linux_fs::FdFlags::empty())
            }
            general::F_DUPFD_CLOEXEC => {
                let mut fd_flags = crate::linux_fs::FdFlags::empty();
                fd_flags.set_cloexec(true);
                self.dup_min_with_flags(fd, arg as i32, fd_flags)
            }
            general::F_GETFD => Ok(self.slot(fd)?.fd_flags.raw() as i32),
            general::F_SETFD => {
                self.slot_mut(fd)?.fd_flags = crate::linux_fs::FdFlags::from_raw(arg as u32);
                Ok(0)
            }
            general::F_GETFL => Ok(self.file_status_flags(fd)? as i32),
            general::F_SETFL => {
                let mutable_flags = arg as u32
                    & (crate::linux_fs::OpenStatusFlags::APPEND
                        | crate::linux_fs::OpenStatusFlags::NONBLOCK);
                self.set_file_status_flags(fd, mutable_flags)?;
                Ok(0)
            }
            _ => Ok(0),
        }
    }

    fn lseek(&mut self, fd: i32, offset: i64, whence: u32) -> Result<u64, LinuxError> {
        match self.entry(fd)? {
            FdEntry::File(desc) => desc.seek_file(offset, whence),
            FdEntry::DevNull => Ok(0),
            FdEntry::Directory(_) => Err(LinuxError::EISDIR),
            FdEntry::Pipe(_) => Err(LinuxError::ESPIPE),
            _ => Err(LinuxError::ESPIPE),
        }
    }

    fn dup(&mut self, fd: i32) -> Result<i32, LinuxError> {
        self.dup_min(fd, 0)
    }

    fn dup_min(&mut self, fd: i32, min_fd: i32) -> Result<i32, LinuxError> {
        self.dup_min_with_flags(fd, min_fd, crate::linux_fs::FdFlags::empty())
    }

    fn dup_min_with_flags(
        &mut self,
        fd: i32,
        min_fd: i32,
        fd_flags: crate::linux_fs::FdFlags,
    ) -> Result<i32, LinuxError> {
        if min_fd < 0 {
            return Err(LinuxError::EINVAL);
        }
        let entry = self.entry(fd)?.duplicate_for_fork()?;
        self.insert_min_with_flags(entry, min_fd as usize, fd_flags)
    }

    fn dup3(&mut self, oldfd: i32, newfd: i32, flags: u32) -> Result<i32, LinuxError> {
        if flags & !general::O_CLOEXEC != 0 || oldfd == newfd {
            return Err(LinuxError::EINVAL);
        }
        let entry = self.entry(oldfd)?.duplicate_for_fork()?;
        let mut fd_flags = crate::linux_fs::FdFlags::empty();
        fd_flags.set_cloexec(flags & general::O_CLOEXEC != 0);
        if newfd < 0 {
            return Err(LinuxError::EBADF);
        }
        let newfd = newfd as usize;
        if self.entries.len() <= newfd {
            self.entries.resize_with(newfd + 1, || None);
        }
        self.entries[newfd] = Some(FdSlot::new(entry, fd_flags));
        Ok(newfd as i32)
    }

    fn getdents64(&mut self, fd: i32, dst: &mut [u8]) -> Result<usize, LinuxError> {
        let FdEntry::Directory(desc) = self.entry(fd)? else {
            return Err(LinuxError::ENOTDIR);
        };
        let crate::linux_fs::OpenFileBackend::Directory(dir_backend) = &desc.backend else {
            return Err(LinuxError::ENOTDIR);
        };
        let mut dir = dir_backend.dir.lock();
        let mut read_buf: [fops::DirEntry; 16] =
            core::array::from_fn(|_| fops::DirEntry::default());
        let count = dir.read_dir(&mut read_buf).map_err(LinuxError::from)?;
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
        let FdEntry::File(desc) = self.entry(fd)? else {
            return Err(LinuxError::EBADF);
        };
        desc.read_file_at(offset, len)
    }

    fn insert(&mut self, entry: FdEntry) -> Result<i32, LinuxError> {
        self.insert_with_flags(entry, crate::linux_fs::FdFlags::empty())
    }

    fn insert_with_flags(
        &mut self,
        entry: FdEntry,
        fd_flags: crate::linux_fs::FdFlags,
    ) -> Result<i32, LinuxError> {
        self.insert_min_with_flags(entry, 0, fd_flags)
    }

    fn insert_min_with_flags(
        &mut self,
        entry: FdEntry,
        min_fd: usize,
        fd_flags: crate::linux_fs::FdFlags,
    ) -> Result<i32, LinuxError> {
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
            *slot = Some(FdSlot::new(entry, fd_flags));
            return Ok(idx as i32);
        }
        self.entries.push(Some(FdSlot::new(entry, fd_flags)));
        Ok((self.entries.len() - 1) as i32)
    }

    fn entry(&self, fd: i32) -> Result<&FdEntry, LinuxError> {
        self.entries
            .get(fd as usize)
            .and_then(|slot| slot.as_ref())
            .map(|slot| &slot.entry)
            .ok_or(LinuxError::EBADF)
    }

    fn entry_mut(&mut self, fd: i32) -> Result<&mut FdEntry, LinuxError> {
        self.entries
            .get_mut(fd as usize)
            .and_then(|slot| slot.as_mut())
            .map(|slot| &mut slot.entry)
            .ok_or(LinuxError::EBADF)
    }

    fn slot(&self, fd: i32) -> Result<&FdSlot, LinuxError> {
        self.entries
            .get(fd as usize)
            .and_then(|slot| slot.as_ref())
            .ok_or(LinuxError::EBADF)
    }

    fn slot_mut(&mut self, fd: i32) -> Result<&mut FdSlot, LinuxError> {
        self.entries
            .get_mut(fd as usize)
            .and_then(|slot| slot.as_mut())
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
                return open_fd_candidates(&[path], prefer_dir, flags, &opts);
            }
            runtime_absolute_path_candidates(exec_root.as_str(), path)
        } else {
            let cwd = process.cwd();
            let primary =
                crate::linux_fs::normalize_path(cwd.as_str(), path).ok_or(LinuxError::EINVAL)?;
            let mut candidates = vec![primary];
            for extra in runtime_library_name_candidates(exec_root.as_str(), path) {
                push_runtime_candidate(&mut candidates, Some(extra));
            }
            candidates
        };
        if candidates.is_empty() {
            return Err(LinuxError::EINVAL);
        }
        open_fd_candidates(&candidates, prefer_dir, flags, &opts)
    } else {
        let FdEntry::Directory(dir) = table.entry(dirfd)? else {
            return Err(LinuxError::ENOTDIR);
        };
        let primary =
            crate::linux_fs::normalize_path(dir.path(), path).ok_or(LinuxError::EINVAL)?;
        let mut candidates = vec![primary];
        for extra in runtime_library_name_candidates(exec_root.as_str(), path) {
            push_runtime_candidate(&mut candidates, Some(extra));
        }
        open_fd_candidates(&candidates, prefer_dir, flags, &opts)
    }
}

fn open_fd_candidates(
    candidates: &[String],
    prefer_dir: bool,
    flags: u32,
    opts: &OpenOptions,
) -> Result<FdEntry, LinuxError> {
    let mut last_err = LinuxError::ENOENT;
    let may_open_dir_readonly = flags & general::O_ACCMODE == general::O_RDONLY
        && flags & (general::O_CREAT | general::O_TRUNC) == 0;
    for path in candidates {
        if path == "/dev/null" {
            if prefer_dir {
                return Err(LinuxError::ENOTDIR);
            }
            return Ok(FdEntry::DevNull);
        }
        if prefer_dir || may_open_dir_readonly {
            match axfs::api::metadata(path.as_str()).map_err(LinuxError::from) {
                Ok(metadata) if metadata.is_dir() => return open_dir_entry(path.as_str()),
                Ok(_) if prefer_dir => return Err(LinuxError::ENOTDIR),
                Ok(_) => {}
                Err(err) => {
                    last_err = err;
                    if prefer_dir {
                        if err != LinuxError::ENOENT {
                            return Err(err);
                        }
                        continue;
                    }
                }
            }
        }
        match File::open(path.as_str(), opts) {
            Ok(file) => {
                let desc = Arc::new(crate::linux_fs::OpenFileDescription::new_file(
                    file,
                    path.clone(),
                    crate::linux_fs::OpenStatusFlags::from_raw(flags & !general::O_CLOEXEC),
                ));
                return Ok(FdEntry::File(desc));
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
    let normalized = crate::linux_fs::normalize_path("/", path)?;
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
    let desc = Arc::new(crate::linux_fs::OpenFileDescription::new_directory(
        dir,
        attr,
        path.into(),
    ));
    Ok(FdEntry::Directory(desc))
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

fn rename_path_abs(old_path: &str, new_path: &str) -> Result<(), LinuxError> {
    match axfs::api::rename(old_path, new_path).map_err(LinuxError::from) {
        Ok(()) => Ok(()),
        Err(LinuxError::EOPNOTSUPP | LinuxError::ENOSYS) => {
            compat_empty_dir_rename(old_path, new_path)
        }
        Err(err) => Err(err),
    }
}

fn compat_empty_dir_rename(old_path: &str, new_path: &str) -> Result<(), LinuxError> {
    // compat(busybox-filesystem-phase1b): axfs currently reports unsupported
    // for directory rename, while BusyBox `mv empty_dir new_dir` needs the
    // Linux-visible state change.
    // delete-when: axfs::api::rename supports directory rename semantics.
    let old_meta = axfs::api::metadata(old_path).map_err(LinuxError::from)?;
    if !old_meta.is_dir() {
        return Err(LinuxError::EOPNOTSUPP);
    }
    match axfs::api::metadata(new_path) {
        Ok(_) => return Err(LinuxError::EEXIST),
        Err(err) if LinuxError::from(err) == LinuxError::ENOENT => {}
        Err(err) => return Err(LinuxError::from(err)),
    }
    axfs::api::create_dir(new_path).map_err(LinuxError::from)?;
    match axfs::api::remove_dir(old_path).map_err(LinuxError::from) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = axfs::api::remove_dir(new_path);
            Err(err)
        }
    }
}

fn stat_path_abs(path: &str) -> Result<general::stat, LinuxError> {
    if path == "/dev/null" {
        return Ok(stdio_stat(false));
    }
    let mut opts = OpenOptions::new();
    opts.read(true);
    match File::open(path, &opts) {
        Ok(file) => {
            let attr = file.get_attr().map_err(LinuxError::from)?;
            Ok(file_attr_to_stat(&attr, Some(path)))
        }
        Err(err) => {
            let err = LinuxError::from(err);
            if err != LinuxError::EISDIR {
                return Err(err);
            }
            match open_dir_entry(path)? {
                FdEntry::Directory(desc) => Ok(file_attr_to_stat(&desc.attr()?, Some(desc.path()))),
                _ => Err(LinuxError::EINVAL),
            }
        }
    }
}

fn resolve_dirfd_path(
    process: &UserProcess,
    table: &FdTable,
    dirfd: i32,
    path: &str,
) -> Result<String, LinuxError> {
    let cwd = process.cwd();
    let dirfd_base = table.dirfd_base_path(dirfd)?;
    crate::linux_fs::resolve_at_path(
        cwd.as_str(),
        dirfd_base.as_deref(),
        path,
        crate::linux_fs::ResolveOptions::default(),
    )
    .map(|resolved| resolved.path)
}

#[allow(dead_code)]
fn resolve_dirfd_path_allow_empty(
    process: &UserProcess,
    table: &FdTable,
    dirfd: i32,
    path: &str,
) -> Result<String, LinuxError> {
    let cwd = process.cwd();
    let dirfd_base = table.dirfd_base_path(dirfd)?;
    crate::linux_fs::resolve_at_path(
        cwd.as_str(),
        dirfd_base.as_deref(),
        path,
        crate::linux_fs::ResolveOptions::allow_empty(),
    )
    .map(|resolved| resolved.path)
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
