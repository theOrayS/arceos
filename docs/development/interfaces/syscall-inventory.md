# Syscall Inventory

This file tracks syscall numbers, current handlers, status, workload sources,
and next actions. Update it in the same commit whenever a dispatcher arm or
`sys_*` handler is added, removed, renamed, or behaviorally changed.

Sources:

- `examples/shell/src/uspace.rs` for dispatcher arms and handler names.
- `testsuits-for-oskernel/basic/user/lib/syscall_ids.h` for the generic contest
  ABI syscall numbers.
- explicit architecture overrides in `uspace.rs` win only when documented here.

Status values:

- `Real-partial`: handler calls a real ArceOS interface but Linux semantics are
  incomplete.
- `Partial`: handler exists, but backing contracts or important Linux behavior
  are missing.
- `Compat`: bounded compatibility behavior exists and must have a deletion
  condition.
- `Compat/partial`: real handler exists but still contains compatibility
  behavior.
- `Stub-success`: returns success without implementing the operation. This is
  not allowed past Phase 0 gates.
- `Missing`: no handler or no real implementation.

## Inventory

| Syscall | nr(rv/la) | Current handler | Status | Required by workload | Errno fidelity | Owner | Next action |
| --- | ---: | --- | --- | --- | --- | --- | --- |
| `getcwd` | 17 | `sys_getcwd` | Real-partial | basic, busybox, LTP | medium | syscall/path | Keep return ABI as `buf`; remove display rewrites after namespace fix. |
| `dup` | 23 | `sys_dup` | Real-partial | basic, busybox, lmbench | medium | syscall/fd | Move to fd slot + shared open-file-description model. |
| `dup3` | 24 | `sys_dup3` | Real-partial | basic, busybox, pthreads | medium | syscall/fd | Implement `O_CLOEXEC`; keep unsupported flags as `EINVAL`. |
| `fcntl` | 25 | `sys_fcntl` | Partial | busybox, LTP, nptl | low | syscall/fd | Add fd flags, status flags, locks when required. |
| `ioctl` | 29 | `sys_ioctl` | Compat/partial | busybox tty, shell | low | syscall/dev | Route tty/device ioctls through devfs/device registry. |
| `mkdirat` | 34 | `sys_mkdirat` | Real-partial | basic, busybox, LTP | medium | syscall/path/fs | Use unified resolver; honor mode/error matrix. |
| `unlinkat` | 35 | `sys_unlinkat` | Real-partial | basic, busybox, LTP | medium | syscall/path/fs | Add `AT_REMOVEDIR`, dir/file checks, sticky/perms later. |
| `umount2` | 39 | `sys_umount2` | Compat | basic, LTP fs_bind | low | axfs/syscall | Replace `compat_mounts` with real runtime unmount. |
| `mount` | 40 | `sys_mount` | Compat | basic, LTP fs_bind/fs_readonly | low | axfs/syscall | Add runtime mount API and fs factory. |
| `statfs` | 43 | none | Missing | busybox `stat`, LTP | n/a | axfs | Add fs stat interface or return consistent `ENOSYS`. |
| `fstatfs` | 44 | none | Missing | busybox, LTP | n/a | axfs | Same as `statfs`. |
| `truncate` | 45 | none | Missing | busybox, iozone, LTP | n/a | syscall/fs | Add path truncate using resolver and file set_len. |
| `ftruncate` | 46 | `sys_ftruncate` | Real-partial | busybox, iozone, LTP | medium | syscall/fd/fs | Move to open-file-description backend. |
| `fallocate` | 47 | none | Missing | iozone, LTP | n/a | fs | Return `EOPNOTSUPP` until allocation exists. |
| `faccessat` | 48 | `sys_faccessat` | Compat/partial | busybox, LTP perms | low | syscall/path/fs | Add permission model or bounded `access(2)`. |
| `chdir` | 49 | `sys_chdir` | Real-partial | basic, busybox, shell | medium | syscall/path | Use unified resolver and namespace root. |
| `fchdir` | 50 | none | Missing | busybox, LTP | n/a | syscall/fd/path | Add once directory fd model is stable. |
| `openat` | 56 | `sys_openat` | Real-partial | basic, busybox, iozone, lmbench, LTP | medium | syscall/fd/path/fs | Complete flags, mode, path resolver, OFD model. |
| `close` | 57 | `sys_close` | Real-partial | all | medium | syscall/fd | Slot removal only; backend lifetime via OFD `Arc`. |
| `pipe2` | 59 | `sys_pipe2` | Real-partial | basic, UnixBench, lmbench, nptl | medium | syscall/fd/ipc | Add flags, blocking semantics, close wakeups. |
| `getdents64` | 61 | `sys_getdents64` | Real-partial | basic, busybox `find`, LTP | medium | syscall/fd/fs | Store directory offset in OFD. |
| `lseek` | 62 | `sys_lseek` | Real-partial | busybox, iozone, lmbench | medium | syscall/fd/fs | Use shared OFD offset; reject nonseekable fds. |
| `read` | 63 | `sys_read` | Real-partial | all | medium | syscall/fd | Validate fd before user buffer; use OFD offset. |
| `write` | 64 | `sys_write` | Real-partial | all | medium | syscall/fd | Add `O_APPEND`, short writes, pipe close errors. |
| `readv` | 65 | `sys_readv` | Partial | busybox, iozone, nptl | low | syscall/fd | Match partial iovec and fault ordering. |
| `writev` | 66 | `sys_writev` | Partial | busybox, iozone, nptl | low | syscall/fd | Same as `readv`. |
| `pread64` | 67 | `sys_pread64` | Partial | iozone, lmbench | medium | syscall/fd/fs | Explicit offset must not alter OFD offset. |
| `pwrite64` | 68 | none | Missing | iozone, UnixBench | n/a | syscall/fd/fs | Add offset write backend. |
| `preadv` | 69 | none | Missing | iozone | n/a | syscall/fd/fs | Add after scalar pread/pwrite semantics. |
| `pwritev` | 70 | none | Missing | iozone | n/a | syscall/fd/fs | Add after scalar pread/pwrite semantics. |
| `sendfile` | 71 | none | Missing | busybox/coreutils possible | n/a | syscall/fd/fs | Defer; return `ENOSYS` until needed. |
| `pselect6` | 72 | `sys_pselect6` | Partial | busybox, lmbench select | low | syscall/fd/signal | Finish readiness and signal mask semantics. |
| `readlinkat` | 78 | none | Missing | busybox, LTP symlink | n/a | path/fs | Defer symlink support; return consistent errno. |
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
| `sched_setparam` | 118 | `sys_sched_setparam` | Partial | cyclictest, LTP sched | low | task/sched | Map to scheduler or reject unsupported policy. |
| `sched_setscheduler` | 119 | `sys_sched_setscheduler` | Partial | cyclictest, LTP sched | low | task/sched | Same. |
| `sched_getscheduler` | 120 | `sys_sched_getscheduler` | Partial | cyclictest, LTP sched | low | task/sched | Same. |
| `sched_getparam` | 121 | `sys_sched_getparam` | Partial | cyclictest, LTP sched | low | task/sched | Same. |
| `sched_setaffinity` | 122 | `sys_sched_setaffinity` | Partial | LTP sched | low | task/sched | Validate cpuset sizes and task ids. |
| `sched_getaffinity` | 123 | `sys_sched_getaffinity` | Partial | LTP sched | low | task/sched | Same. |
| `sched_yield` | 124 | `sys_sched_yield` | Real-partial | basic yield, UnixBench | medium | task | Keep simple; add signal interactions later. |
| `kill/tkill/tgkill` | 129/130/131 | `sys_kill/sys_tkill/sys_tgkill` | Partial | nptl, lmbench sig, LTP | low | task/signal | Finish pid/tid lookup and delivery. |
| `rt_sigaction` | 134 | `sys_rt_sigaction` | Partial | nptl, busybox | low | signal | Complete flags, restorer, masks. |
| `rt_sigprocmask` | 135 | `sys_rt_sigprocmask` | Partial | nptl | low | signal | Finish thread-local masks. |
| `rt_sigtimedwait` | 137 | `sys_rt_sigtimedwait` | Partial | LTP signal | low | signal | Define pending signal queue. |
| `rt_sigreturn` | 139 | `sys_rt_sigreturn` | Partial | signal tests | low | signal | Verify arch frame layout. |
| `times` | 153 | `sys_times` | Partial | basic, UnixBench | medium | time/task | Fill process CPU times. |
| `getrusage` | 165 | `sys_getrusage` | Partial | wait4, UnixBench | low | task/time | Add child/self resource accounting. |
| `gettimeofday` | 169 | `sys_gettimeofday` | Real-partial | busybox, lmbench | medium | time | Confirm timezone behavior. |
| `getpid/getppid/gettid` | 172/173/178 | inline | Partial | basic, all process tests | medium | task/process | Separate pid/tgid/tid. |
| `shmget/shmctl/shmat/shmdt` | 194-197 | none | Missing | LTP mm/ipc | n/a | mm/ipc | Add shared-memory object model or return `ENOSYS`. |
| `brk` | 214 | `sys_brk` | Partial | basic, libcbench | medium | mm/syscall | Move heap into VMA model. |
| `munmap` | 215 | `sys_munmap` | Partial | basic, libcbench, LTP | medium | mm/syscall | Use VMA splitting/merging. |
| `mremap` | 216 | none | Missing | LTP mm | n/a | mm | Add after VMA model. |
| `clone` | 220 | `sys_clone` | Partial | basic, nptl, UnixBench, lmbench | low | task/mm/fd | Freeze supported flags; reject rest deterministically. |
| `execve` | 221 | `sys_execve` | Real-partial | basic, busybox shell, UnixBench | medium | task/loader/fd | Add close-on-exec and atomic replacement. |
| `mmap` | 222 | `sys_mmap` | Partial | basic, libcbench, iozone, lmbench | low | mm/fs/fd | Replace eager file read with VMA file mapping. |
| `mprotect` | 226 | `sys_mprotect` | Partial | libc, nptl, LTP | low | mm | Use VMA permissions and splitting. |
| `msync` | 227 | none | Missing | mmap IO, LTP | n/a | mm/fs | Add after file-backed mapping. |
| `mlock/munlock/mlockall/munlockall/mlock2` | 228-231/284 | inline `0` | Stub-success | LTP mm | bad | mm/syscall | Replace silent success with pinning or `ENOSYS/EOPNOTSUPP`. |
| `mincore` | 232 | none | Missing | LTP mm | n/a | mm | Add VMA/page residency query. |
| `madvise` | 233 | none | Missing | LTP mm/libc | n/a | mm | Add supported advice or explicit unsupported errno. |
| `mbind/get_mempolicy/set_mempolicy` | 235-237 | `sys_mbind/sys_get_mempolicy/sys_set_mempolicy` | Compat/partial | LTP numa | low | mm/numa | Return unsupported semantics unless NUMA exists. |
| `migrate_pages/move_pages` | 238/239 | none | Missing | LTP numa | n/a | mm/numa | Return `ENOSYS` until NUMA support. |
| `wait4` | 260 | `sys_wait4` | Partial | basic wait, UnixBench, lmbench | medium | task/process | Implement pid/options/rusage matrix. |
| `prlimit64` | 261 | `sys_prlimit64` | Partial | busybox, libc | medium | process | Extend resource set as needed. |
| `renameat2` | 276 | `sys_renameat2` | Partial | busybox `mv`, LTP | low | syscall/path/fs | Honor flags and overwrite/cross-fs rules. |
| `statx` | 291 | `sys_statx` | Compat | busybox `stat`, LTP | low | fs/syscall | Back with real metadata and honest masks. |
| `rseq` | 293 | none | Missing | modern libc/nptl | n/a | task | Return `ENOSYS` unless libc requires registration. |
| `clone3` | 435 | none | Missing | LTP sched/nptl | n/a | task/process | Add only after clone contract is stable. |
| `openat2` | 437 | none | Missing | LTP fs | n/a | path/fs | Add resolver flags after `openat` is correct. |
| `mount_setattr` | 442 | none | Missing | LTP fs_bind | n/a | axfs | Add after real mount API. |
