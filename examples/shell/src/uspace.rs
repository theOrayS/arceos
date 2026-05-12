use core::cmp;
use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use arceos_posix_api::ctypes as posix_ctypes;
use axerrno::LinuxError;
use axfs::fops::{self, Directory, File, OpenOptions};
use axhal::context::{TrapFrame, UspaceContext};
use axhal::mem::virt_to_phys;
use axhal::trap::{
    PAGE_FAULT, PageFaultFlags, SYSCALL, register_trap_handler, register_user_return_handler,
};
use axio::SeekFrom;
use axmm::AddrSpace;
use axns::AxNamespace;
use axsync::Mutex;
use axtask::{AxTaskRef, TaskInner, WaitQueue};
use linux_raw_sys::{general, ioctl};
use memory_addr::{PAGE_SIZE_4K, PageIter4K, VirtAddr};
use std::collections::BTreeMap;
use std::string::{String, ToString};
use std::sync::Arc;
use std::vec::Vec;

#[cfg(target_arch = "riscv64")]
use riscv::register::sstatus::{FS, Sstatus};

mod credentials;
mod fd_pipe;
mod fd_socket;
mod fd_table;
mod futex;
mod linux_abi;
mod memory_map;
mod memory_policy;
mod metadata;
mod program_loader;
mod resource_sched;
mod runtime_paths;
mod select_fdset;
mod signal_abi;
mod synthetic_fs;
mod system_info;
mod sysv_shm;
mod task_context;
mod task_registry;
mod time_abi;
mod user_memory;

use credentials::{
    access_allowed, apply_chown_metadata, chown_ids, read_group_list, set_fs_id, set_re_ids,
    set_res_ids, set_single_id, write_group_list, write_id_triplet,
};
use fd_pipe::PipeEndpoint;
use fd_socket::{
    LocalSocketEntry, SocketEntry, read_socket_addr_from_user, read_socket_data_from_user,
    recv_socket_data_to_user, recv_socket_data_to_user_with_addr, write_socket_addr_to_user,
};
use fd_table::{DirectoryEntry, FdEntry, FdTable, FileEntry, PathEntry};
use linux_abi::*;
use memory_map::{align_down, align_up, mmap_prot_to_flags, user_mapping_flags};
use memory_policy::{validate_mempolicy_nodemask, write_default_mempolicy};
use metadata::{
    apply_recorded_path_metadata, canonical_permission_path, dirent_type, fd_entry_path,
    fd_entry_statfs_path, file_attr_to_stat, generic_statfs, normalize_file_mode, stdio_stat,
};
use program_loader::load_program_image;
use resource_sched::{
    UserRlimit, UserSchedParam, default_rlimit, default_sched_param, rlimit_is_valid,
    sched_param_accepts_policy, sched_param_accepts_setparam,
};
use runtime_paths::{
    busybox_applet_target_path, current_cwd, normalize_path, push_runtime_candidate,
    resolve_host_path, runtime_absolute_path_candidates, runtime_library_name_candidates,
};
use select_fdset::{SelectMode, poll_fd_set, read_fd_set, read_pselect_deadline, write_fd_set};
#[cfg(target_arch = "riscv64")]
use signal_abi::{
    RiscvSignalFrame, apply_riscv_sigcontext, make_riscv_signal_frame, riscv_signal_frame_offsets,
    riscv_signal_frame_size, trap_frame_to_riscv_sigcontext,
};
use synthetic_fs::{
    dev_shm_host_path, ensure_dev_shm_dir, is_proc_self_maps_path, proc_exe_link_target,
    proc_self_maps_fd_entry, proc_self_maps_is_writable_open, proc_self_maps_path_entry,
    synthetic_file_is_writable_open, synthetic_userdb_content, synthetic_userdb_fd_entry,
    synthetic_userdb_path_entry,
};
use system_info::{SyslogAction, default_rusage, default_utsname, syslog_action};
use task_context::{UserTaskExt, current_process, current_task_ext, current_tid, task_ext};
use task_registry::{
    UserThreadEntry, register_user_task, unregister_user_task, user_thread_entry_by_process_pid,
    user_thread_entry_by_tid, user_thread_entry_for_process,
};
use time_abi::{
    UserTimex, adjtimex_changes_clock, adjtimex_input_valid, clock_now_duration,
    clock_resolution_timespec, current_timeval, default_timex, default_tms,
    itimerval_to_micros_pair, micros_to_duration, micros_to_timeval, monotonic_time_micros,
    read_timespec_duration, rtc_time_from_wall_time, set_realtime_offset_from_timespec,
    sleep_duration, socket_duration_to_timeval, socket_timeval_to_duration, timespec_from_duration,
    validate_clock_id, zero_timespec, zero_timezone,
};
use user_memory::{
    clear_user_bytes, read_cstr, read_execve_argv, read_iovec_entries, read_user_bytes,
    read_user_value, user_io_buffer, validate_user_read, validate_user_write,
    with_readable_user_buffer, with_writable_user_buffer, write_user_bytes, write_user_value,
};

static USER_RETURN_HOOK_REGISTERED: AtomicBool = AtomicBool::new(false);

macro_rules! user_trace {
    ($($arg:tt)*) => {};
}

macro_rules! return_on_user_write_error {
    ($process:expr, $ptr:expr, $value:expr) => {
        let ret = write_user_value($process, $ptr, $value);
        if ret != 0 {
            return ret;
        }
    };
}

macro_rules! read_cstr_or_return {
    ($process:expr, $ptr:expr) => {
        match read_cstr($process, $ptr) {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        }
    };
}

macro_rules! socket_entry_or_return {
    ($process:expr, $fd:expr) => {
        match socket_entry($process, $fd) {
            Ok(socket) => socket,
            Err(err) => return neg_errno(err),
        }
    };
}

macro_rules! return_errno_if {
    ($condition:expr, $err:expr) => {
        if $condition {
            return neg_errno($err);
        }
    };
}

macro_rules! return_on_fd_set_write_error {
    ($process:expr, $ptr:expr, $bits:expr) => {
        let ret = write_fd_set($process, $ptr, $bits);
        if ret != 0 {
            return ret;
        }
    };
}

struct AxNamespaceImpl;

struct UserProcess {
    aspace: Mutex<AddrSpace>,
    brk: Mutex<BrkState>,
    fds: Mutex<FdTable>,
    cwd: Mutex<String>,
    exec_root: Mutex<String>,
    exec_path: Mutex<String>,
    children: Mutex<Vec<ChildTask>>,
    child_exit_wait: WaitQueue,
    rlimits: Mutex<BTreeMap<u32, UserRlimit>>,
    signal_actions: Mutex<BTreeMap<usize, general::kernel_sigaction>>,
    path_modes: Mutex<BTreeMap<String, u32>>,
    path_owners: Mutex<BTreeMap<String, (u32, u32)>>,
    shm_attachments: Mutex<BTreeMap<usize, (i32, usize)>>,
    real_uid: AtomicU32,
    uid: AtomicU32,
    saved_uid: AtomicU32,
    real_gid: AtomicU32,
    gid: AtomicU32,
    saved_gid: AtomicU32,
    groups: Mutex<Vec<u32>>,
    personality: AtomicUsize,
    real_timer_generation: AtomicU64,
    real_timer_deadline_us: AtomicU64,
    real_timer_interval_us: AtomicU64,
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

struct ChildTask {
    pid: i32,
    task: AxTaskRef,
    process: Arc<UserProcess>,
}

struct LoadedProgram {
    process: Arc<UserProcess>,
    context: UspaceContext,
}

const NO_EXIT_GROUP_CODE: i32 = i32::MIN;

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
    task.init_task_ext(UserTaskExt::new(loaded.process.clone(), 0, 0));
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
        cwd: Mutex::new(cwd.into()),
        exec_root: Mutex::new(image.exec_root.clone()),
        exec_path: Mutex::new(image.exec_path.clone()),
        children: Mutex::new(Vec::new()),
        child_exit_wait: WaitQueue::new(),
        rlimits: Mutex::new(BTreeMap::new()),
        signal_actions: Mutex::new(BTreeMap::new()),
        path_modes: Mutex::new(BTreeMap::new()),
        path_owners: Mutex::new(BTreeMap::new()),
        shm_attachments: Mutex::new(BTreeMap::new()),
        real_uid: AtomicU32::new(0),
        uid: AtomicU32::new(0),
        saved_uid: AtomicU32::new(0),
        real_gid: AtomicU32::new(0),
        gid: AtomicU32::new(0),
        saved_gid: AtomicU32::new(0),
        groups: Mutex::new(Vec::new()),
        personality: AtomicUsize::new(0),
        real_timer_generation: AtomicU64::new(0),
        real_timer_deadline_us: AtomicU64::new(0),
        real_timer_interval_us: AtomicU64::new(0),
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
    process.set_exec_path(image.exec_path);
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
        self.aspace.lock().clear();
        let mut fds = self.fds.lock();
        fds.close_all();
        *fds = FdTable::new();
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

    fn real_uid(&self) -> u32 {
        self.real_uid.load(Ordering::Acquire)
    }

    fn uid(&self) -> u32 {
        self.uid.load(Ordering::Acquire)
    }

    fn saved_uid(&self) -> u32 {
        self.saved_uid.load(Ordering::Acquire)
    }

    fn real_gid(&self) -> u32 {
        self.real_gid.load(Ordering::Acquire)
    }

    fn gid(&self) -> u32 {
        self.gid.load(Ordering::Acquire)
    }

    fn saved_gid(&self) -> u32 {
        self.saved_gid.load(Ordering::Acquire)
    }

    fn set_uid(&self, uid: u32) {
        self.real_uid.store(uid, Ordering::Release);
        self.uid.store(uid, Ordering::Release);
        self.saved_uid.store(uid, Ordering::Release);
    }

    fn set_gid(&self, gid: u32) {
        self.real_gid.store(gid, Ordering::Release);
        self.gid.store(gid, Ordering::Release);
        self.saved_gid.store(gid, Ordering::Release);
    }

    fn personality(&self) -> usize {
        self.personality.load(Ordering::Acquire)
    }

    fn set_personality(&self, persona: usize) {
        self.personality
            .store(persona & LINUX_PERSONALITY_MASK, Ordering::Release);
    }

    fn set_user_ids(&self, real: Option<u32>, effective: Option<u32>, saved: Option<u32>) {
        if let Some(uid) = real {
            self.real_uid.store(uid, Ordering::Release);
        }
        if let Some(uid) = effective {
            self.uid.store(uid, Ordering::Release);
        }
        if let Some(uid) = saved {
            self.saved_uid.store(uid, Ordering::Release);
        }
    }

    fn set_group_ids(&self, real: Option<u32>, effective: Option<u32>, saved: Option<u32>) {
        if let Some(gid) = real {
            self.real_gid.store(gid, Ordering::Release);
        }
        if let Some(gid) = effective {
            self.gid.store(gid, Ordering::Release);
        }
        if let Some(gid) = saved {
            self.saved_gid.store(gid, Ordering::Release);
        }
    }

    fn set_path_mode(&self, path: String, mode: u32) {
        self.path_modes
            .lock()
            .insert(path, normalize_file_mode(mode));
    }

    fn path_mode(&self, path: &str) -> Option<u32> {
        self.path_modes.lock().get(path).copied()
    }

    fn set_path_owner(&self, path: String, owner: Option<u32>, group: Option<u32>) {
        let mut path_owners = self.path_owners.lock();
        let (current_owner, current_group) =
            path_owners.get(path.as_str()).copied().unwrap_or((0, 0));
        path_owners.insert(
            path,
            (
                owner.unwrap_or(current_owner),
                group.unwrap_or(current_group),
            ),
        );
    }

    fn path_owner(&self, path: &str) -> Option<(u32, u32)> {
        self.path_owners.lock().get(path).copied()
    }

    fn clear_path_chown_special_bits(&self, path: &str, current_mode: u32) {
        let mode = self
            .path_mode(path)
            .unwrap_or(current_mode & FILE_MODE_PERMISSION_MASK);
        let mut updated_mode = mode & !FILE_MODE_SET_UID;
        if mode & FILE_MODE_GROUP_EXECUTE != 0 {
            updated_mode &= !FILE_MODE_SET_GID;
        }
        self.set_path_mode(path.to_string(), updated_mode);
    }

    fn groups(&self) -> Vec<u32> {
        self.groups.lock().clone()
    }

    fn set_groups(&self, groups: Vec<u32>) {
        *self.groups.lock() = groups;
    }

    fn has_group(&self, gid: u32) -> bool {
        self.gid() == gid || self.groups.lock().contains(&gid)
    }

    fn real_timer_active(&self) -> bool {
        self.real_timer_deadline_us.load(Ordering::Acquire) != 0
    }

    fn add_thread(&self) {
        self.live_threads.fetch_add(1, Ordering::AcqRel);
    }

    fn note_thread_exit(&self, code: i32) {
        self.exit_code.store(code, Ordering::Release);
        let live_before = self.live_threads.fetch_sub(1, Ordering::AcqRel);
        if live_before == 1 {
            self.exit_wait.notify_all(false);
            notify_parent_child_exit(self.ppid);
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

    fn fork(&self) -> Result<Arc<UserProcess>, LinuxError> {
        let mut aspace = axmm::new_user_aspace(VirtAddr::from(USER_ASPACE_BASE), USER_ASPACE_SIZE)
            .map_err(LinuxError::from)?;
        {
            let parent_aspace = self.aspace.lock();
            aspace
                .clone_user_mappings_from(&parent_aspace)
                .map_err(LinuxError::from)?;
        }

        Ok(Arc::new(UserProcess {
            aspace: Mutex::new(aspace),
            brk: Mutex::new(*self.brk.lock()),
            fds: Mutex::new(self.fds.lock().fork_copy()?),
            cwd: Mutex::new(self.cwd()),
            exec_root: Mutex::new(self.exec_root()),
            exec_path: Mutex::new(self.exec_path()),
            children: Mutex::new(Vec::new()),
            child_exit_wait: WaitQueue::new(),
            rlimits: Mutex::new(self.rlimits.lock().clone()),
            signal_actions: Mutex::new(self.signal_actions.lock().clone()),
            path_modes: Mutex::new(self.path_modes.lock().clone()),
            path_owners: Mutex::new(self.path_owners.lock().clone()),
            shm_attachments: Mutex::new(self.shm_attachments.lock().clone()),
            real_uid: AtomicU32::new(self.real_uid()),
            uid: AtomicU32::new(self.uid()),
            saved_uid: AtomicU32::new(self.saved_uid()),
            real_gid: AtomicU32::new(self.real_gid()),
            gid: AtomicU32::new(self.gid()),
            saved_gid: AtomicU32::new(self.saved_gid()),
            groups: Mutex::new(self.groups()),
            personality: AtomicUsize::new(self.personality()),
            real_timer_generation: AtomicU64::new(0),
            real_timer_deadline_us: AtomicU64::new(0),
            real_timer_interval_us: AtomicU64::new(0),
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

                if let Some(index) = exited_index {
                    Some(children.remove(index))
                } else if nohang {
                    return Ok(None);
                } else {
                    match pid {
                        p if p > 0 => {
                            let index = children
                                .iter()
                                .position(|child| child.pid == p)
                                .ok_or(LinuxError::ECHILD)?;
                            Some(children.remove(index))
                        }
                        -1 => None,
                        _ => return Err(LinuxError::EINVAL),
                    }
                }
            };

            if let Some(child) = maybe_child {
                break child;
            }
            self.child_exit_wait.wait_until(|| {
                let children = self.children.lock();
                children.is_empty() || children.iter().any(is_exited)
            });
        };
        let status = child.task.join().ok_or(LinuxError::ECHILD)?;
        let child_pid = child.pid;
        child.process.teardown();
        drop(child);
        axtask::yield_now();
        Ok(Some((child_pid, status)))
    }

    fn child_thread_entry_by_pid(&self, pid: i32) -> Option<UserThreadEntry> {
        let children = self.children.lock();
        children
            .iter()
            .find(|child| {
                child.pid == pid && child.process.live_threads.load(Ordering::Acquire) != 0
            })
            .map(|child| UserThreadEntry {
                task: child.task.clone(),
                process: child.process.clone(),
            })
    }
}

fn notify_parent_child_exit(ppid: i32) {
    if let Some(parent) = user_thread_entry_by_process_pid(ppid) {
        parent.process.child_exit_wait.notify_all(false);
    }
}

fn deliver_user_signal(entry: &UserThreadEntry, sig: i32) -> Result<(), LinuxError> {
    if sig == 0 {
        return Ok(());
    }
    let ext = task_ext(&entry.task).ok_or(LinuxError::ESRCH)?;
    if sig == SIGKILL_NUM {
        ext.process.request_exit_group(128 + sig);
    }
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
        let futex_wait = ext.futex_wait.load(Ordering::Acquire);
        if futex_wait != 0 {
            futex::wake_task(futex_wait, &entry.task);
        }
    }
    Ok(())
}

fn deliver_user_signal_result(entry: &UserThreadEntry, sig: i32) -> isize {
    match deliver_user_signal(entry, sig) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
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
    let _ = futex::wake_addr(clear_tid, 1);
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
        if sig == SIGKILL_NUM {
            ext.process.request_exit_group(128 + sig);
            terminate_current_thread(ext.process.as_ref(), 128 + sig);
        }
        return Ok(());
    }
    let current_mask = ext.signal_mask.load(Ordering::Acquire);
    let frame_size = riscv_signal_frame_size();
    let frame_addr = align_down(tf.regs.sp.saturating_sub(frame_size), 16);
    ensure_signal_frame_pages(ext.process.as_ref(), frame_addr, frame_size)?;

    let frame = make_riscv_signal_frame(
        sig,
        SI_TKILL_CODE,
        current_tid(),
        current_mask,
        SS_DISABLE,
        RISCV_SIGTRAMP_CODE,
        trap_frame_to_riscv_sigcontext(tf),
    );

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

    let frame_offsets = riscv_signal_frame_offsets();
    tf.regs.sp = frame_addr;
    tf.regs.ra = frame_addr + frame_offsets.trampoline;
    tf.regs.a0 = sig as usize;
    tf.regs.a1 = frame_addr + frame_offsets.info;
    tf.regs.a2 = frame_addr + frame_offsets.ucontext;
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
        general::__NR_read => sys_read(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_pread64 => sys_pread64(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3()),
        general::__NR_write => sys_write(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_pwrite64 => {
            sys_pwrite64(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_writev => sys_writev(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_readv => sys_readv(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_statfs => sys_statfs(&process, tf.arg0(), tf.arg1()),
        general::__NR_fstatfs => sys_fstatfs(&process, tf.arg0(), tf.arg1()),
        general::__NR_getcwd => sys_getcwd(&process, tf.arg0(), tf.arg1()),
        general::__NR_chdir => sys_chdir(&process, tf.arg0()),
        general::__NR_openat => sys_openat(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3()),
        general::__NR_mkdirat => sys_mkdirat(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_unlinkat => sys_unlinkat(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_pipe2 => sys_pipe2(&process, tf.arg0(), tf.arg1()),
        general::__NR_ftruncate => sys_ftruncate(&process, tf.arg0(), tf.arg1()),
        general::__NR_fchmod => sys_fchmod(&process, tf.arg0(), tf.arg1()),
        general::__NR_fchmodat => sys_fchmodat(&process, tf.arg0(), tf.arg1(), tf.arg2(), 0),
        general::__NR_fchmodat2 => {
            sys_fchmodat(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_fchown => sys_fchown(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_fchownat => sys_fchownat(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
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
        general::__NR_fsync | general::__NR_fdatasync => sys_fsync(&process, tf.arg0()),
        general::__NR_newfstatat => {
            sys_newfstatat(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_fstat => sys_fstat(&process, tf.arg0(), tf.arg1()),
        general::__NR_getdents64 => sys_getdents64(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_lseek => sys_lseek(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_dup => sys_dup(&process, tf.arg0()),
        general::__NR_dup3 => sys_dup3(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_fcntl => sys_fcntl(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_fchdir => sys_fchdir(&process, tf.arg0()),
        general::__NR_readlinkat => {
            sys_readlinkat(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_socket => sys_socket_bridge(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_bind => sys_bind_bridge(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_listen => sys_listen_bridge(&process, tf.arg0(), tf.arg1()),
        general::__NR_accept => sys_accept_bridge(&process, tf.arg0(), tf.arg1(), tf.arg2(), 0),
        general::__NR_accept4 => {
            sys_accept_bridge(&process, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3())
        }
        general::__NR_connect => sys_connect_bridge(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_sendto => sys_sendto_bridge(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
            tf.arg5(),
        ),
        general::__NR_recvfrom => sys_recvfrom_bridge(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
            tf.arg5(),
        ),
        general::__NR_shutdown => sys_shutdown_bridge(&process, tf.arg0(), tf.arg1()),
        general::__NR_getsockname => {
            sys_getsockname_bridge(&process, tf.arg0(), tf.arg1(), tf.arg2())
        }
        general::__NR_getpeername => {
            sys_getpeername_bridge(&process, tf.arg0(), tf.arg1(), tf.arg2())
        }
        general::__NR_setsockopt => sys_setsockopt_bridge(
            &process,
            tf.arg0(),
            tf.arg1(),
            tf.arg2(),
            tf.arg3(),
            tf.arg4(),
        ),
        general::__NR_getsockopt => sys_getsockopt_bridge(
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
        general::__NR_clock_settime => sys_clock_settime(&process, tf.arg0(), tf.arg1()),
        general::__NR_clock_getres => sys_clock_getres(&process, tf.arg0(), tf.arg1()),
        general::__NR_gettimeofday => sys_gettimeofday(&process, tf.arg0(), tf.arg1()),
        general::__NR_adjtimex => sys_adjtimex(&process, tf.arg0()),
        general::__NR_getrandom => sys_getrandom(&process, tf.arg0(), tf.arg1(), tf.arg2()),
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
        general::__NR_shmget => sys_shmget(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_shmat => sys_shmat(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_shmdt => sys_shmdt(&process, tf, tf.arg0()),
        general::__NR_shmctl => sys_shmctl(&process, tf.arg0(), tf.arg1(), tf.arg2()),
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
        general::__NR_getuid => process.real_uid() as isize,
        general::__NR_geteuid => process.uid() as isize,
        general::__NR_getgid => process.real_gid() as isize,
        general::__NR_getegid => process.gid() as isize,
        general::__NR_setuid => sys_setuid(&process, tf.arg0()),
        general::__NR_setgid => sys_setgid(&process, tf.arg0()),
        general::__NR_setreuid => sys_setreuid(&process, tf.arg0(), tf.arg1()),
        general::__NR_setregid => sys_setregid(&process, tf.arg0(), tf.arg1()),
        general::__NR_setresuid => sys_setresuid(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_getresuid => sys_getresuid(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_setresgid => sys_setresgid(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_getresgid => sys_getresgid(&process, tf.arg0(), tf.arg1(), tf.arg2()),
        general::__NR_setfsuid => sys_setfsuid(&process, tf.arg0()),
        general::__NR_setfsgid => sys_setfsgid(&process, tf.arg0()),
        general::__NR_getgroups => sys_getgroups(&process, tf.arg0(), tf.arg1()),
        general::__NR_setgroups => sys_setgroups(&process, tf.arg0(), tf.arg1()),
        general::__NR_umask => 0,
        general::__NR_personality => sys_personality(&process, tf.arg0()),
        general::__NR_prctl => 0,
        general::__NR_setpgid => sys_setpgid(&process, tf.arg0(), tf.arg1()),
        general::__NR_getpgid => sys_getpgid(&process, tf.arg0()),
        general::__NR_setsid => sys_setsid(&process),
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
    if let Ok(socket) = socket_entry(process, fd) {
        return recv_socket_data_to_user(process, socket.posix_fd, buf, count, 0);
    }
    with_writable_user_buffer(process, buf, count, |dst| {
        process.fds.lock().read(fd as i32, dst)
    })
}

fn sys_pread64(process: &UserProcess, fd: usize, buf: usize, count: usize, offset: usize) -> isize {
    with_writable_user_buffer(process, buf, count, |dst| {
        let mut table = process.fds.lock();
        let FdEntry::File(file) = table.entry_mut(fd as i32)? else {
            return Err(LinuxError::EBADF);
        };
        read_file_at_into(&file.file, offset as u64, dst)
    })
}

fn read_file_at_into(file: &File, offset: u64, dst: &mut [u8]) -> Result<usize, LinuxError> {
    let mut filled = 0usize;
    while filled < dst.len() {
        let read = file
            .read_at(offset + filled as u64, &mut dst[filled..])
            .map_err(LinuxError::from)?;
        if read == 0 {
            break;
        }
        filled += read;
    }
    Ok(filled)
}

fn sys_write(process: &UserProcess, fd: usize, buf: usize, count: usize) -> isize {
    with_readable_user_buffer(process, buf, count, |src| {
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
    with_readable_user_buffer(process, buf, count, |src| {
        process
            .fds
            .lock()
            .write_file_at(fd as i32, offset as u64, src)
    })
}

fn sys_sched_yield(_tf: &TrapFrame) -> isize {
    axtask::yield_now();
    0
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
    if let Err(err) = validate_mempolicy_nodemask(process, nodemask, maxnode) {
        return neg_errno(err);
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
    write_default_mempolicy(process, mode, nodemask, maxnode)
}

fn sys_set_mempolicy(process: &UserProcess, mode: usize, nodemask: usize, maxnode: usize) -> isize {
    let _ = mode;
    if let Err(err) = validate_mempolicy_nodemask(process, nodemask, maxnode) {
        return neg_errno(err);
    }
    0
}

fn sys_pipe2(process: &UserProcess, pipefd: usize, flags: usize) -> isize {
    let flags = flags as u32;
    if flags & !general::O_CLOEXEC != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let fd_flags = fd_cloexec_flag(flags & general::O_CLOEXEC != 0);
    let (read_end, write_end) = PipeEndpoint::new_pair();
    let fds = {
        let mut table = process.fds.lock();
        let read_fd = match table.insert_with_flags(FdEntry::Pipe(read_end), fd_flags) {
            Ok(fd) => fd,
            Err(err) => return neg_errno(err),
        };
        let write_fd = match table.insert_with_flags(FdEntry::Pipe(write_end), fd_flags) {
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
        if current_unblocked_signal_pending() {
            return neg_errno(LinuxError::EINTR);
        }
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
            return_on_fd_set_write_error!(process, readfds, &ready_read);
            return_on_fd_set_write_error!(process, writefds, &ready_write);
            return_on_fd_set_write_error!(process, exceptfds, &ready_except);
            // In this cooperative single-core environment, a hot readiness loop
            // can otherwise starve the peer process that would consume the event.
            axtask::yield_now();
            return ready as isize;
        }
        if deadline.is_some_and(|ddl| axhal::time::wall_time() >= ddl) {
            axtask::yield_now();
            return_on_fd_set_write_error!(process, readfds, &[0; FD_SET_WORDS]);
            return_on_fd_set_write_error!(process, writefds, &[0; FD_SET_WORDS]);
            return_on_fd_set_write_error!(process, exceptfds, &[0; FD_SET_WORDS]);
            return 0;
        }
        axtask::yield_now();
    }
}

fn sys_writev(process: &UserProcess, fd: usize, iov: usize, iovcnt: usize) -> isize {
    let iov_entries = match read_iovec_entries(process, iov, iovcnt) {
        Ok(iov_entries) => iov_entries,
        Err(err) => return neg_errno(err),
    };
    let mut written = 0isize;
    for entry in iov_entries {
        let len = entry.iov_len as usize;
        if len == 0 {
            continue;
        }
        let src = match read_user_bytes(process, entry.iov_base as usize, len) {
            Ok(bytes) => bytes,
            Err(err) => return if written > 0 { written } else { neg_errno(err) },
        };
        let n = match process.fds.lock().write(fd as i32, &src) {
            Ok(v) => v,
            Err(err) => return if written > 0 { written } else { neg_errno(err) },
        };
        written += n as isize;
        if n < len {
            break;
        }
    }
    written
}

fn sys_readv(process: &UserProcess, fd: usize, iov: usize, iovcnt: usize) -> isize {
    let iov_entries = match read_iovec_entries(process, iov, iovcnt) {
        Ok(iov_entries) => iov_entries,
        Err(err) => return neg_errno(err),
    };
    let mut total = 0isize;
    for entry in iov_entries {
        let len = entry.iov_len as usize;
        if len == 0 {
            continue;
        }
        let base = entry.iov_base as usize;
        if let Err(err) = validate_user_write(process, base, len) {
            return if total > 0 { total } else { neg_errno(err) };
        }
        let mut bytes = match user_io_buffer(len) {
            Ok(bytes) => bytes,
            Err(err) => return if total > 0 { total } else { neg_errno(err) },
        };
        let n = match process.fds.lock().read(fd as i32, &mut bytes) {
            Ok(v) => v,
            Err(err) => return if total > 0 { total } else { neg_errno(err) },
        };
        if n > len {
            return if total > 0 {
                total
            } else {
                neg_errno(LinuxError::EINVAL)
            };
        }
        if let Err(err) = write_user_bytes(process, base, &bytes[..n]) {
            return if total > 0 { total } else { neg_errno(err) };
        }
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
    write_user_bytes(process, buf, &bytes)
        .map_or_else(|err| neg_errno(err), |_| bytes.len() as isize)
}

fn sys_chdir(process: &UserProcess, pathname: usize) -> isize {
    let path = read_cstr_or_return!(process, pathname);
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
    let path = read_cstr_or_return!(process, pathname);
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
    process.fds.lock().close_cloexec();
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
            Err(err) => {
                println!(
                    "clone-failure-diagnostic: err={err:?} flags={flags:#x} clone_flags={clone_flags:#x} exit_signal={exit_signal} child_stack={child_stack:#x} parent_sp={:#x} parent_pc={:#x} clone_vm={} clone_vfork={}",
                    tf.regs.sp,
                    user_pc(tf),
                    clone_flags & general::CLONE_VM as usize != 0,
                    clone_flags & general::CLONE_VFORK as usize != 0,
                );
                return neg_errno(err);
            }
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
        task.init_task_ext(UserTaskExt::new(
            child_process.clone(),
            child_clear_tid,
            inherited_signal_mask,
        ));
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
    task.init_task_ext(UserTaskExt::new(
        process.clone(),
        child_clear_tid,
        inherited_signal_mask,
    ));

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
        return_on_user_write_error!(process, status, &wait_status);
    }
    child_pid as isize
}

fn sys_openat(
    process: &UserProcess,
    dirfd: usize,
    pathname: usize,
    flags: usize,
    mode: usize,
) -> isize {
    let path = read_cstr_or_return!(process, pathname);
    match process.fds.lock().open(
        process,
        dirfd as i32,
        path.as_str(),
        flags as u32,
        mode as u32,
    ) {
        Ok(fd) => fd as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_mkdirat(process: &UserProcess, dirfd: usize, pathname: usize, mode: usize) -> isize {
    let path = read_cstr_or_return!(process, pathname);
    match process
        .fds
        .lock()
        .mkdirat(process, dirfd as i32, path.as_str(), mode as u32)
    {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_unlinkat(process: &UserProcess, dirfd: usize, pathname: usize, flags: usize) -> isize {
    let path = read_cstr_or_return!(process, pathname);
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
    mode: usize,
    _flags: usize,
) -> isize {
    if mode & !ACCESS_MODE_MASK != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let path = read_cstr_or_return!(process, pathname);
    let mut fds = process.fds.lock();
    let (resolved_path, stat) = match fds.path_stat(process, dirfd as i32, path.as_str()) {
        Ok(result) => result,
        Err(err) => return neg_errno(err),
    };
    let uid = process.uid();
    let gid = process.gid();
    let parents_searchable =
        match fds.parent_dirs_searchable(process, resolved_path.as_str(), uid, gid) {
            Ok(searchable) => searchable,
            Err(err) => return neg_errno(err),
        };
    if parents_searchable && access_allowed(&stat, mode, uid, gid) {
        0
    } else {
        neg_errno_code(LINUX_EACCES)
    }
}

fn sys_setuid(process: &UserProcess, uid: usize) -> isize {
    set_single_id(uid, |uid| process.set_uid(uid))
}

fn sys_setgid(process: &UserProcess, gid: usize) -> isize {
    set_single_id(gid, |gid| process.set_gid(gid))
}

fn sys_setreuid(process: &UserProcess, ruid: usize, euid: usize) -> isize {
    set_re_ids(ruid, euid, |ruid, euid, saved| {
        process.set_user_ids(ruid, euid, saved);
    })
}

fn sys_setregid(process: &UserProcess, rgid: usize, egid: usize) -> isize {
    set_re_ids(rgid, egid, |rgid, egid, saved| {
        process.set_group_ids(rgid, egid, saved);
    })
}

fn sys_setresuid(process: &UserProcess, ruid: usize, euid: usize, suid: usize) -> isize {
    set_res_ids(ruid, euid, suid, |ruid, euid, suid| {
        process.set_user_ids(ruid, euid, suid);
    })
}

fn sys_setresgid(process: &UserProcess, rgid: usize, egid: usize, sgid: usize) -> isize {
    set_res_ids(rgid, egid, sgid, |rgid, egid, sgid| {
        process.set_group_ids(rgid, egid, sgid);
    })
}

fn sys_getresuid(process: &UserProcess, ruid: usize, euid: usize, suid: usize) -> isize {
    write_id_triplet(
        process,
        [ruid, euid, suid],
        [process.real_uid(), process.uid(), process.saved_uid()],
    )
}

fn sys_getresgid(process: &UserProcess, rgid: usize, egid: usize, sgid: usize) -> isize {
    write_id_triplet(
        process,
        [rgid, egid, sgid],
        [process.real_gid(), process.gid(), process.saved_gid()],
    )
}

fn sys_setfsuid(process: &UserProcess, uid: usize) -> isize {
    let old = process.uid();
    set_fs_id(old, uid, |uid| {
        process.set_user_ids(None, Some(uid), None);
    })
}

fn sys_setfsgid(process: &UserProcess, gid: usize) -> isize {
    let old = process.gid();
    set_fs_id(old, gid, |gid| {
        process.set_group_ids(None, Some(gid), None);
    })
}

fn sys_getgroups(process: &UserProcess, size: usize, list: usize) -> isize {
    let groups = process.groups();
    if size == 0 {
        return groups.len() as isize;
    }
    if size < groups.len() {
        return neg_errno(LinuxError::EINVAL);
    }
    write_group_list(process, list, &groups)
}

fn sys_setgroups(process: &UserProcess, size: usize, list: usize) -> isize {
    if process.uid() != 0 {
        return neg_errno(LinuxError::EPERM);
    }
    if size > 65_536 {
        return neg_errno(LinuxError::EINVAL);
    }
    let groups = match read_group_list(process, size, list) {
        Ok(groups) => groups,
        Err(err) => return neg_errno(err),
    };
    process.set_groups(groups);
    0
}

fn sys_setpgid(process: &UserProcess, pid: usize, pgid: usize) -> isize {
    let pid = pid as i32;
    let pgid = pgid as i32;
    if pid < 0 || pgid < 0 {
        return neg_errno(LinuxError::EINVAL);
    }

    let current = process.pid();
    let target = if pid == 0 { current } else { pid };
    if target != current {
        return neg_errno(LinuxError::ESRCH);
    }

    let group = if pgid == 0 { target } else { pgid };
    if group <= 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    if group != target {
        return neg_errno(LinuxError::EPERM);
    }

    0
}

fn sys_getpgid(process: &UserProcess, pid: usize) -> isize {
    let pid = pid as i32;
    if pid < 0 {
        return neg_errno(LinuxError::EINVAL);
    }

    let current = process.pid();
    let target = if pid == 0 { current } else { pid };
    if target != current {
        return neg_errno(LinuxError::ESRCH);
    }

    target as isize
}

fn sys_setsid(process: &UserProcess) -> isize {
    process.pid() as isize
}

fn sys_fchmod(process: &UserProcess, fd: usize, mode: usize) -> isize {
    let path = match process.fds.lock().entry(fd as i32) {
        Ok(entry) => fd_entry_path(entry).map(ToString::to_string),
        Err(err) => return neg_errno(err),
    };
    if let Some(path) = path {
        process.set_path_mode(path, mode as u32);
    }
    0
}

fn sys_fchmodat(
    process: &UserProcess,
    dirfd: usize,
    pathname: usize,
    mode: usize,
    flags: usize,
) -> isize {
    let flags = flags as u32;
    let supported_flags = general::AT_SYMLINK_NOFOLLOW | general::AT_EMPTY_PATH;
    if flags & !supported_flags != 0 {
        return neg_errno(LinuxError::EINVAL);
    }

    let path = read_cstr_or_return!(process, pathname);
    let mode = mode as u32;
    if path.is_empty() {
        if flags & general::AT_EMPTY_PATH == 0 {
            return neg_errno(LinuxError::ENOENT);
        }
        if dirfd as i32 == general::AT_FDCWD {
            let cwd = process.cwd();
            return match axfs::api::metadata(cwd.as_str()) {
                Ok(_) => {
                    process.set_path_mode(cwd, mode);
                    0
                }
                Err(err) => neg_errno(LinuxError::from(err)),
            };
        }
        return match process.fds.lock().entry(dirfd as i32) {
            Ok(entry) => {
                if let Some(path) = fd_entry_path(entry) {
                    process.set_path_mode(path.to_string(), mode);
                }
                0
            }
            Err(err) => neg_errno(err),
        };
    }

    let mut fds = process.fds.lock();
    match fds.path_stat(process, dirfd as i32, path.as_str()) {
        Ok((resolved_path, _)) => {
            process.set_path_mode(resolved_path, mode);
            0
        }
        Err(err) => neg_errno(err),
    }
}

fn sys_fchown(process: &UserProcess, fd: usize, owner: usize, group: usize) -> isize {
    let (owner, group) = match chown_ids(owner, group) {
        Ok(ids) => ids,
        Err(err) => return neg_errno(err),
    };
    let (path, st) = match process
        .fds
        .lock()
        .stat_with_recorded_path(process, fd as i32)
    {
        Ok((path, st)) => (path, st),
        Err(err) => return neg_errno(err),
    };
    apply_chown_metadata(process, path, &st, owner, group)
}

fn sys_fchownat(
    process: &UserProcess,
    dirfd: usize,
    pathname: usize,
    owner: usize,
    group: usize,
    flags: usize,
) -> isize {
    let flags = flags as u32;
    let supported_flags = general::AT_SYMLINK_NOFOLLOW | general::AT_EMPTY_PATH;
    if flags & !supported_flags != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let (owner, group) = match chown_ids(owner, group) {
        Ok(ids) => ids,
        Err(err) => return neg_errno(err),
    };
    let path = read_cstr_or_return!(process, pathname);
    let (record_path, st) = if path.is_empty() {
        if flags & general::AT_EMPTY_PATH == 0 {
            return neg_errno(LinuxError::ENOENT);
        }
        if dirfd as i32 == general::AT_FDCWD {
            let cwd = process.cwd();
            let st = match process
                .fds
                .lock()
                .stat_path(process, general::AT_FDCWD, ".")
            {
                Ok(st) => st,
                Err(err) => return neg_errno(err),
            };
            (Some(cwd), st)
        } else {
            match process
                .fds
                .lock()
                .stat_with_recorded_path(process, dirfd as i32)
            {
                Ok((path, st)) => (path, st),
                Err(err) => return neg_errno(err),
            }
        }
    } else {
        let mut fds = process.fds.lock();
        let (resolved_path, st) = match fds.path_stat(process, dirfd as i32, path.as_str()) {
            Ok(result) => result,
            Err(err) => return neg_errno(err),
        };
        (Some(resolved_path), st)
    };
    apply_chown_metadata(process, record_path, &st, owner, group)
}

fn sys_ftruncate(process: &UserProcess, fd: usize, length: usize) -> isize {
    let length = length as isize;
    if length < 0 {
        return neg_errno(LinuxError::EINVAL);
    }
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
    let path = read_cstr_or_return!(process, pathname);
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
    let old_path = read_cstr_or_return!(process, oldpath);
    let new_path = read_cstr_or_return!(process, newpath);
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
    let path = read_cstr_or_return!(process, pathname);
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
    let st = match process
        .fds
        .lock()
        .stat_with_recorded_path(process, fd as i32)
    {
        Ok((_, st)) => st,
        Err(err) => return neg_errno(err),
    };
    write_user_value(process, statbuf, &st)
}

fn sys_statfs(process: &UserProcess, pathname: usize, statfsbuf: usize) -> isize {
    if statfsbuf == 0 {
        return neg_errno(LinuxError::EFAULT);
    }
    let path = read_cstr_or_return!(process, pathname);
    let cwd = process.cwd();
    let Some(abs_path) = normalize_path(cwd.as_str(), path.as_str()) else {
        return neg_errno(LinuxError::EINVAL);
    };
    let st = match process
        .fds
        .lock()
        .statfs_path(process, general::AT_FDCWD, abs_path.as_str())
    {
        Ok(st) => st,
        Err(err) => return neg_errno(err),
    };
    write_user_value(process, statfsbuf, &st)
}

fn sys_fstatfs(process: &UserProcess, fd: usize, statfsbuf: usize) -> isize {
    if statfsbuf == 0 {
        return neg_errno(LinuxError::EFAULT);
    }
    let st = match process.fds.lock().statfs(fd as i32) {
        Ok(st) => st,
        Err(err) => return neg_errno(err),
    };
    write_user_value(process, statfsbuf, &st)
}

fn sys_getdents64(process: &UserProcess, fd: usize, dirp: usize, count: usize) -> isize {
    if let Err(err) = validate_user_write(process, dirp, count) {
        return neg_errno(err);
    }
    let bytes = match process.fds.lock().getdents64(fd as i32, count) {
        Ok(bytes) => bytes,
        Err(err) => return neg_errno(err),
    };
    if let Err(err) = write_user_bytes(process, dirp, &bytes) {
        return neg_errno(err);
    }
    bytes.len() as isize
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

fn socket_entry(process: &UserProcess, fd: usize) -> Result<SocketEntry, LinuxError> {
    let table = process.fds.lock();
    match table.entry(fd as i32)? {
        FdEntry::Socket(socket) => Ok(socket.clone()),
        FdEntry::Path(_) => Err(LinuxError::EBADF),
        _ => Err(LinuxError::ENOTSOCK),
    }
}

fn insert_socket_entry(process: &UserProcess, posix_fd: i32, socktype: i32, flags: i32) -> isize {
    if flags & posix_ctypes::SOCK_NONBLOCK as i32 != 0 {
        let ret = arceos_posix_api::sys_fcntl(
            posix_fd,
            posix_ctypes::F_SETFL as i32,
            posix_ctypes::O_NONBLOCK as usize,
        );
        if ret < 0 {
            let _ = arceos_posix_api::sys_close(posix_fd);
            return neg_errno(posix_errno_from_ret(ret as isize));
        }
    }
    match process.fds.lock().insert_with_flags(
        FdEntry::Socket(SocketEntry::new(posix_fd, socktype)),
        fd_cloexec_flag(flags & posix_ctypes::SOCK_CLOEXEC as i32 != 0),
    ) {
        Ok(fd) => fd as isize,
        Err(err) => {
            let _ = arceos_posix_api::sys_close(posix_fd);
            neg_errno(err)
        }
    }
}

fn insert_local_socket_entry(process: &UserProcess, socktype: i32, flags: i32) -> isize {
    match process.fds.lock().insert_with_flags(
        FdEntry::LocalSocket(LocalSocketEntry::new(socktype, flags)),
        fd_cloexec_flag(flags & posix_ctypes::SOCK_CLOEXEC as i32 != 0),
    ) {
        Ok(fd) => fd as isize,
        Err(err) => neg_errno(err),
    }
}

fn is_local_socket_fd(process: &UserProcess, fd: usize) -> Result<bool, LinuxError> {
    let table = process.fds.lock();
    Ok(matches!(table.entry(fd as i32)?, FdEntry::LocalSocket(_)))
}

fn sys_socket_bridge(
    process: &UserProcess,
    domain: usize,
    socktype: usize,
    protocol: usize,
) -> isize {
    let domain = domain as i32;
    let raw_socktype = socktype as i32;
    let protocol = protocol as i32;
    let flag_mask = (posix_ctypes::SOCK_CLOEXEC | posix_ctypes::SOCK_NONBLOCK) as i32;
    let flags = raw_socktype & flag_mask;
    let base_socktype = raw_socktype & !flag_mask;
    if domain == AF_UNIX_DOMAIN {
        if protocol != 0 {
            return neg_errno_code(LINUX_EPROTONOSUPPORT);
        }
        if base_socktype as u32 != posix_ctypes::SOCK_STREAM
            && base_socktype as u32 != posix_ctypes::SOCK_DGRAM
        {
            return neg_errno_code(LINUX_ESOCKTNOSUPPORT);
        }
        return insert_local_socket_entry(process, base_socktype, flags);
    }
    if domain as u32 != posix_ctypes::AF_INET {
        return neg_errno_code(LINUX_EAFNOSUPPORT);
    }
    if base_socktype as u32 == posix_ctypes::SOCK_STREAM {
        if protocol != 0 && protocol as u32 != posix_ctypes::IPPROTO_TCP {
            return neg_errno_code(LINUX_EPROTONOSUPPORT);
        }
    } else if base_socktype as u32 == posix_ctypes::SOCK_DGRAM {
        if protocol != 0 && protocol as u32 != posix_ctypes::IPPROTO_UDP {
            return neg_errno_code(LINUX_EPROTONOSUPPORT);
        }
    } else {
        return neg_errno_code(LINUX_ESOCKTNOSUPPORT);
    }
    let posix_fd = match posix_ret_i32(arceos_posix_api::sys_socket(
        domain,
        base_socktype,
        protocol,
    )) {
        Ok(fd) => fd,
        Err(err) => return neg_errno(err),
    };
    let ret = insert_socket_entry(process, posix_fd, base_socktype, flags);
    ret
}

fn socket_addr_call<F>(
    process: &UserProcess,
    fd: usize,
    addr: usize,
    addrlen: usize,
    call: F,
) -> isize
where
    F: FnOnce(i32, *const posix_ctypes::sockaddr, posix_ctypes::socklen_t) -> i32,
{
    let socket = socket_entry_or_return!(process, fd);
    let addr_bytes = match read_socket_addr_from_user(process, addr, addrlen) {
        Ok(bytes) => bytes,
        Err(err) => return neg_errno(err),
    };
    match posix_ret_i32(call(
        socket.posix_fd,
        addr_bytes.as_ptr() as *const posix_ctypes::sockaddr,
        addrlen as posix_ctypes::socklen_t,
    )) {
        Ok(_) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_bind_bridge(process: &UserProcess, fd: usize, addr: usize, addrlen: usize) -> isize {
    socket_addr_call(process, fd, addr, addrlen, arceos_posix_api::sys_bind)
}

fn sys_listen_bridge(process: &UserProcess, fd: usize, backlog: usize) -> isize {
    let socket = socket_entry_or_return!(process, fd);
    let ret = match posix_ret_i32(arceos_posix_api::sys_listen(
        socket.posix_fd,
        backlog as i32,
    )) {
        Ok(_) => 0,
        Err(err) => neg_errno(err),
    };
    ret
}

fn sys_accept_bridge(
    process: &UserProcess,
    fd: usize,
    addr: usize,
    addrlen: usize,
    flags: usize,
) -> isize {
    match is_local_socket_fd(process, fd) {
        Ok(true) => return neg_errno(LinuxError::EINVAL),
        Ok(false) => {}
        Err(err) => return neg_errno(err),
    }
    let socket = socket_entry_or_return!(process, fd);
    let flag_mask = (posix_ctypes::SOCK_CLOEXEC | posix_ctypes::SOCK_NONBLOCK) as usize;
    if flags & !flag_mask != 0 {
        return neg_errno(LinuxError::EINVAL);
    }

    let user_addr_requested = !(addr == 0 && addrlen == 0);
    if user_addr_requested && (addr == 0 || addrlen == 0) {
        return neg_errno(LinuxError::EFAULT);
    }

    let mut local_addr: posix_ctypes::sockaddr = unsafe { core::mem::zeroed() };
    let mut local_len = size_of::<posix_ctypes::sockaddr>() as posix_ctypes::socklen_t;

    let new_posix_fd = match posix_ret_i32(unsafe {
        arceos_posix_api::sys_accept(socket.posix_fd, &mut local_addr, &mut local_len)
    }) {
        Ok(fd) => fd,
        Err(err) => return neg_errno(err),
    };

    if user_addr_requested {
        let cleanup = |err| {
            let _ = arceos_posix_api::sys_close(new_posix_fd);
            neg_errno(err)
        };
        if let Err(err) =
            validate_user_write(process, addrlen, size_of::<posix_ctypes::socklen_t>())
        {
            return cleanup(err);
        }
        let len = match read_user_value::<posix_ctypes::socklen_t>(process, addrlen) {
            Ok(len) => len as usize,
            Err(err) => return cleanup(err),
        };
        if let Err(err) = validate_user_write(process, addr, len) {
            return cleanup(err);
        }
        let ret = write_socket_addr_to_user(process, addr, addrlen, len, &local_addr, local_len);
        if ret < 0 {
            let _ = arceos_posix_api::sys_close(new_posix_fd);
            return ret;
        }
    }

    let ret = insert_socket_entry(process, new_posix_fd, socket.socktype, flags as i32);
    ret
}

fn sys_connect_bridge(process: &UserProcess, fd: usize, addr: usize, addrlen: usize) -> isize {
    socket_addr_call(process, fd, addr, addrlen, arceos_posix_api::sys_connect)
}

fn sys_sendto_bridge(
    process: &UserProcess,
    fd: usize,
    buf: usize,
    len: usize,
    flags: usize,
    addr: usize,
    addrlen: usize,
) -> isize {
    let socket = socket_entry_or_return!(process, fd);
    let bytes = match read_socket_data_from_user(process, buf, len) {
        Ok(bytes) => bytes,
        Err(err) => return neg_errno(err),
    };
    let data_ptr = bytes.as_ptr() as *const c_void;
    let ret = if addr == 0 {
        unsafe { arceos_posix_api::sys_send(socket.posix_fd, data_ptr, len, flags as i32) }
    } else {
        let addr_bytes = match read_socket_addr_from_user(process, addr, addrlen) {
            Ok(bytes) => bytes,
            Err(err) => return neg_errno(err),
        };
        unsafe {
            arceos_posix_api::sys_sendto(
                socket.posix_fd,
                data_ptr,
                len,
                flags as i32,
                addr_bytes.as_ptr() as *const posix_ctypes::sockaddr,
                addrlen as posix_ctypes::socklen_t,
            )
        }
    };
    let ret = match posix_ret_usize(ret) {
        Ok(n) => n as isize,
        Err(err) => neg_errno(err),
    };
    ret
}

fn sys_recvfrom_bridge(
    process: &UserProcess,
    fd: usize,
    buf: usize,
    len: usize,
    flags: usize,
    addr: usize,
    addrlen: usize,
) -> isize {
    let socket = socket_entry_or_return!(process, fd);
    let ret = if addr == 0 || addrlen == 0 {
        recv_socket_data_to_user(process, socket.posix_fd, buf, len, flags as i32)
    } else {
        if let Err(err) =
            validate_user_write(process, addrlen, size_of::<posix_ctypes::socklen_t>())
        {
            return neg_errno(err);
        }
        let addr_len_value = match read_user_value::<posix_ctypes::socklen_t>(process, addrlen) {
            Ok(len) => len as usize,
            Err(err) => return neg_errno(err),
        };
        if let Err(err) = validate_user_write(process, addr, addr_len_value) {
            return neg_errno(err);
        }
        recv_socket_data_to_user_with_addr(
            process,
            socket.posix_fd,
            buf,
            len,
            flags as i32,
            addr,
            addrlen,
            addr_len_value,
        )
    };
    ret
}

fn sys_shutdown_bridge(process: &UserProcess, fd: usize, how: usize) -> isize {
    let socket = socket_entry_or_return!(process, fd);
    match posix_ret_i32(arceos_posix_api::sys_shutdown(socket.posix_fd, how as i32)) {
        Ok(_) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_getsockname_bridge(process: &UserProcess, fd: usize, addr: usize, addrlen: usize) -> isize {
    socket_name_bridge(
        process,
        fd,
        addr,
        addrlen,
        arceos_posix_api::sys_getsockname,
    )
}

fn sys_getpeername_bridge(process: &UserProcess, fd: usize, addr: usize, addrlen: usize) -> isize {
    socket_name_bridge(
        process,
        fd,
        addr,
        addrlen,
        arceos_posix_api::sys_getpeername,
    )
}

type SocketNameOp =
    unsafe fn(i32, *mut posix_ctypes::sockaddr, *mut posix_ctypes::socklen_t) -> i32;

fn socket_name_bridge(
    process: &UserProcess,
    fd: usize,
    addr: usize,
    addrlen: usize,
    op: SocketNameOp,
) -> isize {
    let socket = socket_entry_or_return!(process, fd);
    if let Err(err) = validate_user_write(process, addrlen, size_of::<posix_ctypes::socklen_t>()) {
        return neg_errno(err);
    }
    let len = match read_user_value::<posix_ctypes::socklen_t>(process, addrlen) {
        Ok(len) => len as usize,
        Err(err) => return neg_errno(err),
    };
    if addr == 0 {
        return neg_errno(LinuxError::EFAULT);
    }
    if let Err(err) = validate_user_write(process, addr, len) {
        return neg_errno(err);
    }
    let mut local_addr: posix_ctypes::sockaddr = unsafe { core::mem::zeroed() };
    let mut local_len = len as posix_ctypes::socklen_t;
    match posix_ret_i32(unsafe { op(socket.posix_fd, &mut local_addr, &mut local_len) }) {
        Ok(_) => write_socket_addr_to_user(process, addr, addrlen, len, &local_addr, local_len),
        Err(err) => neg_errno(err),
    }
}

fn socket_option_supported(level: i32, optname: i32) -> bool {
    if level == SOL_SOCKET_LEVEL {
        matches!(
            optname,
            SO_REUSEADDR_OPT
                | SO_REUSEPORT_OPT
                | SO_DONTROUTE_OPT
                | SO_BROADCAST_OPT
                | SO_KEEPALIVE_OPT
                | SO_SNDBUF_OPT
                | SO_RCVBUF_OPT
                | SO_RCVTIMEO_OPT
                | SO_SNDTIMEO_OPT
                | SO_ERROR_OPT
                | SO_TYPE_OPT
        )
    } else if level == IPPROTO_IP_LEVEL {
        matches!(
            optname,
            IP_RECVERR_OPT | MCAST_JOIN_GROUP_OPT | MCAST_LEAVE_GROUP_OPT
        )
    } else if level == posix_ctypes::IPPROTO_TCP as i32 {
        matches!(optname, TCP_NODELAY_OPT | TCP_MAXSEG_OPT)
    } else {
        false
    }
}

fn current_real_itimer(process: &UserProcess) -> general::itimerval {
    let deadline = process.real_timer_deadline_us.load(Ordering::Acquire);
    let remaining = if deadline == 0 {
        0
    } else {
        deadline.saturating_sub(monotonic_time_micros())
    };
    general::itimerval {
        it_interval: micros_to_timeval(process.real_timer_interval_us.load(Ordering::Acquire)),
        it_value: micros_to_timeval(remaining),
    }
}

fn arm_real_itimer(
    process: Arc<UserProcess>,
    generation: u64,
    first_delay_us: u64,
    interval_us: u64,
) {
    let _ = axtask::spawn(move || {
        let mut delay_us = first_delay_us;
        loop {
            if delay_us == 0 {
                axtask::yield_now();
            } else {
                axtask::sleep(micros_to_duration(delay_us));
            }
            if process.real_timer_generation.load(Ordering::Acquire) != generation
                || process.live_threads.load(Ordering::Acquire) == 0
            {
                break;
            }
            if let Some(entry) = user_thread_entry_for_process(&process) {
                let _ = deliver_user_signal(&entry, SIGALRM_NUM);
            }
            if interval_us == 0 {
                if process.real_timer_generation.load(Ordering::Acquire) == generation {
                    process.real_timer_deadline_us.store(0, Ordering::Release);
                }
                break;
            }
            process.real_timer_deadline_us.store(
                monotonic_time_micros().saturating_add(interval_us),
                Ordering::Release,
            );
            delay_us = interval_us;
        }
    });
}

fn recv_with_real_timer_interrupt<F>(
    process: &UserProcess,
    posix_fd: i32,
    mut recv_once: F,
) -> isize
where
    F: FnMut() -> isize,
{
    let original_timeout = match arceos_posix_api::socket_recv_timeout(posix_fd) {
        Ok(timeout) => timeout,
        Err(err) => return neg_errno(err),
    };
    if original_timeout.is_some() || !process.real_timer_active() {
        return match posix_ret_usize(recv_once()) {
            Ok(n) => n as isize,
            Err(err) => neg_errno(err),
        };
    }

    if let Err(err) =
        arceos_posix_api::set_socket_recv_timeout(posix_fd, Some(INTERRUPTIBLE_SOCKET_RECV_QUANTUM))
    {
        return neg_errno(err);
    }

    let result = loop {
        if current_unblocked_signal_pending() {
            break neg_errno(LinuxError::EINTR);
        }
        match posix_ret_usize(recv_once()) {
            Ok(n) => break n as isize,
            Err(LinuxError::EAGAIN) => {
                if current_unblocked_signal_pending() {
                    break neg_errno(LinuxError::EINTR);
                }
                if !process.real_timer_active() {
                    break neg_errno(LinuxError::EAGAIN);
                }
            }
            Err(err) => break neg_errno(err),
        }
    };

    match arceos_posix_api::set_socket_recv_timeout(posix_fd, original_timeout) {
        Ok(()) => result,
        Err(err) if result >= 0 => neg_errno(err),
        Err(_) => result,
    }
}

fn sys_setsockopt_bridge(
    process: &UserProcess,
    fd: usize,
    level: usize,
    optname: usize,
    optval: usize,
    optlen: usize,
) -> isize {
    let socket = socket_entry_or_return!(process, fd);
    if optlen > 0 {
        if let Err(err) = validate_user_read(process, optval, optlen) {
            return neg_errno(err);
        }
    }
    let level_i32 = level as i32;
    let optname_i32 = optname as i32;
    let ret = if level_i32 == IPPROTO_IP_LEVEL
        && matches!(optname_i32, MCAST_JOIN_GROUP_OPT | MCAST_LEAVE_GROUP_OPT)
    {
        if optval == 0 || optlen < size_of::<u32>() {
            neg_errno(LinuxError::EINVAL)
        } else {
            let mut table = process.fds.lock();
            match table.entry_mut(fd as i32) {
                Ok(FdEntry::Socket(socket)) => {
                    let mut options = socket.options.lock();
                    if optname_i32 == MCAST_JOIN_GROUP_OPT {
                        options.ip_mcast_joined = true;
                        0
                    } else if options.ip_mcast_joined {
                        options.ip_mcast_joined = false;
                        0
                    } else {
                        neg_errno(LinuxError::EADDRNOTAVAIL)
                    }
                }
                Ok(_) => neg_errno(LinuxError::ENOTSOCK),
                Err(err) => neg_errno(err),
            }
        }
    } else if !socket_option_supported(level_i32, optname_i32) {
        neg_errno(LinuxError::EINVAL)
    } else if level_i32 == SOL_SOCKET_LEVEL
        && matches!(optname_i32, SO_RCVTIMEO_OPT | SO_SNDTIMEO_OPT)
    {
        if optlen < size_of::<general::timeval>() {
            neg_errno(LinuxError::EINVAL)
        } else {
            match read_user_value::<general::timeval>(process, optval)
                .and_then(socket_timeval_to_duration)
            {
                Ok(timeout) => {
                    let result = if optname_i32 == SO_RCVTIMEO_OPT {
                        arceos_posix_api::set_socket_recv_timeout(socket.posix_fd, timeout)
                    } else {
                        arceos_posix_api::set_socket_send_timeout(socket.posix_fd, timeout)
                    };
                    match result {
                        Ok(()) => 0,
                        Err(err) => neg_errno(err),
                    }
                }
                Err(err) => neg_errno(err),
            }
        }
    } else {
        0
    };
    ret
}

fn sys_getsockopt_bridge(
    process: &UserProcess,
    fd: usize,
    level: usize,
    optname: usize,
    optval: usize,
    optlen: usize,
) -> isize {
    let socket = socket_entry_or_return!(process, fd);
    if optval == 0 || optlen == 0 {
        return neg_errno(LinuxError::EFAULT);
    }
    let len = match read_user_value::<posix_ctypes::socklen_t>(process, optlen) {
        Ok(len) => len as usize,
        Err(err) => return neg_errno(err),
    };
    let level = level as i32;
    let optname = optname as i32;
    if level == posix_ctypes::IPPROTO_TCP as i32 && optname == TCP_INFO_OPT {
        if len == 0 {
            return neg_errno(LinuxError::EINVAL);
        }
        let out_len = len.min(TCP_INFO_COMPAT_SIZE);
        if let Err(err) = clear_user_bytes(process, optval, out_len) {
            return neg_errno(err);
        }
        let out_len = out_len as posix_ctypes::socklen_t;
        return write_user_value(process, optlen, &out_len);
    }
    if level == SOL_SOCKET_LEVEL && matches!(optname, SO_RCVTIMEO_OPT | SO_SNDTIMEO_OPT) {
        if len < size_of::<general::timeval>() {
            return neg_errno(LinuxError::EINVAL);
        }
        if let Err(err) = validate_user_write(process, optval, size_of::<general::timeval>()) {
            return neg_errno(err);
        }
        let timeout = if optname == SO_RCVTIMEO_OPT {
            arceos_posix_api::socket_recv_timeout(socket.posix_fd)
        } else {
            arceos_posix_api::socket_send_timeout(socket.posix_fd)
        };
        let value = match timeout {
            Ok(timeout) => socket_duration_to_timeval(timeout),
            Err(err) => return neg_errno(err),
        };
        return_on_user_write_error!(process, optval, &value);
        let out_len = size_of::<general::timeval>() as posix_ctypes::socklen_t;
        return write_user_value(process, optlen, &out_len);
    }
    if len < size_of::<i32>() {
        return neg_errno(LinuxError::EINVAL);
    }
    if let Err(err) = validate_user_write(process, optval, size_of::<i32>()) {
        return neg_errno(err);
    }
    let value = if level == SOL_SOCKET_LEVEL {
        match optname {
            SO_ERROR_OPT => 0,
            SO_TYPE_OPT => socket.socktype,
            SO_SNDBUF_OPT | SO_RCVBUF_OPT => DEFAULT_SOCKET_BUFFER_SIZE,
            _ if socket_option_supported(level, optname) => 0,
            _ => return neg_errno(LinuxError::EINVAL),
        }
    } else if level == posix_ctypes::IPPROTO_TCP as i32 && socket_option_supported(level, optname) {
        match optname {
            TCP_MAXSEG_OPT => DEFAULT_TCP_MAXSEG,
            _ => 0,
        }
    } else if level == IPPROTO_IP_LEVEL && socket_option_supported(level, optname) {
        0
    } else {
        return neg_errno(LinuxError::EINVAL);
    };
    return_on_user_write_error!(process, optval, &value);
    let out_len = size_of::<i32>() as posix_ctypes::socklen_t;
    write_user_value(process, optlen, &out_len)
}

fn sys_getrandom(process: &UserProcess, buf: usize, len: usize, flags: usize) -> isize {
    const GRND_NONBLOCK: usize = 0x0001;
    const GRND_RANDOM: usize = 0x0002;
    const GRND_INSECURE: usize = 0x0004;
    if flags & !(GRND_NONBLOCK | GRND_RANDOM | GRND_INSECURE) != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    if let Err(err) = validate_user_write(process, buf, len) {
        return neg_errno(err);
    }

    let mut opts = OpenOptions::new();
    opts.read(true);
    let mut file = match File::open("/dev/urandom", &opts) {
        Ok(file) => file,
        Err(err) => return neg_errno(LinuxError::from(err)),
    };

    let mut filled = 0usize;
    let mut chunk = [0u8; 256];
    while filled < len {
        let chunk_len = (len - filled).min(chunk.len());
        let n = match file.read(&mut chunk[..chunk_len]) {
            Ok(n) => n,
            Err(err) => return neg_errno(LinuxError::from(err)),
        };
        if n == 0 {
            break;
        }
        let dst = match buf.checked_add(filled) {
            Some(dst) => dst,
            None => return neg_errno(LinuxError::EFAULT),
        };
        if let Err(err) = write_user_bytes(process, dst, &chunk[..n]) {
            return neg_errno(err);
        }
        filled += n;
    }
    filled as isize
}

fn sys_readlinkat(
    process: &UserProcess,
    dirfd: usize,
    pathname: usize,
    buf: usize,
    bufsiz: usize,
) -> isize {
    if bufsiz == 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let path = read_cstr_or_return!(process, pathname);
    let resolved_path = {
        let table = process.fds.lock();
        match resolve_dirfd_path(process, &table, dirfd as i32, path.as_str()) {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        }
    };
    if let Some(target) = proc_exe_link_target(process, resolved_path.as_str()) {
        let bytes = target.as_bytes();
        let copy_len = cmp::min(bytes.len(), bufsiz);
        return write_user_bytes(process, buf, &bytes[..copy_len])
            .map_or_else(|err| neg_errno(err), |_| copy_len as isize);
    }
    match axfs::api::metadata(resolved_path.as_str()) {
        Ok(_) => neg_errno(LinuxError::EINVAL),
        Err(err) => neg_errno(LinuxError::from(err)),
    }
}

fn sys_fsync(process: &UserProcess, fd: usize) -> isize {
    match process.fds.lock().entry(fd as i32) {
        Ok(_) => 0,
        Err(err) => neg_errno(err),
    }
}

fn sys_fchdir(process: &UserProcess, fd: usize) -> isize {
    let new_cwd = {
        let table = process.fds.lock();
        match table.entry(fd as i32) {
            Ok(FdEntry::Directory(dir)) => dir.path.clone(),
            Ok(_) => return neg_errno(LinuxError::ENOTDIR),
            Err(err) => return neg_errno(err),
        }
    };
    process.set_cwd(new_cwd);
    0
}

fn sys_ioctl(process: &UserProcess, fd: usize, req: usize, arg: usize) -> isize {
    if req as u32 == RTC_RD_TIME && process.fds.lock().is_rtc(fd as i32) {
        let rtc = rtc_time_from_wall_time();
        return write_user_value(process, arg, &rtc);
    }
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
    let ts = timespec_from_duration(now);
    write_user_value(process, tp, &ts)
}

fn sys_clock_settime(process: &UserProcess, clk_id: usize, tp: usize) -> isize {
    if clk_id != general::CLOCK_REALTIME as usize {
        return neg_errno(LinuxError::EINVAL);
    }
    let ts = match read_user_value::<general::timespec>(process, tp) {
        Ok(ts) => ts,
        Err(err) => return neg_errno(err),
    };
    if ts.tv_sec < 0 || !(0..1_000_000_000).contains(&ts.tv_nsec) {
        return neg_errno(LinuxError::EINVAL);
    }
    set_realtime_offset_from_timespec(ts);
    0
}

fn sys_clock_getres(process: &UserProcess, clk_id: usize, tp: usize) -> isize {
    if let Err(err) = validate_clock_id(clk_id as u32) {
        return neg_errno(err);
    }
    if tp == 0 {
        return 0;
    }
    let ts = clock_resolution_timespec();
    write_user_value(process, tp, &ts)
}

fn sys_gettimeofday(process: &UserProcess, tv: usize, tz: usize) -> isize {
    if tv != 0 {
        let value = current_timeval();
        return_on_user_write_error!(process, tv, &value);
    }
    if tz != 0 {
        let value = zero_timezone();
        return_on_user_write_error!(process, tz, &value);
    }
    0
}

fn sys_adjtimex(process: &UserProcess, tx: usize) -> isize {
    const TIME_OK: isize = 0;

    let input = match read_user_value::<UserTimex>(process, tx) {
        Ok(input) => input,
        Err(err) => return neg_errno(err),
    };
    if !adjtimex_input_valid(input) {
        return neg_errno(LinuxError::EINVAL);
    }
    if adjtimex_changes_clock(input) && process.uid() != 0 {
        return neg_errno(LinuxError::EPERM);
    }

    let output = default_timex();
    let ret = write_user_value(process, tx, &output);
    if ret != 0 {
        return ret;
    }
    TIME_OK
}

fn sys_setitimer(
    process: &Arc<UserProcess>,
    which: i32,
    new_value: usize,
    old_value: usize,
) -> isize {
    if which != general::ITIMER_REAL as i32 {
        return neg_errno(LinuxError::EINVAL);
    }
    if old_value != 0 {
        let value = current_real_itimer(process);
        return_on_user_write_error!(process, old_value, &value);
    }

    let new_timer = if new_value == 0 {
        None
    } else {
        match read_user_value::<general::itimerval>(process, new_value) {
            Ok(value) => Some(value),
            Err(_) => return neg_errno(LinuxError::EFAULT),
        }
    };
    let (first_us, interval_us) = match new_timer {
        Some(value) => match itimerval_to_micros_pair(value) {
            Ok(pair) => pair,
            Err(err) => return neg_errno(err),
        },
        None => (0, 0),
    };

    let generation = process.real_timer_generation.fetch_add(1, Ordering::AcqRel) + 1;
    process
        .real_timer_interval_us
        .store(interval_us, Ordering::Release);
    if first_us == 0 {
        process.real_timer_deadline_us.store(0, Ordering::Release);
    } else {
        process.real_timer_deadline_us.store(
            monotonic_time_micros().saturating_add(first_us),
            Ordering::Release,
        );
        arm_real_itimer(process.clone(), generation, first_us, interval_us);
    }
    0
}

fn sys_times(process: &UserProcess, buf: usize) -> isize {
    let tms = default_tms();
    return_on_user_write_error!(process, buf, &tms);
    axhal::time::monotonic_time().as_millis() as isize
}

fn is_same_sched_target(process: &UserProcess, pid: i32) -> bool {
    pid == 0 || pid == current_tid() || pid == process.pid()
}

fn sys_sched_setparam(process: &UserProcess, pid: i32, param: usize) -> isize {
    return_errno_if!(!is_same_sched_target(process, pid), LinuxError::ESRCH);
    return_errno_if!(param == 0, LinuxError::EINVAL);
    match read_user_value::<UserSchedParam>(process, param) {
        Ok(value) if sched_param_accepts_setparam(value) => 0,
        Ok(_) => neg_errno(LinuxError::EINVAL),
        Err(err) => neg_errno(err),
    }
}

fn sys_sched_getparam(process: &UserProcess, pid: i32, param: usize) -> isize {
    return_errno_if!(!is_same_sched_target(process, pid), LinuxError::ESRCH);
    return_errno_if!(param == 0, LinuxError::EINVAL);
    let value = default_sched_param();
    write_user_value(process, param, &value)
}

fn sys_sched_setscheduler(process: &UserProcess, pid: i32, policy: i32, param: usize) -> isize {
    return_errno_if!(!is_same_sched_target(process, pid), LinuxError::ESRCH);
    return_errno_if!(param == 0, LinuxError::EINVAL);
    let param = match read_user_value::<UserSchedParam>(process, param) {
        Ok(param) => param,
        Err(err) => return neg_errno(err),
    };
    if sched_param_accepts_policy(policy, param) {
        0
    } else {
        neg_errno(LinuxError::EINVAL)
    }
}

fn sys_sched_getscheduler(process: &UserProcess, pid: i32) -> isize {
    return_errno_if!(!is_same_sched_target(process, pid), LinuxError::ESRCH);
    0
}

fn sys_sched_setaffinity(process: &UserProcess, pid: i32, cpusetsize: usize, mask: usize) -> isize {
    return_errno_if!(!is_same_sched_target(process, pid), LinuxError::ESRCH);
    return_errno_if!(cpusetsize == 0 || mask == 0, LinuxError::EINVAL);
    if let Err(err) = validate_user_read(process, mask, cpusetsize) {
        return neg_errno(err);
    }
    match read_user_value::<u8>(process, mask) {
        Ok(first) if first & 1 != 0 => 0,
        Ok(_) => neg_errno(LinuxError::EINVAL),
        Err(err) => neg_errno(err),
    }
}

fn sys_sched_getaffinity(process: &UserProcess, pid: i32, cpusetsize: usize, mask: usize) -> isize {
    return_errno_if!(!is_same_sched_target(process, pid), LinuxError::ESRCH);
    return_errno_if!(cpusetsize == 0 || mask == 0, LinuxError::EINVAL);
    if let Err(err) = clear_user_bytes(process, mask, cpusetsize) {
        return neg_errno(err);
    }
    if let Err(err) = write_user_bytes(process, mask, &[1]) {
        return neg_errno(err);
    }
    cmp::min(cpusetsize, size_of::<usize>()) as isize
}

fn sys_syslog(process: &UserProcess, log_type: i32, buf: usize, len: usize) -> isize {
    match syslog_action(log_type) {
        SyslogAction::EmptyRead => {
            if len > 0 && buf != 0 {
                if let Err(err) = validate_user_write(process, buf, len) {
                    return neg_errno(err);
                }
                if let Err(err) = write_user_bytes(process, buf, &[0]) {
                    return neg_errno(err);
                }
            }
            0
        }
        SyslogAction::SizeBuffer | SyslogAction::ConsoleControl => 0,
        SyslogAction::Invalid => neg_errno(LinuxError::EINVAL),
    }
}

fn sys_getrusage(process: &UserProcess, who: i32, usage: usize) -> isize {
    match who {
        x if x == general::RUSAGE_SELF as i32
            || x == general::RUSAGE_THREAD as i32
            || x == general::RUSAGE_CHILDREN => {}
        _ => return neg_errno(LinuxError::EINVAL),
    }
    let value = default_rusage();
    write_user_value(process, usage, &value)
}

fn sys_uname(process: &UserProcess, buf: usize) -> isize {
    let uts = default_utsname();
    write_user_value(process, buf, &uts)
}

fn sys_nanosleep(process: &UserProcess, req: usize, rem: usize) -> isize {
    let duration = match read_timespec_duration(process, req) {
        Ok(duration) => duration,
        Err(err) => return neg_errno(err),
    };
    sleep_duration(duration);
    if rem != 0 {
        let zero = zero_timespec();
        return_on_user_write_error!(process, rem, &zero);
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
            sleep_duration(delta);
        }
        return 0;
    }
    sys_nanosleep(process, req, rem)
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

fn sys_shmget(_process: &UserProcess, key: usize, size: usize, shmflg: usize) -> isize {
    match sysv_shm::get_or_create(key, size, shmflg) {
        Ok(shmid) => shmid as isize,
        Err(err) => neg_errno(err),
    }
}

fn sys_shmat(process: &UserProcess, shmid: usize, shmaddr: usize, shmflg: usize) -> isize {
    let shmid = shmid as i32;
    let Some((size, backing_vaddr)) = sysv_shm::lookup(shmid) else {
        return neg_errno(LinuxError::EINVAL);
    };
    let map_flags = if shmflg as i32 & SYSV_SHM_RDONLY != 0 {
        user_mapping_flags(true, false, false)
    } else {
        user_mapping_flags(true, true, false)
    };
    let target = {
        let mut brk = process.brk.lock();
        let start = if shmaddr == 0 {
            let start = align_up(brk.next_mmap, PAGE_SIZE_4K);
            brk.next_mmap = start + size + PAGE_SIZE_4K;
            start
        } else {
            align_down(shmaddr, PAGE_SIZE_4K)
        };
        let Some(end) = start.checked_add(size) else {
            return neg_errno(LinuxError::ENOMEM);
        };
        if start < USER_MMAP_BASE || end >= USER_STACK_TOP - USER_STACK_SIZE {
            return neg_errno(LinuxError::ENOMEM);
        }
        start
    };
    let paddr = virt_to_phys(VirtAddr::from(backing_vaddr));
    let map_result = {
        let mut aspace = process.aspace.lock();
        if shmaddr != 0 {
            let _ = aspace.unmap(VirtAddr::from(target), size);
        }
        aspace.map_linear(VirtAddr::from(target), paddr, size, map_flags)
    };
    if let Err(err) = map_result {
        return neg_errno(LinuxError::from(err));
    }
    process.shm_attachments.lock().insert(target, (shmid, size));
    target as isize
}

fn sys_shmdt(process: &UserProcess, tf: &TrapFrame, shmaddr: usize) -> isize {
    let Some((_shmid, size)) = process.shm_attachments.lock().remove(&shmaddr) else {
        return neg_errno(LinuxError::EINVAL);
    };
    sys_munmap(process, tf, shmaddr, size)
}

fn sys_shmctl(process: &UserProcess, shmid: usize, cmd: usize, buf: usize) -> isize {
    let shmid = shmid as i32;
    let cmd = cmd as i32;
    if !sysv_shm::contains(shmid) {
        return neg_errno(LinuxError::EINVAL);
    }
    match cmd {
        SYSV_IPC_RMID => {
            sysv_shm::remove(shmid);
            0
        }
        SYSV_IPC_STAT => {
            if buf != 0 {
                if let Err(err) = clear_user_bytes(process, buf, size_of::<usize>() * 16) {
                    return neg_errno(err);
                }
            }
            0
        }
        SYSV_IPC_SET => 0,
        _ => neg_errno(LinuxError::EINVAL),
    }
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
    let anonymous = flags as u32 & general::MAP_ANONYMOUS != 0;
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
    if anonymous && size <= 0x40000 {
        user_trace!("user-mmap: target={target:#x} len={size:#x} prot={prot:#x} flags={flags:#x}");
    }
    let populate = !anonymous;
    {
        let mut aspace = process.aspace.lock();
        if map_fixed {
            let _ = aspace.unmap(VirtAddr::from(target), size);
        }
        if let Err(err) = aspace.map_alloc(VirtAddr::from(target), size, map_flags, populate) {
            return neg_errno(LinuxError::from(err));
        }
    }

    if !anonymous {
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

fn sys_personality(process: &UserProcess, persona: usize) -> isize {
    let old = process.personality();
    if persona != LINUX_PERSONALITY_QUERY {
        process.set_personality(persona);
    }
    old as isize
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
    return_on_user_write_error!(process, head_ptr, &head);
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
            let state = futex::state(uaddr);
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
        general::FUTEX_WAKE => futex::wake_addr(uaddr, val) as isize,
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
        return_on_user_write_error!(process, oldact, &old);
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
        if let Err(err) = clear_user_bytes(process, oldset, sigsetsize) {
            return neg_errno(err);
        }
        if sigsetsize >= KERNEL_SIGSET_BYTES {
            if let Err(err) = write_user_bytes(process, oldset, &current_mask.to_ne_bytes()) {
                return neg_errno(err);
            }
        }
    }
    if set != 0 {
        let src = match read_user_bytes(process, set, KERNEL_SIGSET_BYTES) {
            Ok(src) => src,
            Err(err) => return neg_errno(err),
        };
        let mut set_bytes = [0u8; KERNEL_SIGSET_BYTES];
        set_bytes.copy_from_slice(&src);
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
        if let Err(err) = clear_user_bytes(process, info, 128) {
            return neg_errno(err);
        }
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
    if pid == 0 {
        return 0;
    }
    if pid == process.pid() || pid == current_tid() {
        let Some(entry) = user_thread_entry_for_process(process) else {
            return neg_errno(LinuxError::ESRCH);
        };
        return deliver_user_signal_result(&entry, sig);
    }
    let Some(entry) = process
        .child_thread_entry_by_pid(pid)
        .or_else(|| user_thread_entry_by_process_pid(pid))
    else {
        return neg_errno(LinuxError::ESRCH);
    };
    deliver_user_signal_result(&entry, sig)
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
    deliver_user_signal_result(&entry, sig)
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
    deliver_user_signal_result(&entry, sig)
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
        return_on_user_write_error!(process, old_limit, &current);
    }

    if new_limit != 0 {
        let limit = match read_user_value::<UserRlimit>(process, new_limit) {
            Ok(limit) => limit,
            Err(err) => return neg_errno(err),
        };
        if !rlimit_is_valid(limit) {
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

impl FdTable {
    fn new() -> Self {
        Self {
            entries: vec![
                Some(FdEntry::Stdin),
                Some(FdEntry::Stdout),
                Some(FdEntry::Stderr),
            ],
            fd_flags: vec![0, 0, 0],
        }
    }

    fn fork_copy(&self) -> Result<Self, LinuxError> {
        let mut entries = Vec::with_capacity(self.entries.len());
        let mut fd_flags = Vec::with_capacity(self.entries.len());
        for (idx, entry) in self.entries.iter().enumerate() {
            entries.push(match entry {
                Some(entry) => Some(entry.duplicate_for_fork()?),
                None => None,
            });
            fd_flags.push(if entry.is_some() {
                self.fd_flags.get(idx).copied().unwrap_or(0)
            } else {
                0
            });
        }
        Ok(Self { entries, fd_flags })
    }

    fn is_stdio(&self, fd: i32) -> bool {
        matches!(fd, 0..=2)
    }

    fn is_rtc(&self, fd: i32) -> bool {
        matches!(self.entry(fd), Ok(FdEntry::Rtc))
    }

    fn poll(&self, fd: i32, mode: SelectMode) -> bool {
        let Ok(entry) = self.entry(fd) else {
            return matches!(mode, SelectMode::Except);
        };
        match mode {
            SelectMode::Read => match entry {
                FdEntry::Stdin => false,
                FdEntry::Stdout | FdEntry::Stderr => false,
                FdEntry::DevNull
                | FdEntry::Rtc
                | FdEntry::File(_)
                | FdEntry::Directory(_)
                | FdEntry::MemoryFile(_) => true,
                FdEntry::Path(_) => false,
                FdEntry::Pipe(pipe) => pipe.poll().readable,
                FdEntry::Socket(socket) => socket.poll(mode),
                FdEntry::LocalSocket(socket) => socket.poll(mode),
            },
            SelectMode::Write => match entry {
                FdEntry::Stdin => false,
                FdEntry::Stdout | FdEntry::Stderr | FdEntry::DevNull | FdEntry::Rtc => true,
                FdEntry::File(_) => true,
                FdEntry::Directory(_) | FdEntry::Path(_) | FdEntry::MemoryFile(_) => false,
                FdEntry::Pipe(pipe) => pipe.poll().writable,
                FdEntry::Socket(socket) => socket.poll(mode),
                FdEntry::LocalSocket(socket) => socket.poll(mode),
            },
            SelectMode::Except => false,
        }
    }

    fn read(&mut self, fd: i32, dst: &mut [u8]) -> Result<usize, LinuxError> {
        match self.entry_mut(fd)? {
            FdEntry::Stdin => Ok(0),
            FdEntry::DevNull => Ok(0),
            FdEntry::Rtc => Ok(0),
            FdEntry::File(file) => file.file.read(dst).map_err(LinuxError::from),
            FdEntry::MemoryFile(file) => Ok(file.read(dst)),
            FdEntry::Directory(_) => Err(LinuxError::EISDIR),
            FdEntry::Pipe(pipe) => pipe.read(dst),
            FdEntry::Socket(socket) => socket.read(dst),
            FdEntry::LocalSocket(socket) => socket.read(dst),
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
            FdEntry::Rtc => Ok(src.len()),
            FdEntry::File(file) => file.file.write(src).map_err(LinuxError::from),
            FdEntry::Pipe(pipe) => pipe.write(src),
            FdEntry::Socket(socket) => socket.write(src),
            FdEntry::LocalSocket(socket) => socket.write(src),
            _ => Err(LinuxError::EBADF),
        }
    }

    fn write_file_at(&mut self, fd: i32, offset: u64, src: &[u8]) -> Result<usize, LinuxError> {
        let FdEntry::File(file) = self.entry_mut(fd)? else {
            return Err(LinuxError::EBADF);
        };
        let mut written = 0usize;
        while written < src.len() {
            let count = file
                .file
                .write_at(offset + written as u64, &src[written..])
                .map_err(LinuxError::from)?;
            if count == 0 {
                break;
            }
            written += count;
        }
        Ok(written)
    }

    fn open(
        &mut self,
        process: &UserProcess,
        dirfd: i32,
        path: &str,
        flags: u32,
        mode: u32,
    ) -> Result<i32, LinuxError> {
        let entry = open_fd_entry(process, self, dirfd, path, flags, mode)?;
        self.insert_with_flags(entry, fd_cloexec_flag(flags & general::O_CLOEXEC != 0))
    }

    fn mkdirat(
        &mut self,
        process: &UserProcess,
        dirfd: i32,
        path: &str,
        mode: u32,
    ) -> Result<(), LinuxError> {
        if path.starts_with('/') || dirfd == general::AT_FDCWD {
            let cwd = process.cwd();
            let abs_path = resolve_host_path(cwd, path).map_err(|_| LinuxError::EINVAL)?;
            directory_create_dir(abs_path.as_str())?;
            process.set_path_mode(abs_path, mode);
            return Ok(());
        }
        let FdEntry::Directory(dir) = self.entry(dirfd)? else {
            return Err(LinuxError::ENOTDIR);
        };
        let abs_path = normalize_path(dir.path.as_str(), path).ok_or(LinuxError::EINVAL)?;
        dir.dir.create_dir(path).map_err(LinuxError::from)?;
        process.set_path_mode(abs_path, mode);
        Ok(())
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

    fn close_slot(&mut self, idx: usize) -> Result<(), LinuxError> {
        if let Some(FdEntry::Socket(socket)) = self.entries[idx].as_ref() {
            socket.close()?;
        }
        self.entries[idx] = None;
        if let Some(flags) = self.fd_flags.get_mut(idx) {
            *flags = 0;
        }
        Ok(())
    }

    fn close(&mut self, fd: i32) -> Result<(), LinuxError> {
        if !(0..self.entries.len() as i32).contains(&fd) || self.entries[fd as usize].is_none() {
            return Err(LinuxError::EBADF);
        }
        self.close_slot(fd as usize)
    }

    fn close_all(&mut self) {
        for idx in 0..self.entries.len() {
            let _ = self.close_slot(idx);
        }
    }

    fn close_cloexec(&mut self) {
        for idx in 0..self.entries.len() {
            if self.fd_flags.get(idx).copied().unwrap_or(0) & general::FD_CLOEXEC == 0 {
                continue;
            }
            let _ = self.close_slot(idx);
        }
    }

    fn stat(&mut self, fd: i32) -> Result<general::stat, LinuxError> {
        match self.entry_mut(fd)? {
            FdEntry::Stdin => Ok(stdio_stat(true)),
            FdEntry::Stdout | FdEntry::Stderr => Ok(stdio_stat(false)),
            FdEntry::DevNull => Ok(stdio_stat(false)),
            FdEntry::Rtc => Ok(stdio_stat(false)),
            FdEntry::File(file) => Ok(file_attr_to_stat(
                &file.file.get_attr().map_err(LinuxError::from)?,
                Some(file.path.as_str()),
            )),
            FdEntry::Directory(dir) => Ok(file_attr_to_stat(&dir.attr, Some(dir.path.as_str()))),
            FdEntry::Path(path) => Ok(path.stat()),
            FdEntry::MemoryFile(file) => Ok(file.stat()),
            FdEntry::Pipe(pipe) => Ok(pipe.stat()),
            FdEntry::Socket(socket) => Ok(socket.stat()),
            FdEntry::LocalSocket(socket) => Ok(socket.stat()),
        }
    }

    fn stat_with_recorded_path(
        &mut self,
        process: &UserProcess,
        fd: i32,
    ) -> Result<(Option<String>, general::stat), LinuxError> {
        let path = fd_entry_path(self.entry(fd)?).map(ToString::to_string);
        let st = self.stat(fd)?;
        let st = match path.as_deref() {
            Some(path) => apply_recorded_path_metadata(process, path, st),
            None => st,
        };
        Ok((path, st))
    }

    fn statfs(&self, fd: i32) -> Result<general::statfs, LinuxError> {
        Ok(generic_statfs(fd_entry_statfs_path(self.entry(fd)?)))
    }

    fn stat_path(
        &mut self,
        process: &UserProcess,
        dirfd: i32,
        path: &str,
    ) -> Result<general::stat, LinuxError> {
        match open_fd_entry(process, self, dirfd, path, general::O_RDONLY, 0) {
            Ok(FdEntry::DevNull) | Ok(FdEntry::Rtc) => Ok(stdio_stat(false)),
            Ok(FdEntry::File(file)) => Ok(apply_recorded_path_metadata(
                process,
                file.path.as_str(),
                file_attr_to_stat(
                    &file.file.get_attr().map_err(LinuxError::from)?,
                    Some(file.path.as_str()),
                ),
            )),
            Ok(FdEntry::Directory(dir)) => Ok(apply_recorded_path_metadata(
                process,
                dir.path.as_str(),
                file_attr_to_stat(&dir.attr, Some(dir.path.as_str())),
            )),
            Ok(FdEntry::Path(path)) => Ok(apply_recorded_path_metadata(
                process,
                path.path.as_str(),
                path.stat(),
            )),
            Ok(FdEntry::MemoryFile(file)) => Ok(apply_recorded_path_metadata(
                process,
                file.path.as_str(),
                file.stat(),
            )),
            Ok(_) => Err(LinuxError::EINVAL),
            Err(err) => Err(err),
        }
    }

    fn path_stat(
        &mut self,
        process: &UserProcess,
        dirfd: i32,
        path: &str,
    ) -> Result<(String, general::stat), LinuxError> {
        let resolved_path = self.resolve_path(process, dirfd, path)?;
        let st = self.stat_path(process, dirfd, path)?;
        Ok((resolved_path, st))
    }

    fn resolve_path(
        &self,
        process: &UserProcess,
        dirfd: i32,
        path: &str,
    ) -> Result<String, LinuxError> {
        if path.is_empty() {
            return Err(LinuxError::ENOENT);
        }
        let normalized = if path.starts_with('/') {
            normalize_path("/", path).ok_or(LinuxError::EINVAL)?
        } else if dirfd == general::AT_FDCWD {
            let cwd = process.cwd();
            normalize_path(cwd.as_str(), path).ok_or(LinuxError::EINVAL)?
        } else {
            let base = match self.entry(dirfd)? {
                FdEntry::Directory(dir) => dir.path.as_str(),
                FdEntry::Path(path_entry) if path_entry.mode & ST_MODE_TYPE_MASK == ST_MODE_DIR => {
                    path_entry.path.as_str()
                }
                _ => return Err(LinuxError::ENOTDIR),
            };
            normalize_path(base, path).ok_or(LinuxError::EINVAL)?
        };
        Ok(canonical_permission_path(normalized))
    }

    fn parent_dirs_searchable(
        &mut self,
        process: &UserProcess,
        path: &str,
        uid: u32,
        gid: u32,
    ) -> Result<bool, LinuxError> {
        if uid == 0 {
            return Ok(true);
        }
        let components: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
        if components.len() <= 1 {
            return Ok(true);
        }
        let mut parent = String::new();
        for component in &components[..components.len() - 1] {
            parent.push('/');
            parent.push_str(component);
            let st = self.stat_path(process, general::AT_FDCWD, parent.as_str())?;
            if !access_allowed(&st, ACCESS_X_OK, uid, gid) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn statfs_path(
        &mut self,
        process: &UserProcess,
        dirfd: i32,
        path: &str,
    ) -> Result<general::statfs, LinuxError> {
        let entry = open_fd_entry(process, self, dirfd, path, general::O_RDONLY, 0)?;
        Ok(generic_statfs(fd_entry_statfs_path(&entry)))
    }

    fn truncate(&mut self, fd: i32, size: u64) -> Result<(), LinuxError> {
        match self.entry_mut(fd)? {
            FdEntry::File(file) => {
                if size > MAX_IN_MEMORY_FILE_SIZE {
                    return Err(LinuxError::ENOSPC);
                }
                file.file.truncate(size).map_err(LinuxError::from)
            }
            FdEntry::DevNull => Ok(()),
            FdEntry::Rtc => Ok(()),
            FdEntry::Path(_) | FdEntry::MemoryFile(_) => Err(LinuxError::EBADF),
            _ => Err(LinuxError::EINVAL),
        }
    }

    fn fcntl(&mut self, fd: i32, cmd: u32, arg: usize) -> Result<i32, LinuxError> {
        if matches!(self.entry(fd)?, FdEntry::Path(_)) && cmd == general::F_GETFL {
            return Ok(O_PATH_FLAG as i32);
        }
        let local_socket = match self.entry(fd)? {
            FdEntry::LocalSocket(socket) => Some(socket.clone()),
            _ => None,
        };
        if let Some(socket) = local_socket {
            return match cmd {
                general::F_DUPFD => {
                    self.insert_min_with_flags(FdEntry::LocalSocket(socket.duplicate()), arg, 0)
                }
                general::F_DUPFD_CLOEXEC => self.insert_min_with_flags(
                    FdEntry::LocalSocket(socket.duplicate()),
                    arg,
                    general::FD_CLOEXEC,
                ),
                general::F_GETFD => self.get_fd_flags(fd),
                general::F_SETFD => self.set_fd_flags(fd, arg as u32),
                general::F_GETFL => Ok(socket.status_flags()),
                general::F_SETFL => Ok(0),
                _ => Ok(0),
            };
        }
        let socket = match self.entry(fd)? {
            FdEntry::Socket(socket) => Some(socket.clone()),
            _ => None,
        };
        if let Some(socket) = socket {
            return match cmd {
                general::F_DUPFD => {
                    self.insert_min_with_flags(FdEntry::Socket(socket.duplicate()?), arg, 0)
                }
                general::F_DUPFD_CLOEXEC => self.insert_min_with_flags(
                    FdEntry::Socket(socket.duplicate()?),
                    arg,
                    general::FD_CLOEXEC,
                ),
                general::F_GETFD => self.get_fd_flags(fd),
                general::F_SETFD => self.set_fd_flags(fd, arg as u32),
                general::F_GETFL | general::F_SETFL => posix_ret_i32(arceos_posix_api::sys_fcntl(
                    socket.posix_fd,
                    cmd as i32,
                    arg,
                )),
                _ => Ok(0),
            };
        }
        match cmd {
            general::F_DUPFD => self.dup_min_with_flags(fd, arg as i32, 0),
            general::F_DUPFD_CLOEXEC => {
                self.dup_min_with_flags(fd, arg as i32, general::FD_CLOEXEC)
            }
            general::F_GETFD => self.get_fd_flags(fd),
            general::F_SETFD => self.set_fd_flags(fd, arg as u32),
            general::F_GETFL | general::F_SETFL => Ok(0),
            _ => Ok(0),
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
            FdEntry::Rtc => Ok(0),
            FdEntry::Directory(_) => Err(LinuxError::EISDIR),
            FdEntry::Path(_) => Err(LinuxError::EBADF),
            FdEntry::MemoryFile(file) => file.seek(pos),
            FdEntry::Pipe(_) => Err(LinuxError::ESPIPE),
            FdEntry::Socket(_) | FdEntry::LocalSocket(_) => Err(LinuxError::ESPIPE),
            _ => Err(LinuxError::ESPIPE),
        }
    }

    fn dup(&mut self, fd: i32) -> Result<i32, LinuxError> {
        self.dup_min(fd, 0)
    }

    fn dup_min(&mut self, fd: i32, min_fd: i32) -> Result<i32, LinuxError> {
        self.dup_min_with_flags(fd, min_fd, 0)
    }

    fn dup_min_with_flags(
        &mut self,
        fd: i32,
        min_fd: i32,
        fd_flags: u32,
    ) -> Result<i32, LinuxError> {
        if min_fd < 0 {
            return Err(LinuxError::EINVAL);
        }
        let entry = self.entry(fd)?.duplicate_for_fork()?;
        self.insert_min_with_flags(entry, min_fd as usize, fd_flags & general::FD_CLOEXEC)
    }

    fn dup3(&mut self, oldfd: i32, newfd: i32, flags: u32) -> Result<i32, LinuxError> {
        if oldfd == newfd {
            return Err(LinuxError::EINVAL);
        }
        if flags & !general::O_CLOEXEC != 0 {
            return Err(LinuxError::EINVAL);
        }
        let entry = self.entry(oldfd)?.duplicate_for_fork()?;
        if newfd < 0 {
            return Err(LinuxError::EBADF);
        }
        let newfd = newfd as usize;
        if self.entries.len() <= newfd {
            self.entries.resize_with(newfd + 1, || None);
            self.fd_flags.resize(newfd + 1, 0);
        } else if self.entries[newfd].is_some() {
            let _ = self.close(newfd as i32);
        }
        if self.fd_flags.len() <= newfd {
            self.fd_flags.resize(newfd + 1, 0);
        }
        self.entries[newfd] = Some(entry);
        self.fd_flags[newfd] = fd_cloexec_flag(flags & general::O_CLOEXEC != 0);
        Ok(newfd as i32)
    }

    fn getdents64(&mut self, fd: i32, max_len: usize) -> Result<Vec<u8>, LinuxError> {
        let entry = self.entry_mut(fd)?;
        let FdEntry::Directory(dir) = entry else {
            return Err(LinuxError::ENOTDIR);
        };
        let mut read_buf: [fops::DirEntry; 16] =
            core::array::from_fn(|_| fops::DirEntry::default());
        let count = dir.dir.read_dir(&mut read_buf).map_err(LinuxError::from)?;
        let mut out = Vec::new();
        for (idx, item) in read_buf[..count].iter().enumerate() {
            let name = item.name_as_bytes();
            let reclen = align_up(
                offset_of!(general::linux_dirent64, d_name) + name.len() + 1,
                8,
            );
            if out.len() + reclen > max_len {
                break;
            }
            let start = out.len();
            out.resize(start + reclen, 0);
            unsafe {
                let dirent = out[start..].as_mut_ptr() as *mut general::linux_dirent64;
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
            }
            let name_start = start + offset_of!(general::linux_dirent64, d_name);
            out[name_start..name_start + name.len()].copy_from_slice(name);
        }
        Ok(out)
    }

    fn read_file_at(&mut self, fd: i32, offset: u64, len: usize) -> Result<Vec<u8>, LinuxError> {
        let FdEntry::File(file) = self.entry_mut(fd)? else {
            return Err(LinuxError::EBADF);
        };
        let mut buf = vec![0u8; len];
        let filled = read_file_at_into(&file.file, offset, &mut buf)?;
        buf.truncate(filled);
        Ok(buf)
    }

    fn insert_with_flags(&mut self, entry: FdEntry, fd_flags: u32) -> Result<i32, LinuxError> {
        self.insert_min_with_flags(entry, 0, fd_flags)
    }

    fn insert_min_with_flags(
        &mut self,
        entry: FdEntry,
        min_fd: usize,
        fd_flags: u32,
    ) -> Result<i32, LinuxError> {
        if self.entries.len() < min_fd {
            self.entries.resize_with(min_fd, || None);
            self.fd_flags.resize(min_fd, 0);
        }
        if self.fd_flags.len() < self.entries.len() {
            self.fd_flags.resize(self.entries.len(), 0);
        }
        if let Some((idx, slot)) = self
            .entries
            .iter_mut()
            .enumerate()
            .skip(min_fd)
            .find(|(_, slot)| slot.is_none())
        {
            *slot = Some(entry);
            self.fd_flags[idx] = fd_flags & general::FD_CLOEXEC;
            return Ok(idx as i32);
        }
        self.entries.push(Some(entry));
        self.fd_flags.push(fd_flags & general::FD_CLOEXEC);
        Ok((self.entries.len() - 1) as i32)
    }

    fn get_fd_flags(&self, fd: i32) -> Result<i32, LinuxError> {
        self.entry(fd)?;
        Ok(self.fd_flags.get(fd as usize).copied().unwrap_or(0) as i32)
    }

    fn set_fd_flags(&mut self, fd: i32, flags: u32) -> Result<i32, LinuxError> {
        self.entry(fd)?;
        let idx = fd as usize;
        if self.fd_flags.len() <= idx {
            self.fd_flags.resize(idx + 1, 0);
        }
        self.fd_flags[idx] = flags & general::FD_CLOEXEC;
        Ok(0)
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
    mode: u32,
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

    let absolute = path.starts_with('/');
    let exec_root = process.exec_root();
    let add_busybox_aliases = busybox_applet_alias_allowed(flags, access);

    if absolute || dirfd == general::AT_FDCWD {
        let mut candidates = if absolute {
            if let Some(path) = dev_shm_host_path(path) {
                ensure_dev_shm_dir()?;
                return open_candidates(process, &[path], &opts, flags, mode);
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
        if add_busybox_aliases {
            append_busybox_applet_alias_candidates(&mut candidates);
        }
        if candidates.is_empty() {
            return Err(LinuxError::EINVAL);
        }
        open_candidates(process, &candidates, &opts, flags, mode)
    } else {
        let FdEntry::Directory(dir) = table.entry(dirfd)? else {
            return Err(LinuxError::ENOTDIR);
        };
        let primary = normalize_path(dir.path.as_str(), path).ok_or(LinuxError::EINVAL)?;
        let mut candidates = vec![primary];
        for extra in runtime_library_name_candidates(exec_root.as_str(), path) {
            push_runtime_candidate(&mut candidates, Some(extra));
        }
        if add_busybox_aliases {
            append_busybox_applet_alias_candidates(&mut candidates);
        }
        open_candidates(process, &candidates, &opts, flags, mode)
    }
}

fn busybox_applet_alias_allowed(flags: u32, access: u32) -> bool {
    access != general::O_WRONLY
        && access != general::O_RDWR
        && flags & (general::O_CREAT | general::O_TRUNC | general::O_APPEND) == 0
}

fn append_busybox_applet_alias_candidates(candidates: &mut Vec<String>) {
    for candidate in candidates.clone() {
        push_runtime_candidate(candidates, busybox_applet_target_path(candidate.as_str()));
    }
}

fn open_candidates(
    process: &UserProcess,
    candidates: &[String],
    opts: &OpenOptions,
    flags: u32,
    mode: u32,
) -> Result<FdEntry, LinuxError> {
    let prefer_dir = flags & general::O_DIRECTORY != 0;
    let path_only = flags & O_PATH_FLAG != 0;
    let mut path_opts = OpenOptions::new();
    if path_only {
        path_opts.read(true);
    }
    let file_opts = if path_only { &path_opts } else { opts };
    let mut last_err = LinuxError::ENOENT;
    for path in candidates {
        if is_proc_self_maps_path(path.as_str()) {
            if prefer_dir {
                return Err(LinuxError::ENOTDIR);
            }
            if !path_only && proc_self_maps_is_writable_open(flags) {
                return Err(LinuxError::EPERM);
            }
            return Ok(if path_only {
                proc_self_maps_path_entry(process)
            } else {
                proc_self_maps_fd_entry(process)
            });
        }
        if let Some((synthetic_path, data)) = synthetic_userdb_content(path.as_str()) {
            if prefer_dir {
                return Err(LinuxError::ENOTDIR);
            }
            if !path_only && synthetic_file_is_writable_open(flags) {
                return Err(LinuxError::EPERM);
            }
            return Ok(if path_only {
                synthetic_userdb_path_entry(synthetic_path, data)
            } else {
                synthetic_userdb_fd_entry(synthetic_path, data)
            });
        }
        if path == "/dev/null" {
            if prefer_dir {
                return Err(LinuxError::ENOTDIR);
            }
            return Ok(if path_only {
                FdEntry::Path(PathEntry::synthetic_char("/dev/null"))
            } else {
                FdEntry::DevNull
            });
        }
        if path == "/dev/misc/rtc" || path == "/dev/rtc" {
            if prefer_dir {
                return Err(LinuxError::ENOTDIR);
            }
            return Ok(if path_only {
                FdEntry::Path(PathEntry::synthetic_char(path.as_str()))
            } else {
                FdEntry::Rtc
            });
        }
        if prefer_dir {
            match open_dir_entry(path.as_str()) {
                Ok(FdEntry::Directory(dir)) if path_only => {
                    return Ok(path_entry_from_directory(dir));
                }
                Ok(entry) if !path_only => return Ok(entry),
                Ok(_) => return Err(LinuxError::EINVAL),
                Err(err) => record_missing_candidate(&mut last_err, err)?,
            }
            continue;
        }
        let created_by_this_open = !path_only
            && flags & general::O_CREAT != 0
            && axfs::api::metadata(path.as_str()).is_err();
        match File::open(path.as_str(), file_opts) {
            Ok(file) if path_only => {
                let attr = file.get_attr().map_err(LinuxError::from)?;
                return Ok(FdEntry::Path(PathEntry::from_attr(path.as_str(), &attr)));
            }
            Ok(file) => {
                if created_by_this_open {
                    process.set_path_mode(path.clone(), mode);
                    process.set_path_owner(path.clone(), Some(process.uid()), Some(process.gid()));
                }
                return Ok(FdEntry::File(FileEntry {
                    file,
                    path: path.clone(),
                }));
            }
            Err(err) => {
                let err = LinuxError::from(err);
                if err == LinuxError::EISDIR {
                    return match open_dir_entry(path.as_str())? {
                        FdEntry::Directory(dir) if path_only => Ok(path_entry_from_directory(dir)),
                        entry if !path_only => Ok(entry),
                        _ => Err(LinuxError::EINVAL),
                    };
                }
                record_missing_candidate(&mut last_err, err)?;
            }
        }
    }
    Err(last_err)
}

fn path_entry_from_directory(dir: DirectoryEntry) -> FdEntry {
    FdEntry::Path(PathEntry::from_attr(dir.path.as_str(), &dir.attr))
}

fn record_missing_candidate(last_err: &mut LinuxError, err: LinuxError) -> Result<(), LinuxError> {
    *last_err = err;
    if err == LinuxError::ENOENT {
        Ok(())
    } else {
        Err(err)
    }
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
