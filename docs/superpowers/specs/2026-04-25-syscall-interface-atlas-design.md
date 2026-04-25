# Syscall Interface Atlas Design

Date: 2026-04-25

Status: historical design record.

Canonical long-lived interface rules now live under `docs/development/`:

- `docs/development/README.md`
- `docs/development/interfaces/`
- `docs/development/policies/`

Use those documents for current development. This file records the original
design discussion that led to the long-lived split.

## Goal

Build a shared implementation map for the ArceOS syscall work that covers the
test groups owned by this track:

- Filesystem and file descriptors: `basic`, `busybox`, `iozone`,
  `UnixBench fstime`, `lmbench` filesystem/file tests, and LTP filesystem
  runtests.
- Memory management: `basic`, `libcbench`, `lmbench` mmap/pagefault tests, and
  LTP memory runtests.
- Process, thread, and scheduling: `basic`, `cyclictest`, `UnixBench`,
  `lmbench`, and LTP sched/nptl runtests.

This document is not a patch plan for one syscall. It is the interface atlas we
should use before adding more behavior, so temporary compatibility code does not
become the architecture.

## Constraints

- Rust build, QEMU, and testsuite commands must run inside the long-lived Docker
  container `arceos-eval-fix`.
- The container already exists and must not be deleted or recreated.
- Start it only when needed with `docker start arceos-eval-fix`, then run
  commands with `docker exec arceos-eval-fix ...`.
- The syscall ABI entry point for the shell test path is
  `examples/shell/src/uspace.rs`.
- RISC-V and LoongArch behavior must be kept aligned when syscall numbers or
  architecture-specific paths are touched.

## Design Rules

1. The syscall layer should translate Linux ABI details into ArceOS internal
   operations. It should not invent a second filesystem, memory manager, or task
   manager.
2. A temporary compatibility path is acceptable only when it is explicitly
   marked, bounded to a milestone, and returns real errors for unsupported
   states. Silent success for unimplemented behavior is not acceptable.
3. If a teammate-owned layer is expected to provide an interface, design the
   syscall side around that real interface and leave a narrow adapter or
   tracked follow-up, not a hardcoded replacement.
4. Prefer returning a Linux-compatible errno over pretending success. This is
   required for LTP and keeps later integration failures visible.
5. File descriptors, address spaces, and task relationships should each have one
   authority. Adapters may translate, but should not fork state in parallel
   unless that state is explicitly compatibility-only.

## Current Layering

### Syscall/Process Layer

`examples/shell/src/uspace.rs` currently contains:

- `UserProcess`: user address space, cwd, heap break, children, fd table, and
  temporary process-local compatibility state.
- `FdTable` and `FdEntry`: regular files, directories, pipes, stdio, and
  `/dev/null`.
- syscall dispatch in `user_syscall`.
- Linux ABI helpers for user memory, C strings, iovec, errno conversion, and
  stat conversion.

This layer is the right place for Linux ABI details such as `AT_FDCWD`,
`O_*` flag translation, `linux_dirent64` layout, `dup3` error rules, and syscall
number differences.

It is not the right long-term place for runtime mount tables, filesystem
metadata synthesis, file-backed page cache policy, or scheduler policy.

### Filesystem Layer

Available ArceOS interfaces:

- `axfs::api`: `metadata`, `read_dir`, `create_dir`, `remove_dir`,
  `remove_file`, `rename`, `current_dir`, and `set_current_dir`.
- `axfs::api::File` and `OpenOptions`: open/create/truncate/append plus
  `Read`, `Write`, `Seek`, `flush`, `set_len`, and `metadata`.
- `axfs::api::ReadDir` and `DirEntry`: directory iteration.
- `axfs::root::RootDirectory`: internal mount table with `mount`, mounted-fs
  lookup, and mount-point directory entries.

Known interface gap:

- Runtime `mount`/`umount` is not exposed as a public API suitable for syscall
  use. `RootDirectory::mount` takes `&'static str`, `_umount` is private, and
  there is no syscall-facing filesystem factory from `(source, target, fstype,
  flags, data)` to `Arc<dyn VfsOps>`.

### Memory Layer

Available ArceOS interfaces:

- `axmm::AddrSpace`: `map_alloc`, `map_linear`, `unmap`, `protect`,
  `find_free_area`, `read`, `write`, `can_access_range`, `handle_page_fault`,
  and user mapping clone support.
- `MemorySet<Backend>` and paging flags behind `AddrSpace`.

Known interface gaps:

- File-backed mmap and `MAP_SHARED` persistence need an explicit backing object
  path. The current syscall layer can allocate anonymous pages, but should not
  fake file-backed semantics.
- `clone_user_mappings_from` copies pages eagerly. That is usable for early
  `fork`, but it is not copy-on-write and will limit process benchmarks.
- `mremap`, `mincore`, `madvise`, `mlock`, and System V shared memory need real
  memory-manager contracts before LTP can be handled cleanly.

### Task/Process Layer

Available ArceOS interfaces:

- `axtask` and `arceos_api` task operations: spawn, exit, wait for exit,
  sleep, yield, current task id, priority, affinity, and wait queues.
- Current shell process logic in `uspace.rs`: child list, wait queue,
  `fork`-style process construction, `execve` image loading, and exit status.

Known interface gaps:

- Linux `clone` flag semantics are much richer than the current process model.
- `vfork`, thread groups, robust futex lists, signal delivery, wait-id variants,
  scheduler classes, and realtime policy need explicit task-layer contracts.
- The current eager address-space copy is acceptable for functional bring-up but
  will be expensive for `UnixBench`, `lmbench`, and shell-heavy workloads.

## Required Interface Contracts

These are the interface shapes the syscall layer should be able to call. The
Rust blocks are example API sketches: crate boundaries, type names, field names,
and exact ownership can change to match the implementation. The required
semantics listed under each sketch are hard constraints.

### Runtime Mount Contract

Provider: `axfs` or an `axfs`-owned crate/module. The syscall layer should not
own the real mount table.

Example API sketch:

```rust
pub struct MountRequest<'a> {
    pub source: &'a str,
    pub target: &'a str,
    pub fstype: &'a str,
    pub flags: MountFlags,
    pub data: Option<&'a [u8]>,
}

pub trait FileSystemFactory: Send + Sync {
    fn mount(&self, request: &MountRequest<'_>) -> AxResult<Arc<dyn VfsOps>>;
    fn supports(&self, fstype: &str) -> bool;
}

pub fn mount(request: MountRequest<'_>) -> AxResult<()>;
pub fn umount(target: &str, flags: UmountFlags) -> AxResult<()>;
pub fn is_mount_point(path: &str) -> bool;
```

Required semantics:

- `target` must resolve to an existing directory in the current namespace.
- duplicate targets return `EBUSY`.
- unknown `fstype` returns `ENODEV` or `EOPNOTSUPP`, not success.
- unsupported `flags` or incompatible `data` return `EINVAL` or `EOPNOTSUPP`.
- unmount of a non-mounted target returns `EINVAL` or `ENOENT`.
- the VFS layer owns mount-point crossing during later path resolution.

### File-Backed Mapping Contract

Provider: memory manager plus filesystem/open-file-description layer. The
syscall layer should construct the request and let the VMA/page-fault path own
semantics.

Example API sketch:

```rust
pub trait FileMapping: Send + Sync {
    fn len(&self) -> AxResult<u64>;
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn read_page(&self, file_offset: u64, dst: &mut [u8]) -> AxResult<usize>;
    fn write_page(&self, file_offset: u64, src: &[u8]) -> AxResult<usize>;
    fn flush_range(&self, file_offset: u64, len: usize) -> AxResult<()>;
}

pub enum VmaKind {
    Heap,
    Anonymous,
    File { mapping: Arc<dyn FileMapping>, offset: u64, shared: bool },
    Stack,
    SharedMemory,
    Guard,
}

pub fn mmap_file(request: FileMmapRequest) -> Result<VirtAddr, LinuxError>;
```

Required semantics:

- file-backed `mmap` stores file identity, mapping offset, length, permissions,
  and shared/private mode in a VMA.
- page fault asks the VMA how to fill the page; `AddrSpace` only installs or
  removes mappings.
- `MAP_SHARED` dirty pages must have a defined flush/writeback path.
- `pread/pwrite` and `mmap` must not depend on the current file offset.
- file lifetime is held by `Arc<dyn FileMapping>` or an equivalent stable open
  file object until the VMA is destroyed.

### Task and Process Contract

Provider: task/process layer above `axtask`. The syscall layer can translate
Linux flags, but process identity and lifecycle should be explicit.

Example API sketch:

```rust
pub struct Process {
    pub pid: Pid,
    pub tgid: Pid,
    pub parent: Option<Pid>,
    pub children: ChildSet,
    pub threads: ThreadGroup,
    pub fd_table: Arc<Mutex<FdTable>>,
    pub memory: Arc<Mutex<MemoryMap>>,
    pub exit_state: ExitState,
}

pub struct Thread {
    pub tid: Tid,
    pub tgid: Pid,
    pub task: AxTaskRef,
    pub clear_child_tid: AtomicUsize,
    pub robust_list: RobustListState,
    pub signal_mask: SignalSet,
}

pub enum ExitState {
    Running,
    Zombie { code: i32 },
    Reaped,
}
```

Required semantics:

- `pid` identifies a process leader; `tid` identifies a task/thread.
- thread-group exit and single-thread exit are distinct states.
- parent/child ownership and zombie reaping are not inferred from raw task ids.
- `clone` decides whether memory, fd table, cwd/fs state, and signal handlers
  are shared or copied from flags.
- `execve` closes `FD_CLOEXEC` fds and replaces memory atomically from the
  caller's point of view.

## Core Authority Models

The structs below are example shapes for discussion and review. The rules under
each model are the required behavior. Implementations may choose different type
names or module boundaries if those rules remain true and the deviation is
recorded in the implementation plan.

### File Descriptor and Open-File-Description Model

This is a Phase 1 prerequisite, not an open-ended future question. Linux file
semantics depend on separating fd slots from open file descriptions.

Example model sketch:

```rust
pub struct FdTable {
    entries: Vec<Option<FdSlot>>,
}

pub struct FdSlot {
    pub fd_flags: FdFlags, // FD_CLOEXEC lives here.
    pub desc: Arc<OpenFileDescription>,
}

pub struct OpenFileDescription {
    pub status_flags: Mutex<OpenStatusFlags>, // O_APPEND, O_NONBLOCK, etc.
    pub offset: Mutex<u64>,
    pub backend: OpenFileBackend,
}

pub enum OpenFileBackend {
    Regular(Arc<dyn FileIo>),
    Directory(Arc<dyn DirectoryIo>),
    Pipe(PipeEndpoint),
    CharDevice(Arc<dyn DeviceIo>),
    DevNull,
}
```

Rules:

- `dup`, `dup2`, and `dup3` create a new `FdSlot` pointing at the same
  `OpenFileDescription`.
- `fork` copies fd slots, but those slots point at the same open file
  descriptions.
- `clone(CLONE_FILES)` shares the whole `FdTable`; without `CLONE_FILES`, it
  copies fd slots.
- `execve` removes only slots with `FD_CLOEXEC`.
- `read`, `write`, `lseek`, and `getdents64` use and update the shared offset
  in `OpenFileDescription`.
- `pread`, `pwrite`, `preadv`, and `pwritev` use their explicit offset and do
  not change the shared offset.
- `O_APPEND` is stored in `OpenFileDescription.status_flags`; writes append
  atomically with respect to that description.

### Path Resolution Model

All path-taking syscalls should use one resolver. Ad hoc path joins in syscall
handlers should be deleted as the resolver becomes available.

Example resolver input:

```rust
pub struct PathResolveRequest<'a> {
    pub dirfd: Option<i32>,       // None means AT_FDCWD.
    pub path: &'a str,
    pub flags: ResolveFlags,     // AT_* plus syscall-specific lookup rules.
    pub must_be_dir: bool,
    pub allow_empty_path: bool,
}
```

Rules:

- a null pathname pointer returns `EFAULT`.
- an empty path returns `ENOENT` unless the syscall explicitly accepts
  `AT_EMPTY_PATH`; then the fd object is used.
- absolute paths start at the process namespace root and ignore `dirfd`.
- relative paths start at process cwd for `AT_FDCWD`, otherwise at the directory
  referenced by `dirfd`; a non-directory `dirfd` returns `ENOTDIR`.
- `.` is ignored; `..` walks to the parent but cannot escape namespace root.
- trailing slash requires the final object to be a directory; otherwise return
  `ENOTDIR`.
- unknown `AT_*` flags return `EINVAL`.
- if symlinks are unsupported by the active filesystem, symlink creation returns
  `ENOSYS` and symlink traversal returns `ELOOP` or `EOPNOTSUPP`; this must be
  consistent per syscall.
- `AT_SYMLINK_NOFOLLOW` is accepted for stat-like calls, but has no observable
  effect until symlink objects exist.
- `AT_EMPTY_PATH` is initially supported only for `fstatat/statx`-style
  metadata queries; other uses return `EINVAL`.
- mount-point crossing is handled by VFS lookup after real runtime mounts
  exist, not by syscall-local path rewriting.

### VMA and Address-Space Model

`AddrSpace` should execute page-table changes. A separate memory map/VMA layer
must own Linux mmap semantics.

Example model sketch:

```rust
pub struct MemoryMap {
    vmas: BTreeMap<VirtAddr, Vma>,
}

pub struct Vma {
    pub start: VirtAddr,
    pub end: VirtAddr,
    pub perms: VmaPerms,
    pub flags: VmaFlags,
    pub kind: VmaKind,
}
```

Rules:

- VMA types are `Heap`, `Anonymous`, `File`, `Stack`, `SharedMemory`, and
  `Guard`.
- `mmap`, `munmap`, `mprotect`, and `mremap` update VMAs first, then ask
  `AddrSpace` to update mappings.
- unmap/protect/remap can split and merge VMAs.
- page-table flags are derived from VMA permissions; they are not the semantic
  source of truth.
- page fault looks up the VMA, checks access, fills anonymous/file/shared pages,
  and then calls `AddrSpace` to install the mapping.
- file-backed mappings store file offset rounded to page boundaries plus the
  intra-page delta required by Linux `mmap`.
- `MAP_PRIVATE` and `fork` should move toward COW; eager copy is only a
  functional bring-up path.

### Concurrency and Lifecycle Model

The order below is a provisional acquisition order for short, non-blocking
critical sections. Blocking operations must first copy or `Arc`-clone the
needed object references, release higher-level locks, and only then sleep,
wait, or perform potentially blocking backend I/O.

Reference acquisition order:

1. process registry/global pid table;
2. `UserProcess` lifecycle state;
3. `FdTable`;
4. open-file-description backend lock;
5. `MemoryMap`;
6. `AddrSpace`;
7. child list state.

Rules:

- Never hold `FdTable` while blocking on pipe/futex/wait queue operations.
- Never hold `UserProcess`, `FdTable`, `MemoryMap`, or `AddrSpace` locks while
  waiting on a wait queue, pipe condition, futex queue, or child exit.
- A syscall that obtains an `Arc<OpenFileDescription>` may continue using it if
  another thread closes the fd slot after lookup.
- `close` removes the fd slot; it does not destroy the open file description
  until the last `Arc` is dropped.
- Page fault handling may consult VMA metadata, but it must not block on
  filesystem I/O while holding unrelated process or fd-table locks.
- `fork` snapshots fd slots and process state under process/fd locks, then
  creates the child task.
- `execve` builds the new memory image first; the switch to new memory/fd
  close-on-exec state is committed in one step.
- `exit` transitions the thread/process to zombie state under lifecycle locks,
  releases those locks, wakes waiters, handles `clear_child_tid`, and only then
  releases task-local resources.
- `wait*` checks child state under the child-list lock, releases it before
  blocking, and observes `Running -> Zombie -> Reaped`; only one waiter can reap
  a child.

## Syscall Atlas

The `nr(rv/la)` column uses the contest generic Linux ABI values from
`testsuits-for-oskernel/basic/user/lib/syscall_ids.h`; RISC-V64 and
LoongArch64 share these values for the tracked syscalls. `mount` and `umount2`
are explicitly forced to 40 and 39 in `uspace.rs` for both architectures.

Maintenance and source rules:

- Any change that adds, removes, renames, or changes behavior of a syscall
  dispatcher arm or `sys_*` handler must update this table in the same commit.
- `Current handler` is sourced from `examples/shell/src/uspace.rs`, especially
  `user_syscall` and the matching `sys_*` function definitions.
- `nr(rv/la)` is sourced from `examples/shell/src/uspace.rs` for explicit
  architecture overrides and from
  `testsuits-for-oskernel/basic/user/lib/syscall_ids.h` for the generic contest
  ABI. If those disagree, record the exception next to the syscall instead of
  silently choosing one.
- `Status` is based on the implementation that exists in code, not the intended
  final design.
- Before using the table as a gate artifact, verify it with `rg` against
  `uspace.rs` and the testsuite syscall header.

Status values:

- `Real-partial`: handler calls a real ArceOS interface but Linux semantics are
  incomplete.
- `Partial`: handler exists, but either the backing contract is not yet the
  right authority or important Linux behavior is still missing.
- `Compat`: handler intentionally emulates only a bounded test need.
- `Compat/partial`: a real handler exists but still contains compatibility
  behavior that must be removed before broad gates.
- `Stub-success`: handler returns success without implementing the operation;
  this must not pass broad-suite gates.
- `Missing`: no handler or no real implementation; expected result is
  `ENOSYS`, `EINVAL`, or `EOPNOTSUPP` depending on syscall contract.

| Syscall | nr(rv/la) | Current handler | Status | Required by workload | Errno fidelity | Owner | Next action |
| --- | ---: | --- | --- | --- | --- | --- | --- |
| `getcwd` | 17 | `sys_getcwd` | Real-partial | basic, busybox, LTP | medium | syscall/path | Keep return ABI as `buf`; remove display rewrites after namespace fix. |
| `dup` | 23 | `sys_dup` | Real-partial | basic, busybox, lmbench | medium | syscall/fd | Move to fd slot + shared open-file-description model. |
| `dup3` | 24 | `sys_dup3` | Real-partial | basic, busybox, pthreads | medium | syscall/fd | Implement `O_CLOEXEC`; keep unsupported flags as `EINVAL`. |
| `fcntl` | 25 | `sys_fcntl` | Partial | busybox, LTP, nptl | low | syscall/fd | Add fd flags, status flags, locks only when required. |
| `ioctl` | 29 | `sys_ioctl` | Compat/partial | busybox tty, shell | low | syscall/dev | Route tty/device ioctls through devfs/device registry. |
| `mkdirat` | 34 | `sys_mkdirat` | Real-partial | basic, busybox, LTP | medium | syscall/path/fs | Use unified resolver; honor mode/error matrix. |
| `unlinkat` | 35 | `sys_unlinkat` | Real-partial | basic, busybox, LTP | medium | syscall/path/fs | Add `AT_REMOVEDIR`, dir/file checks, sticky/perms later. |
| `umount2` | 39 | `sys_umount2` | Compat | basic, LTP fs_bind | low | axfs/syscall | Replace `compat_mounts` with real runtime unmount. |
| `mount` | 40 | `sys_mount` | Compat | basic, LTP fs_bind/fs_readonly | low | axfs/syscall | Add runtime mount API and fs factory. |
| `statfs` | 43 | none | Missing | busybox `stat`, LTP | n/a | axfs | Add fs stat interface or return consistent `ENOSYS` before gate. |
| `fstatfs` | 44 | none | Missing | busybox, LTP | n/a | axfs | Same as `statfs`. |
| `truncate` | 45 | none | Missing | busybox, iozone, LTP | n/a | syscall/fs | Add path-based truncate using resolver + file set_len. |
| `ftruncate` | 46 | `sys_ftruncate` | Real-partial | busybox, iozone, LTP | medium | syscall/fd/fs | Move to open-file-description backend. |
| `fallocate` | 47 | none | Missing | iozone, LTP | n/a | fs | Return `EOPNOTSUPP` until filesystem allocation exists. |
| `faccessat` | 48 | `sys_faccessat` | Compat/partial | busybox, LTP perms | low | syscall/path/fs | Add real permission model or bounded `access(2)` semantics. |
| `chdir` | 49 | `sys_chdir` | Real-partial | basic, busybox, shell | medium | syscall/path | Use unified resolver and namespace root. |
| `fchdir` | 50 | none | Missing | busybox, LTP | n/a | syscall/fd/path | Add once directory fd model is stable. |
| `openat` | 56 | `sys_openat` | Real-partial | basic, busybox, iozone, lmbench, LTP | medium | syscall/fd/path/fs | Complete flags, mode, path resolver, OFD model. |
| `close` | 57 | `sys_close` | Real-partial | all | medium | syscall/fd | Slot removal only; backend lifetime via OFD `Arc`. |
| `pipe2` | 59 | `sys_pipe2` | Real-partial | basic, UnixBench, lmbench, nptl | medium | syscall/fd/ipc | Add flags, blocking semantics, close wakeups. |
| `getdents64` | 61 | `sys_getdents64` | Real-partial | basic, busybox `find`, LTP | medium | syscall/fd/fs | Store directory offset in OFD; support repeated reads. |
| `lseek` | 62 | `sys_lseek` | Real-partial | busybox, iozone, lmbench | medium | syscall/fd/fs | Use shared OFD offset; reject nonseekable fds. |
| `read` | 63 | `sys_read` | Real-partial | all | medium | syscall/fd | Validate fd before user buffer; use OFD offset. |
| `write` | 64 | `sys_write` | Real-partial | all | medium | syscall/fd | Add `O_APPEND`, short writes, pipe close errors. |
| `readv` | 65 | `sys_readv` | Partial | busybox, iozone, nptl | low | syscall/fd | Match partial iovec and fault ordering. |
| `writev` | 66 | `sys_writev` | Partial | busybox, iozone, nptl | low | syscall/fd | Same as `readv`. |
| `pread64` | 67 | `sys_pread64` | Partial | iozone, lmbench | medium | syscall/fd/fs | Ensure explicit offset does not alter OFD offset. |
| `pwrite64` | 68 | none | Missing | iozone, UnixBench | n/a | syscall/fd/fs | Add offset write backend. |
| `preadv` | 69 | none | Missing | iozone | n/a | syscall/fd/fs | Add after scalar pread/pwrite semantics. |
| `pwritev` | 70 | none | Missing | iozone | n/a | syscall/fd/fs | Add after scalar pread/pwrite semantics. |
| `sendfile` | 71 | none | Missing | busybox/coreutils possible | n/a | syscall/fd/fs | Defer; return `ENOSYS` until needed. |
| `pselect6` | 72 | `sys_pselect6` | Partial | busybox, lmbench select | low | syscall/fd/signal | Finish fd readiness and signal mask semantics. |
| `readlinkat` | 78 | none | Missing | busybox, LTP symlink | n/a | path/fs | Defer symlink support; return `ENOSYS` or `EINVAL` consistently. |
| `sync` | 81 | none | Missing | busybox, LTP | n/a | fs | Add global flush or explicit `ENOSYS`. |
| `fsync` | 82 | none | Missing | iozone, UnixBench, LTP | n/a | syscall/fd/fs | Wire to backend `flush`. |
| `fdatasync` | 83 | none | Missing | iozone, LTP | n/a | syscall/fd/fs | Same as `fsync`, data-only if supported. |
| `utimensat` | 88 | `sys_utimensat` | Compat/partial | busybox, LTP | low | fs/path | Add timestamp metadata or bounded errors. |
| `exit` | 93 | `sys_exit` | Real-partial | all process tests | medium | task/syscall | Separate thread exit and process exit. |
| `exit_group` | 94 | `sys_exit_group` | Real-partial | libc/nptl | medium | task/syscall | Finish thread-group teardown. |
| `waitid` | 95 | none | Missing | LTP process | n/a | task/syscall | Add after wait state model. |
| `set_tid_address` | 96 | `sys_set_tid_address` | Partial | nptl, libc | medium | task/futex | Keep clear-child-tid wake semantics accurate. |
| `futex` | 98 | `sys_futex` | Partial | nptl, lmbench, LTP | low | sync/task | Expand op set; define alignment/fault ordering. |
| `set_robust_list` | 99 | `sys_set_robust_list` | Partial | nptl | low | task/futex | Implement robust-list exit handling. |
| `get_robust_list` | 100 | `sys_get_robust_list` | Partial | nptl, LTP | low | task/futex | Validate pid/tid model. |
| `nanosleep` | 101 | `sys_nanosleep` | Real-partial | basic sleep, busybox | medium | task/time | Add signal interruption behavior. |
| `clock_gettime` | 113 | `sys_clock_gettime` | Real-partial | busybox, cyclictest | medium | time | Ensure clock ids and precision. |
| `clock_nanosleep` | 115 | `sys_clock_nanosleep` | Partial | cyclictest | low | task/time | Add absolute sleeps and interruption. |
| `sched_setparam` | 118 | `sys_sched_setparam` | Partial | cyclictest, LTP sched | low | task/sched | Map to real scheduler or reject unsupported policy. |
| `sched_setscheduler` | 119 | `sys_sched_setscheduler` | Partial | cyclictest, LTP sched | low | task/sched | Same. |
| `sched_getscheduler` | 120 | `sys_sched_getscheduler` | Partial | cyclictest, LTP sched | low | task/sched | Same. |
| `sched_getparam` | 121 | `sys_sched_getparam` | Partial | cyclictest, LTP sched | low | task/sched | Same. |
| `sched_setaffinity` | 122 | `sys_sched_setaffinity` | Partial | LTP sched | low | task/sched | Validate cpuset sizes and task ids. |
| `sched_getaffinity` | 123 | `sys_sched_getaffinity` | Partial | LTP sched | low | task/sched | Same. |
| `sched_yield` | 124 | `sys_sched_yield` | Real-partial | basic yield, UnixBench | medium | task | Keep simple; add signal interactions later. |
| `kill/tkill/tgkill` | 129/130/131 | `sys_kill/sys_tkill/sys_tgkill` | Partial | nptl, lmbench sig, LTP | low | task/signal | Finish pid/tid lookup and delivery semantics. |
| `rt_sigaction` | 134 | `sys_rt_sigaction` | Partial | nptl, busybox | low | signal | Complete flags, restorer, masks. |
| `rt_sigprocmask` | 135 | `sys_rt_sigprocmask` | Partial | nptl | low | signal | Finish thread-local masks. |
| `rt_sigtimedwait` | 137 | `sys_rt_sigtimedwait` | Partial | LTP signal | low | signal | Define pending signal queue. |
| `rt_sigreturn` | 139 | `sys_rt_sigreturn` | Partial | signal tests | low | signal | Verify arch frame layout. |
| `times` | 153 | `sys_times` | Partial | basic, UnixBench | medium | time/task | Fill process CPU times when available. |
| `getrusage` | 165 | `sys_getrusage` | Partial | wait4, UnixBench | low | task/time | Add child/self resource accounting. |
| `gettimeofday` | 169 | `sys_gettimeofday` | Real-partial | busybox, lmbench | medium | time | Confirm tz behavior. |
| `getpid/getppid/gettid` | 172/173/178 | inline | Partial | basic, all process tests | medium | task/process | Separate pid/tgid/tid. |
| `shmget/shmctl/shmat/shmdt` | 194-197 | none | Missing | LTP mm | n/a | mm/ipc | Add shared-memory object model or return `ENOSYS`. |
| `brk` | 214 | `sys_brk` | Partial | basic, libcbench | medium | mm/syscall | Move heap into VMA model. |
| `munmap` | 215 | `sys_munmap` | Partial | basic, libcbench, LTP | medium | mm/syscall | Use VMA splitting/merging. |
| `mremap` | 216 | none | Missing | LTP mm | n/a | mm | Add after VMA model. |
| `clone` | 220 | `sys_clone` | Partial | basic, nptl, UnixBench, lmbench | low | task/mm/fd | Freeze supported flags; reject rest deterministically. |
| `execve` | 221 | `sys_execve` | Real-partial | basic, busybox shell, UnixBench | medium | task/loader/fd | Add close-on-exec and atomic replacement. |
| `mmap` | 222 | `sys_mmap` | Partial | basic, libcbench, iozone, lmbench | low | mm/fs/fd | Replace eager file read with VMA file mapping. |
| `mprotect` | 226 | `sys_mprotect` | Partial | libc, nptl, LTP | low | mm | Use VMA permissions and splitting. |
| `msync` | 227 | none | Missing | mmap IO, LTP | n/a | mm/fs | Add after file-backed mapping. |
| `mlock/munlock/mlockall/munlockall/mlock2` | 228-231/284 | inline `0` | Stub-success | LTP mm | bad | mm/syscall | Replace silent success with real pinning or `ENOSYS/EOPNOTSUPP`. |
| `mincore` | 232 | none | Missing | LTP mm | n/a | mm | Add VMA/page residency query. |
| `madvise` | 233 | none | Missing | LTP mm/libc | n/a | mm | Add supported advice or `EINVAL/EOPNOTSUPP`. |
| `mbind/get_mempolicy/set_mempolicy` | 235-237 | `sys_mbind/sys_get_mempolicy/sys_set_mempolicy` | Compat/partial | LTP numa | low | mm/numa | Return explicit unsupported semantics unless NUMA exists. |
| `migrate_pages/move_pages` | 238/239 | none | Missing | LTP numa | n/a | mm/numa | Return `ENOSYS` until NUMA support. |
| `wait4` | 260 | `sys_wait4` | Partial | basic wait, UnixBench, lmbench | medium | task/process | Implement full pid/options/rusage matrix. |
| `prlimit64` | 261 | `sys_prlimit64` | Partial | busybox, libc | medium | process | Extend resource set as needed. |
| `renameat2` | 276 | `sys_renameat2` | Partial | busybox `mv`, LTP | low | syscall/path/fs | Honor flags and overwrite/cross-fs rules. |
| `statx` | 291 | `sys_statx` | Compat | busybox `stat`, LTP | low | fs/syscall | Back with real metadata and honest masks. |
| `rseq` | 293 | none | Missing | modern libc/nptl | n/a | task | Return `ENOSYS` unless libc requires registration. |
| `clone3` | 435 | none | Missing | LTP sched/nptl | n/a | task/process | Add only after clone contract is stable. |
| `openat2` | 437 | none | Missing | LTP fs | n/a | path/fs | Add resolver flags after `openat` is correct. |
| `mount_setattr` | 442 | none | Missing | LTP fs_bind | n/a | axfs | Add after real mount API. |

## Filesystem Interface Matrix

| Area | Current syscall target | Available real interface | Status | Next architectural move |
| --- | --- | --- | --- | --- |
| `open/openat` | `FdEntry::File`, `FdEntry::Directory` | `File::options`, `Directory::open_dir`, `metadata` | Partly real | Complete Linux flag translation, mode handling, `O_CLOEXEC`, error matrix, and directory/file distinction. |
| `close` | `FdTable` slot release | local fd authority | Real local semantics | Keep fd lifecycle in `FdTable`; add close-on-exec state rather than special-casing stdio. |
| `dup/dup2/dup3` | `FdTable` duplication | local fd authority | Partly real | Preserve shared file offsets where Linux requires it; reject unsupported flags with `EINVAL`. |
| `read/write` | fd dispatch | `Read`, `Write`, pipe buffers, console | Partly real | Add short-read/write behavior and shared-offset correctness across duplicated fds. |
| `pread/pwrite` | fd dispatch | positional file I/O helpers or seek/restore | Partial/needed | Prefer true offset-based file APIs; avoid disturbing shared file offset. |
| `readv/writev` | iovec dispatch | repeated `read/write` | Partial | Enforce Linux partial-transfer and fault behavior. |
| `lseek` | fd dispatch | `Seek` | Partly real | Reject nonseekable fds consistently; preserve offset sharing. |
| `fsync/fdatasync` | file flush | `Write::flush` | Needed | Wire to real flush and return errors from the filesystem. |
| `ftruncate/truncate` | file length update | `File::set_len` | Partly real | Add path-based `truncate` and permission/error coverage. |
| `getcwd/chdir` | `UserProcess.cwd` | `axfs::api::current_dir`, `set_current_dir` | Partly real | Keep per-process cwd but remove display-only path rewrites once test staging has a real namespace contract. |
| `getdents64` | `FdEntry::Directory` | `ReadDir`, `DirEntry` | Partly real | Implement stable offsets, repeated reads, and buffer-boundary behavior. |
| `mkdir/rmdir` | path operations | `create_dir`, `remove_dir` | Partly real | Add mode/perms, parent checks, and Linux errno compatibility. |
| `unlink/rename` | path operations | `remove_file`, `rename` | Partly real | Add directory vs file checks, `unlinkat` flags, `renameat2` flags, cross-fs errors, and overwrite rules. |
| `stat/fstat/newfstatat` | `general::stat` projection | `Metadata`, fd metadata | Partly real | Fill timestamps, device ids, uid/gid, block size, link count, and special file modes from real metadata. |
| `statx` | projection from `stat` | no full statx source | Compatibility-only | Replace projection when filesystem metadata can provide statx fields; honor masks and flags. |
| `mount/umount2` | process-local `compat_mounts` | internal `RootDirectory::mount` only | Compatibility-only | Expose real runtime mount/unmount API, filesystem factory, and target lifetime ownership. |
| `/dev/*` nodes | ad hoc fd variants | no devfs contract here | Compatibility-only | Route through a real devfs or device registry when available. |

## Filesystem Test Implications

`basic` validates functional syscall ABI and catches obvious fd/path bugs. The
already implemented compatibility mount can pass the current basic mount case
because that test does not inspect mounted contents, but it must not be treated
as a real mount implementation.

`busybox` raises the bar from single syscalls to command behavior. The main
risks are `stat`, `find`, `mv`, `cp`, `rm -r`, `rmdir`, path normalization,
directory iteration offsets, and overwrite/error semantics.

`iozone`, `UnixBench fstime`, and `lmbench` filesystem tests require stable
performance and correct repeated I/O. The priority interfaces are shared file
offsets, `pread/pwrite`, vector I/O, `fsync`, truncation, and mmap read paths.

LTP filesystem runtests require negative-path correctness. This means hardcoded
"success if it looks valid" behavior must be removed before running broad LTP.
Unsupported mount flags, permissions, readonly filesystems, invalid dirfds, bad
buffers, and non-directory paths should return explicit Linux-compatible errno.

## Workload Gate Matrix

| Workload | Syscall/behavior focus | Expected current result | Promotion gate | Reduction target |
| --- | --- | --- | --- | --- |
| `basic` filesystem/fd | `openat`, `close`, `dup`, `dup3`, `fstat`, `getcwd`, `getdents64`, `mkdirat`, `mount`, `umount2`, `pipe2`, `read`, `unlinkat`, `write` | Should pass focused subset with current compatibility mount | Phase 0 regression | Individual `basic/user/src/oscomp/*.c` case and matching Python assertion. |
| `busybox` file commands | `openat`, `read/write`, `lseek`, `statx/newfstatat/fstat`, `getdents64`, `mkdirat`, `unlinkat`, `renameat2`, `utimensat`, `faccessat`, `readlinkat` | Partial; likely failures around metadata, recursive dir ops, symlink/readlink, timestamps | Phase 1 filesystem gate | Single busybox applet command with one temp directory. |
| `iozone` | `openat`, `read/write`, `pread64/pwrite64`, `readv/writev`, `preadv/pwritev`, `lseek`, `ftruncate`, `fsync/fdatasync`, file-backed `mmap/msync` | Partial; `pwrite*`, `fsync`, `msync`, and real file-backed mmap are gaps | Phase 1 IO gate, then Phase 2 mmap gate | One iozone mode and block size, then reduce to scalar syscall reproducer. |
| `UnixBench fstime` | repeated create/write/read/copy/unlink plus `fork/wait/times` overhead | Partial; fd offset, metadata, and process cost may dominate | Phase 1 performance smoke | `fstime` small-file case before medium/large. |
| `lmbench` fs/file | `lat_syscall` read/write/open/stat/fstat, `lmdd`, `lat_fs`, `bw_file_rd`, `bw_mmap_rd` | Partial; mmap read path not semantically complete | Phase 1/2 latency gate | One lmbench microbenchmark invocation. |
| LTP `fs` | open/read/write/stat/getdents/link/unlink/rename/truncate/error paths | Not gate-ready until errno matrix and resolver are stable | Phase 1 negative-path gate | One LTP syscall testcase under `testcases/kernel/syscalls`. |
| LTP `fs_bind` | real `mount`, `umount2`, mount propagation/attributes where applicable | Not gate-ready; current mount is compatibility-only | After real runtime mount API | Smallest mount/umount testcase first. |
| LTP `fs_perms_simple` | permission checks, ownership, access, readonly cases | Not gate-ready; permission model incomplete | After permission metadata contract | One permission testcase with fixed uid/gid assumptions. |
| LTP `fs_readonly` | write rejection on readonly mounts/filesystems | Not gate-ready; readonly mount state missing | After real mount flags | One readonly mount write attempt. |
| `basic` memory | `brk`, `mmap`, `munmap` | Should be functional for simple anonymous cases | Phase 0/2 regression | Individual basic memory case. |
| `libcbench` | allocator pressure, anonymous mmap/brk, string memory access | Partial; depends on VMA robustness | Phase 2 allocator smoke | Single libcbench subtest. |
| `lmbench` memory | `lat_pagefault`, `lat_mmap`, `bw_mmap_rd` | Partial; lazy fault and file-backed mmap are weak | Phase 2 memory gate | One lmbench memory case. |
| LTP `mm/hugetlb/numa` | `mmap`, `munmap`, `mprotect`, `mremap`, `madvise`, `mincore`, `mlock`, NUMA policy, shared memory | Not gate-ready | Phase 2/3 after VMA contracts | One syscall testcase; unsupported NUMA should be explicit `ENOSYS/EOPNOTSUPP`. |
| `basic` process | `clone`, `execve`, `exit`, `fork`, `getpid`, `getppid`, `sleep`, `times`, `wait`, `yield` | Partial; current supported clone paths need fixed contract | Phase 0/3 regression | Individual basic process case. |
| `cyclictest` | `sched_*`, sleep/clock, thread creation, priority behavior | Partial; scheduler policy mapping incomplete | Phase 3 scheduler gate | Single-thread cyclictest before multi-thread/hackbench. |
| `UnixBench` process | context switch, pipe, spawn, execl, syscall overhead | Partial; fork/exec/wait and pipe semantics/perf | Phase 3 process perf gate | One UnixBench process test. |
| `lmbench` process/signal | `fork/exec/shell`, context switch, pipe, select, signal latency | Partial; signal/futex/select semantics incomplete | Phase 3 process/signal gate | One lmbench subtest. |
| LTP `sched/nptl/cpuhotplug` | clone/thread groups, futex, robust list, signals, scheduler APIs | Not gate-ready | Phase 3 after process/thread model | One LTP syscall or nptl testcase. |

Gate rule: a workload can become a promotion gate only when unsupported
syscalls in its row return explicit Linux-style errors and no required behavior
is still implemented as silent success.

Pass/fail standards:

| Gate | Pass standard |
| --- | --- |
| `basic` filesystem/fd | The focused cases `chdir`, `close`, `dup`, `dup2`, `fstat`, `getcwd`, `getdents`, `mkdir_`, `mount`, `open`, `openat`, `pipe`, `read`, `umount`, `unlink`, and `write` pass on RISC-V64 and LoongArch64, and the gate notes show no new `Stub-success` paths. |
| `busybox` file commands | The file-operation section of `scripts/busybox/busybox_cmd.txt` passes: `touch`, redirection write/append, `cat`, `cut`, `od`, `head`, `tail`, `hexdump`, `md5sum`, `sort | uniq`, `stat`, `strings`, `wc`, `[ -f ]`, `more`, `rm`, `mkdir`, `mv`, `rmdir`, `grep`, `cp`, and `find`. |
| `iozone` functional gate | The commands in `scripts/iozone/iozone_testcode.sh` complete for `-r 1k`, `-s 4m` automatic mode and the `-t 4`, `-r 1k`, `-s 1m` modes `0/1`, `0/2`, `0/3`, `0/5`, `6/7`, `9/10`, and `11/12`. This gate checks completion and data-path correctness, not final performance ranking. |
| `UnixBench fstime` | `fstime` write/read/copy emits counts for small `-b 256 -m 500`, middle `-b 1024 -m 2000`, and big `-b 4096 -m 8000` cases. Performance regressions are tracked separately unless they cause timeout or zero/invalid counts. |
| `lmbench` filesystem/file | The selected single-purpose microbenchmark emits a valid result and can be rerun without stale state; broaden only after the reduced case is stable. |
| LTP filesystem gates | Each promoted LTP testcase must either pass or fail only because the documented unsupported syscall returns the documented errno. Unexpected success from compatibility code is a gate failure. |
| Memory gates | Anonymous `brk/mmap/munmap` basic cases pass before libcbench; file-backed mmap gates require VMA-backed semantics, not eager one-shot file reads. |
| Process/scheduler gates | Basic process cases pass before cyclictest/UnixBench/lmbench; unsupported clone/scheduler flags return documented errors rather than being ignored. |

## Memory Interface Matrix

| Area | Current syscall target | Available real interface | Status | Next architectural move |
| --- | --- | --- | --- | --- |
| `brk` | `UserProcess.brk` and address-space mapping | `AddrSpace::map_alloc`, `unmap` | Partly real | Define heap VMA ownership and partial shrink behavior. |
| anonymous `mmap` | syscall flag translation | `AddrSpace::find_free_area`, `map_alloc` | Partly real | Complete `MAP_FIXED`, `MAP_FIXED_NOREPLACE`, guard-page, and protection rules. |
| file-backed `mmap` | not fully real | needs fd-backed VM object | Missing | Introduce file-backed mapping contract with page fault fill and dirty writeback for `MAP_SHARED`. |
| `munmap` | address-space unmap | `AddrSpace::unmap` | Partly real | Support splitting VMAs and non-whole-area unmap. |
| `mprotect` | protection update | `AddrSpace::protect` | Partly real | Validate Linux permission transitions and VMA splitting. |
| page fault | trap path to address space | `AddrSpace::handle_page_fault` | Partly real | Add lazy allocation/file fault/COW decisions above page-table mechanics. |
| `fork` memory | eager copy | `clone_user_mappings_from` | Functional but expensive | Move to COW when task and memory layers can share refcounted pages. |
| `mremap/mincore/madvise/mlock` | syscall surface needed for LTP | no complete contract | Missing | Add memory-manager contracts before syscall stubs. |
| SysV shared memory | `shm*` family | no contract identified | Missing | Decide whether to implement as named shared VM objects or a separate IPC layer. |

## Process and Scheduling Interface Matrix

| Area | Current syscall target | Available real interface | Status | Next architectural move |
| --- | --- | --- | --- | --- |
| `clone/fork` | `UserProcess::fork`, task spawn | `axtask` spawn/wait and address-space clone | Partly real | Define supported clone flags, thread-group model, fd sharing, signal stack, TLS, and COW memory. |
| `vfork` | likely fallback/missing | task blocking primitives | Missing | Add parent blocking and shared address-space contract until exec/exit. |
| `execve` | ELF/script loader | loader plus new address space | Partly real | Preserve Linux fd close-on-exec, argv/envp limits, interpreter handling, and error cleanup. |
| `exit/exit_group` | process status and task exit | `axtask::exit` | Partly real | Clarify process vs thread-group exit semantics. |
| `wait/waitpid/wait4/waitid` | child list and wait queue | wait queues | Partly real | Add options, rusage, waitid result format, and zombie lifecycle. |
| `getpid/getppid/times` | process metadata/time | task id and time APIs | Partly real | Separate Linux pid namespace from internal task id if needed. |
| `sleep/yield` | scheduling calls | sleep/yield APIs | Real enough | Preserve signal interruption behavior later. |
| `sched_*` | scheduler ABI | priority/affinity APIs | Partial | Map Linux policies carefully; unsupported realtime settings must return explicit errno. |
| futex/signals | synchronization and delivery | wait queues, partial signal code | Partial | Needed before broad LTP nptl and lmbench signal paths. |

## Errno and User-Memory Policy

Linux tests often check both errno value and validation order. The default rule
for this project is:

1. validate syscall number and architecture dispatch;
2. validate scalar flags and impossible argument combinations that do not touch
   user memory;
3. validate fd/pid/tid handles before user data buffers for fd/process syscalls;
4. copy input user memory before performing visible state changes;
5. perform the operation;
6. copy output user memory last, and roll back state when the syscall contract
   requires atomicity.

Unsupported-feature strategy:

- a syscall with no dispatcher arm returns `ENOSYS`.
- a known syscall with an unsupported flag, option, or invalid flag combination
  usually returns `EINVAL`.
- a known syscall whose backend capability does not exist returns
  `EOPNOTSUPP`, `ENODEV`, or a more specific backend errno.
- a syscall handler that exists only for future integration must not return 0
  for unimplemented behavior.
- `statx` unsupported fields are omitted from the returned mask; invalid flags
  still return `EINVAL`.
- `mount` with an unknown filesystem type returns `ENODEV` or `EOPNOTSUPP`;
  unsupported mount flags return `EINVAL` or `EOPNOTSUPP`.
- NUMA and page-pinning calls return explicit unsupported errors until real
  NUMA or pinning state exists.

High-frequency syscall rules:

| Syscall | Validation order | Required errno behavior |
| --- | --- | --- |
| `openat` | flags/mode shape, pathname pointer/string, dirfd if relative, path resolution, create/open | bad pathname pointer `EFAULT`; unknown flags `EINVAL`; bad relative dirfd `EBADF`; non-directory dirfd `ENOTDIR`; missing path `ENOENT`; unsupported create/symlink/perms `EOPNOTSUPP` or real fs errno. |
| `read` | fd lookup, fd readable, user buffer if count > 0, backend read | bad fd before bad buffer gives `EBADF`; write-only/non-readable fd `EBADF`; directory fd `EISDIR`; bad buffer `EFAULT`; pipe closed behavior must distinguish EOF vs `EPIPE` where relevant. |
| `write` | fd lookup, fd writable, user buffer if count > 0, backend write | bad fd before bad buffer gives `EBADF`; read-only/non-writable fd `EBADF`; bad buffer `EFAULT`; closed pipe `EPIPE` plus signal later; unsupported append/special device must not return fake success. |
| `statx` | flags/mask, pathname pointer unless `AT_EMPTY_PATH`, dirfd/path resolution, metadata query, output buffer | unknown flags `EINVAL`; bad output pointer `EFAULT`; unsupported fields are omitted from returned mask, not guessed; no fake success for unimplemented path flags. |
| `mmap` | length, flag combinations, page alignment, fd/mode for file mappings, address range, VMA install | length 0 `EINVAL`; unsupported flags `EINVAL` or `EOPNOTSUPP`; bad fd `EBADF`; offset alignment `EINVAL`; permission mismatch `EACCES`; no address space `ENOMEM`. |
| `clone` | flag combination, required stack/tid pointers for supported modes, memory/fd/process creation, parent/child tid writes | unsupported flag combination returns `EINVAL` once contract is frozen; bad `ptid/ctid` pointers `EFAULT`; resource exhaustion `EAGAIN` or `ENOMEM`; no ignored flags. |
| `wait4/waitid` | option flags, child selection, wait state, output status/rusage write | unknown options `EINVAL`; no matching child `ECHILD`; nonblocking no-child-exited returns 0 for `WNOHANG`; bad output pointer `EFAULT` when writing a result. |
| `futex` | aligned user address, op command, op-specific user pointers, value check, queue operation | unaligned/null futex address `EINVAL`; bad user address `EFAULT`; unsupported op `ENOSYS` or `EOPNOTSUPP`; value mismatch `EAGAIN`; timeout `ETIMEDOUT`; signal interruption `EINTR`. |

Any syscall-local deviation from this table must be documented next to the
handler with the workload that requires it.

## Minimum Process/Thread Boundary

Before Phase 3, the current `clone` behavior should be frozen as a documented
subset instead of growing by ignored flags.

Phase 0/1 allowed process-like modes:

- `fork`/`clone` with no sharing flags and exit signal 0 or `SIGCHLD`.
- optional `CLONE_SETTLS`, `CLONE_PARENT_SETTID`, `CLONE_CHILD_SETTID`, and
  `CLONE_CHILD_CLEARTID` only if the corresponding user pointer is valid.
- `CLONE_VFORK | CLONE_VM` may be accepted only if parent blocking and shared
  address-space lifetime are implemented; otherwise return `EINVAL`.

Phase 0/1 allowed thread-like mode:

- exactly the pthread-style sharing set required by libc:
  `CLONE_VM | CLONE_FS | CLONE_FILES | CLONE_SIGHAND | CLONE_SYSVSEM |
  CLONE_THREAD`, plus optional TLS and tid flags.
- child stack must be nonzero.
- the new thread shares process memory and fd table; `tid` differs from `tgid`.

Rejected modes:

- unknown or unmodeled flags return `EINVAL`, not success.
- partial sharing combinations, such as `CLONE_FILES` without the rest of the
  thread model, return `EINVAL` until their lifecycle semantics are defined.
- `clone3` returns `ENOSYS` until the `clone` contract is complete.

## Compatibility-Only Exit Rules

Compatibility code is allowed only as a bridge to a named milestone. It must be
easy to find and safe to delete.

Naming and comment format:

```rust
// compat(<milestone>): <why this exists>
// delete-when: <real interface or test gate that removes it>
```

Rules:

- function names, fields, or modules that hold fake state must use the
  `compat_` prefix.
- every compatibility path must list a milestone and a deletion condition.
- compatibility code may emulate a narrow successful path, but must return
  explicit errors for unsupported states.
- no compatibility path may claim success for a state-changing operation unless
  it records enough state to make the matching inverse operation meaningful.
- no new `Stub-success` syscall is allowed after Phase 0.
- before any broad suite gate, run a source scan for `compat_`, `Stub-success`,
  and direct `=> 0` syscall arms; each hit must be justified in the gate notes.
- compatibility state must not be copied into real subsystem APIs. When the real
  interface exists, the compatibility state is deleted rather than synchronized.

Current compatibility exits:

| Compatibility path | Delete when | Required interim behavior |
| --- | --- | --- |
| `compat_mounts` | runtime `axfs` mount/unmount can expose mounted contents | keep target state machine; duplicate mount `EBUSY`; unmounted target `EINVAL/ENOENT`; no fake success for flags/data. |
| `statx` from `stat` | filesystem metadata returns statx-capable fields and mask | report only honest fields; unknown flags `EINVAL`; bad buffers `EFAULT`. |
| test staging cwd display rewrite | user process launch has a single namespace root | path resolution and getcwd observe the same namespace; no display-only behavior in broad gates. |
| `mlock* => 0` | real page pinning exists or workload explicitly accepts unsupported | replace with `ENOSYS/EOPNOTSUPP` before LTP mm. |
| `/dev/*` ad hoc fd entries | devfs/device registry exists | only known devices succeed; unknown devices return `ENOENT` or `ENODEV`. |

## Replacement Path for Current Compatibility Code

### Mount/Umount

Current state: `sys_mount` records normalized targets in `compat_mounts`; it
does not create a VFS mount. This is acceptable only as a basic-test
compatibility shim.

Target state:

1. Expose a runtime mount API above `RootDirectory` that accepts owned target
   paths and supports unmount.
2. Add a filesystem factory for supported `fstype` values. The syscall layer
   should not hardcode `vfat`; it should ask the factory whether `vfat`, `ext4`,
   `tmpfs`, or another type is available.
3. Resolve `source` through a real block-device or devfs interface.
4. Keep Linux validation in `sys_mount`, but perform the actual state change in
   the filesystem layer.
5. Delete `compat_mounts` once mounted contents are observable through VFS.

### Statx

Current state: `sys_statx` projects from `stat`, so several fields are guessed
or missing.

Target state:

1. Extend filesystem metadata or add a statx-specific query object.
2. Preserve Linux mask/flag validation in the syscall layer.
3. Fill only fields that are backed by real filesystem metadata, and report the
   returned mask honestly.

### Test Staging Path Display

Current state: cwd display may rewrite `/tmp/testsuite...` into a virtual view
for compatibility with staged tests.

Target state:

1. Define one namespace root for user programs before launch.
2. Make cwd and path resolution agree on that namespace.
3. Remove display-only rewrites once tests run under the same namespace they
   observe through filesystem syscalls.

## Implementation Order

### Phase 0: Guardrails

- Keep the existing basic filesystem/fd pass as the regression baseline.
- Label compatibility-only paths in code and docs.
- Ensure unsupported syscall flags return errors, not accidental success.
- Add the syscall atlas table to review for every new syscall handler.
- Freeze the supported `clone` flag subset and convert ignored sharing flags to
  explicit errors.
- Replace any new silent-success stub with `ENOSYS`, `EINVAL`, or
  `EOPNOTSUPP`.
- Add small focused checks before broad workload runs.

### Phase 1A: FD, Resolver, and Errno Guardrails

- Implement the fd slot/open-file-description model before expanding broad file
  semantics.
- Add the unified path resolver and route `openat`, `mkdirat`, `unlinkat`,
  `renameat2`, `statx/newfstatat`, and `chdir` through it.
- Add `FD_CLOEXEC`, fd status flags, and shared offset semantics for duplicated
  fds and forked children.
- Convert unsupported flags and missing backend capabilities to explicit errno.
- Keep the syscall atlas updated with every touched handler.
- Gate with focused `basic` filesystem/fd plus the busybox file commands that
  do not require mount/devfs.

### Phase 1B: Core File I/O and Metadata

- Complete `fsync`, `fdatasync`, `truncate`, `ftruncate`, `pread/pwrite`,
  `readv/writev`, `preadv/pwritev`, and repeated `getdents64` offset
  semantics.
- Improve metadata projection for `stat`, `fstat`, `newfstatat`, `statx`,
  `statfs`, and `fstatfs`.
- Make `O_APPEND`, short I/O, nonseekable fd errors, and directory fd errors
  match the errno policy.
- Gate with busybox file commands, iozone functional commands, UnixBench
  `fstime`, and selected lmbench filesystem/file microbenchmarks.

### Phase 1C: Runtime Mount and Device Namespace

- Replace compatibility mount state with a real runtime mount/unmount API.
- Add or route through real device/filesystem factory interfaces.
- Move `/dev/*` handling behind a devfs or device registry.
- Delete `compat_mounts` once mounted contents are observable through VFS.
- Gate with LTP `fs_bind`, `fs_readonly`, and mount-related busybox behavior.

### Phase 2: Memory Manager Contracts

- Define VMAs for heap, anonymous mappings, file-backed mappings, and stack.
- Add file-backed mmap page fault fill and `MAP_SHARED` writeback.
- Support VMA splitting for `munmap`, `mprotect`, and later `mremap`.
- Decide the COW strategy for `fork`.

### Phase 3: Process and Scheduler Semantics

- Expand `clone` flag support deliberately instead of accepting ignored flags.
- Add `vfork`, close-on-exec, wait variants, and thread-group semantics.
- Map scheduler syscalls to real task-layer controls where available.
- Treat cyclictest and LTP nptl/sched failures as interface-contract feedback,
  not as places for syscall-local hardcoding.

## Verification Strategy

Run verification inside `arceos-eval-fix`.

Minimum gates:

1. After Phase 1A changes: rerun the focused `basic` filesystem/fd subset on
   RISC-V and LoongArch, then run the busybox file commands that do not depend
   on mount/devfs.
2. After Phase 1B changes: run busybox file commands, iozone functional
   commands, UnixBench fstime, and selected lmbench filesystem/file tests.
3. After Phase 1C changes: run mount/devfs-focused tests, then LTP `fs_bind`
   and `fs_readonly` reductions.
4. After memory changes: run `basic` memory tests, libcbench, lmbench mmap and
   pagefault tests, then focused LTP memory runtests.
5. After process/scheduling changes: run `basic` process tests, cyclictest,
   UnixBench process tests, lmbench process/signal tests, then focused LTP
   sched/nptl runtests.

When a broad suite fails, reduce it to the smallest syscall or command and add
that reduced case to the phase gate before making another broad pass.

## Open Decisions

- Which teammate-owned layer will expose runtime mount/unmount and device
  lookup? The syscall layer should adapt to that interface rather than replace
  it.
- Should `statx` be backed by an extended metadata trait or by filesystem-
  specific optional fields?
- Should the VMA layer be provided directly by `axmm`, or should the shell
  syscall path maintain a transitional `MemoryMap` until `axmm` owns Linux VMA
  semantics?
- Should the unified path resolver live in the syscall layer, the `axfs` API
  layer, or a user-namespace layer above `axfs`?
- Should the unsupported-syscall errno policy be project-wide, or scoped first
  to the shell/testsuite syscall path?
- What is the intended Linux pid/thread model for `clone` and `fork` in this
  project?
- Which workloads are the first promotion gate after `basic`: busybox file
  commands or iozone?
