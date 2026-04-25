# Syscall Interface Atlas Design

Date: 2026-04-25

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
   syscall side around that real interface and leave a narrow adapter or TODO,
   not a hardcoded replacement.
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
- Add small focused checks before broad workload runs.

### Phase 1: Real Filesystem Interfaces

- Replace compatibility mount state with a real runtime mount/unmount API.
- Add or route through real device/filesystem factory interfaces.
- Complete `fsync`, `truncate`, `pread/pwrite`, `readv/writev`, and shared file
  offset semantics.
- Improve metadata projection for `stat`, `fstat`, `newfstatat`, and `statx`.
- Use `busybox`, `iozone`, `UnixBench fstime`, and `lmbench` filesystem tests as
  the promotion gate after `basic`.

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

1. After filesystem/fd changes: rerun the focused `basic` filesystem/fd subset
   on RISC-V and LoongArch where practical.
2. After real filesystem interface changes: run busybox file commands, iozone,
   UnixBench fstime, and lmbench filesystem/file tests.
3. After memory changes: run `basic` memory tests, libcbench, lmbench mmap and
   pagefault tests, then focused LTP memory runtests.
4. After process/scheduling changes: run `basic` process tests, cyclictest,
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
- Should file offsets live in shared open-file descriptions instead of directly
  inside duplicated `FdEntry` values?
- What is the intended Linux pid/thread model for `clone` and `fork` in this
  project?
- Which workloads are the first promotion gate after `basic`: busybox file
  commands or iozone?

