# Process, Thread, Signal, And Scheduler Interfaces

Read this document when changing process identity, `clone`, `fork`, `vfork`,
`execve`, exit/wait, thread groups, signals, scheduler syscalls, or resource
accounting.

Also read:

- `../policies/errno.md`
- `../policies/compatibility.md` if adding temporary behavior.
- `filesystem.md` when fd inheritance or close-on-exec is involved.
- `memory.md` when clone/fork address-space behavior is involved.
- `ipc-sync.md` when futex, pipe, or wait queues are involved.

## Workloads

- `basic`: `clone`, `execve`, `exit`, `fork`, `getpid`, `getppid`, `sleep`,
  `times`, `wait`, `waitpid`, `yield`.
- `cyclictest`: single-thread and multi-thread realtime latency, `hackbench`.
- `UnixBench`: `context1`, `pipe`, `spawn`, `execl`, `syscall`, `looper` plus
  `multi.sh`.
- `lmbench`: `lat_proc fork/exec/shell`, `lat_ctx`, `lat_pipe`, `lat_select`,
  `lat_sig`.
- LTP: prioritize `sched`, `nptl`, `cpuhotplug`.

## Current ArceOS Surfaces

- `axtask` and `arceos_api`: spawn, exit, wait for exit, sleep, yield, current
  task id, priority, affinity, and wait queues.
- Current shell process logic: child list, wait queue, fork-style construction,
  `execve` image loading, and exit status.

Known gaps:

- Linux `clone` flags are richer than the current model.
- `vfork`, thread groups, robust futex lists, signal delivery, wait-id
  variants, scheduler classes, realtime policy, and resource accounting need
  explicit task-layer contracts.
- eager address-space copy limits UnixBench and lmbench process workloads.

## Process And Thread Contract

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
```

Required semantics:

- `pid` identifies a process leader; `tid` identifies a task/thread.
- `tgid` identifies a thread group.
- thread-group exit and single-thread exit are distinct states.
- parent/child ownership and zombie reaping are explicit, not inferred from
  raw task ids.
- `clone` decides whether memory, fd table, cwd/fs state, and signal handlers
  are shared or copied.
- `execve` closes `FD_CLOEXEC` fds and replaces memory atomically from the
  caller's point of view.

## Minimum Clone Boundary

Until the full process model exists, supported `clone` behavior must be frozen
as a documented subset.

Allowed process-like modes:

- `fork` or `clone` with no sharing flags and exit signal 0 or `SIGCHLD`.
- optional `CLONE_SETTLS`, `CLONE_PARENT_SETTID`, `CLONE_CHILD_SETTID`, and
  `CLONE_CHILD_CLEARTID` only with valid user pointers.
- `CLONE_VFORK | CLONE_VM` only if parent blocking and shared address-space
  lifetime are implemented. Otherwise return `EINVAL`.

Allowed thread-like mode:

- the libc pthread sharing set:
  `CLONE_VM | CLONE_FS | CLONE_FILES | CLONE_SIGHAND | CLONE_SYSVSEM |
  CLONE_THREAD`, plus optional TLS and tid flags.
- child stack must be nonzero.
- the new thread shares process memory and fd table; `tid` differs from `tgid`.

Rejected modes:

- unknown or unmodeled flags return `EINVAL`.
- partial sharing combinations return `EINVAL` until lifecycle semantics are
  defined.
- `clone3` returns `ENOSYS` until `clone` is stable.

## Exit And Wait

- exit transitions `Running -> Zombie`, releases lifecycle locks, wakes
  waiters, handles `clear_child_tid`, then releases task-local resources.
- wait observes `Running -> Zombie -> Reaped`.
- only one waiter can reap a child.
- `WNOHANG` with no exited child returns 0.
- unsupported wait options return `EINVAL`.
- no matching child returns `ECHILD`.

## Scheduler And Time Interaction

- `sched_*` should map to real task-layer priority, affinity, and scheduler
  policy. Unsupported realtime policy returns explicit errno.
- `cyclictest` is a scheduler and timer gate, not a place for syscall-local
  sleep hacks.
- signal interruption semantics for sleep/yield/nanosleep are required before
  broad LTP nptl/sched gates.

## Promotion Gates

- Basic process tests pass before cyclictest, UnixBench, or lmbench process
  gates.
- cyclictest starts with single-thread latency before multi-thread and
  `hackbench`.
- Unsupported clone/scheduler flags return documented errors rather than being
  ignored.
- LTP nptl/sched gates require futex, robust-list, signal, and thread-group
  semantics to be explicit.
