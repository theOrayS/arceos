# ArceOS Development Interface Guide

This directory is the canonical, long-lived development guide for contest
syscall and testsuite work. It is intentionally split by subsystem so a
developer or agent can load only the documents needed for the current task.

The older `docs/superpowers/specs/` documents are historical design records.
When a long-lived rule conflicts with an old spec, this directory wins.

## How To Use This Directory

Start here, then open only the files matching the task:

| Task area | Read |
| --- | --- |
| filesystem, fd, paths, stat, mount, device nodes | `interfaces/filesystem.md` |
| `brk`, `mmap`, VMA, page fault, shared memory | `interfaces/memory.md` |
| `clone`, `fork`, `exec`, `wait`, scheduler, signals | `interfaces/process-scheduler.md` |
| pipe, futex, select/poll/epoll, SysV IPC, eventfd/timerfd | `interfaces/ipc-sync.md` |
| sockets, TCP/UDP, iperf, netperf, LTP networking | `interfaces/network.md` |
| time, system info, libc/runtime, language workloads, CPU benchmarks | `interfaces/time-system-runtime.md` |
| syscall numbers, current handlers, status, owners | `interfaces/syscall-inventory.md` |
| compatibility shims and fake-state exit rules | `policies/compatibility.md` |
| errno policy and user-memory validation order | `policies/errno.md` |

If a change touches more than one subsystem, read each affected subsystem file
and the relevant policy files. Do not load every document by default.

## Current Filesystem/Fd Entry Point

Filesystem and fd syscall work currently enters through
`examples/shell/src/uspace.rs`, with Linux-facing filesystem semantics split
into `examples/shell/src/linux_fs/`. Treat `linux_fs` as the current ABI
semantics layer for the shell syscall path, not as a VFS replacement.

For filesystem/fd tasks, read `interfaces/filesystem.md` first. It records the
current `linux_fs` boundary, the `FdTable`/open-file-description target model,
path-resolution rules, stat/statx projection policy, and runtime mount exit
conditions. Do not expand `uspace.rs` or `linux_fs` across subsystem
boundaries without updating that document.

## Shared Development Rules

- The syscall layer translates Linux ABI details into ArceOS interfaces. It
  must not invent a parallel filesystem, memory manager, task manager, or
  network stack.
- Example Rust structs in these documents are API sketches. Type names, fields,
  and crate boundaries may change. The listed semantics are the binding part.
- Compatibility code must follow `policies/compatibility.md`.
- Unsupported features must follow `policies/errno.md`.
- Any syscall dispatcher or handler change must update
  `interfaces/syscall-inventory.md` in the same commit.
- RISC-V64 and LoongArch64 must stay aligned unless an architecture-specific
  exception is explicitly documented.

## Test Workload Map

### Filesystem And File Descriptors

- `basic`, script `scripts/basic/basic_testcode.sh`: `chdir`, `close`, `dup`,
  `dup2`, `fstat`, `getcwd`, `getdents`, `mkdir_`, `mount`, `open`, `openat`,
  `pipe`, `read`, `umount`, `unlink`, `write`.
- `busybox`, script `scripts/busybox/busybox_testcode.sh`: `touch`, `cat`,
  `cut`, `od`, `head`, `tail`, `hexdump`, `md5sum`, `sort`, `uniq`, `stat`,
  `wc`, `more`, `rm`, `mkdir`, `mv`, `rmdir`, `cp`, `find`.
- `iozone`, script `scripts/iozone/iozone_testcode.sh`: sequential I/O,
  random read, reverse read, stride read, `fwrite/fread`, `pwrite/pread`,
  `pwritev/preadv`.
- `unixbench`, script `scripts/unixbench/unixbench_testcode.sh`: `fstime`
  small, middle, and big file write/read/copy.
- `lmbench`, script `scripts/lmbench/lmbench_testcode.sh`: `lat_syscall`
  read/write/open/stat/fstat, `lmdd`, `lat_fs`, `bw_file_rd`, `bw_mmap_rd`.
- LTP, script `scripts/ltp/ltp_testcode.sh`: prioritize `ltp/runtest/fs`,
  `ltp/runtest/fs_bind`, `ltp/runtest/fs_perms_simple`,
  `ltp/runtest/fs_readonly`.

### Memory Management

- `basic`: `brk`, `mmap`, `munmap`.
- `libcbench`, script `scripts/libcbench/libcbench_testcode.sh`: allocator,
  string, and user memory pressure.
- `lmbench`: `lat_pagefault`, `lat_mmap`, `bw_mmap_rd`.
- LTP: prioritize `ltp/runtest/mm`, `ltp/runtest/hugetlb`,
  `ltp/runtest/numa`; syscall families include `brk*`, `mmap*`, `munmap*`,
  `mremap*`, `madvise*`, `mincore*`, `mlock*`, `munlock*`, `move_pages*`,
  `migrate_pages*`, `shmat*`, `shmctl*`, and `shmget*`.

### Process, Threads, And Scheduling

- `basic`: `clone`, `execve`, `exit`, `fork`, `getpid`, `getppid`, `sleep`,
  `times`, `wait`, `waitpid`, `yield`.
- `cyclictest`, script `scripts/cyclictest/cyclictest_testcode.sh`:
  single-thread and multi-thread realtime latency, `hackbench`.
- `unixbench`: `context1`, `pipe`, `spawn`, `execl`, `syscall`, `looper` plus
  `multi.sh`.
- `lmbench`: `lat_proc fork/exec/shell`, `lat_ctx`, `lat_pipe`, `lat_select`,
  `lat_sig`.
- LTP: prioritize `ltp/runtest/sched`, `ltp/runtest/nptl`,
  `ltp/runtest/cpuhotplug`; syscall families include `clone*`, `fork*`,
  `vfork*`, `execve*`, `wait*`, `waitpid*`, `waitid*`, `sched_*`.

### IPC And Synchronization

- `basic`: `pipe`.
- `lmbench`: `lat_pipe`, `lat_select`.
- LTP: prioritize `ltp/runtest/syscalls-ipc`; syscall families include
  `msg*`, `sem*`, `shm*`, `futex*`, `epoll*`, `select*`, `poll*`, `eventfd*`,
  `timerfd*`.

### Networking

- `iperf`, script `scripts/iperf/iperf_testcode.sh`: TCP/UDP throughput,
  concurrent connections, reverse transfer.
- `netperf`, script `scripts/netperf/netperf_testcode.sh`: `UDP_STREAM`,
  `TCP_STREAM`, `UDP_RR`, `TCP_RR`, `TCP_CRR`.
- LTP networking: prioritize `ltp/runtest/net.ipv6`,
  `ltp/runtest/net.multicast`, `ltp/runtest/net.tcp_cmds`,
  `ltp/runtest/net_stress.*`.

### Time, System Info, Runtime, And CPU Benchmarks

- `basic`: `gettimeofday`, `sleep`, `times`, `uname`.
- `busybox`: `date`, `df`, `dmesg`, `du`, `uname`, `uptime`, `ps`, `pwd`,
  `free`, `hwclock`.
- `libctest`, script `scripts/libctest/libctest_testcode.sh`: libc ABI
  compatibility for static/dynamic linking.
- `libcbench`: libc microbenchmarks and runtime hot paths.
- `lua`, script `scripts/lua/lua_testcode.sh`: `date.lua`, `file_io.lua`,
  `random.lua`, `remove.lua`, `sort.lua`, `strings.lua`.
- `unixbench`: `dhry2reg`, `whetstone-double`, `arithoh`, `short`, `int`,
  `long`, `float`, `double`, `hanoi`.

## Recommended Bring-Up Order

1. Use `basic` to close the minimal syscall loops.
2. For filesystem/fd work, keep the focused `basic` subset green on both
   RISC-V64 and LoongArch64 before broadening scope. Parse the saved QEMU logs
   with `testsuits-for-oskernel/basic/user/src/oscomp/test_runner.py` and
   report the exact filesystem/fd subset result.
3. After filesystem basics stabilize, promote busybox file commands, iozone,
   lmbench filesystem cases, and UnixBench `fstime`.
4. After memory and scheduling mature, promote cyclictest, LTP `mm`, LTP
   `sched`, and LTP IPC.
5. Treat networking as a separate track using iperf, netperf, and LTP network
   runtest entries.
