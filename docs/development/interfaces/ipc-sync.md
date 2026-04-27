# IPC And Synchronization Interfaces

Read this document when changing pipes, futexes, select/poll/epoll readiness,
SysV IPC, eventfd, timerfd, wait queues, or cross-thread wakeups.

Also read:

- `../policies/errno.md`
- `process-scheduler.md` for thread identity, signals, and wait lifecycle.
- `filesystem.md` for pipe file descriptors and fd readiness.
- `memory.md` for shared memory.

## Workloads

- `basic`: `pipe`.
- `lmbench`: `lat_pipe`, `lat_select`.
- LTP: prioritize `ltp/runtest/syscalls-ipc`; syscall families include
  `msg*`, `sem*`, `shm*`, `futex*`, `epoll*`, `select*`, `poll*`, `eventfd*`,
  `timerfd*`.

## Authority Model

- fd readiness belongs to open file descriptions and backend objects.
- blocking/wakeup state belongs to pipe/futex/event/timer objects, not syscall
  stack locals.
- shared memory belongs to the memory subsystem and must be represented as VMA
  shared-memory objects.
- task wait and signal interruption belong to the process/thread subsystem.

## Required Semantics

- Never hold `FdTable`, `MemoryMap`, `AddrSpace`, or process lifecycle locks
  while blocking on pipe, futex, select/poll/epoll, timerfd, eventfd, or SysV
  IPC queues.
- A syscall may look up an `Arc` to the backend object, release high-level
  locks, then block on the backend.
- `pipe2` creates read and write endpoints that share buffer state.
- Empty pipe reads and full pipe writes block on pipe-local wait queues rather
  than busy-yielding inside the syscall path.
- closing the last write endpoint wakes readers; closing the last read endpoint
  makes writers fail with `EPIPE` when signal behavior exists.
- `select/poll/epoll` readiness must consult backend readiness, not just fd
  validity.
- futex wait validates alignment and user memory, checks the user value before
  sleeping, then sleeps on an address-keyed wait queue.
- robust futex behavior on thread exit is required before broad nptl gates.
- SysV message and semaphore syscalls return `ENOSYS` until a real IPC object
  registry exists.

## Promotion Gates

- `basic` pipe passes with correct read/write endpoint behavior.
- lmbench `lat_pipe` and `lat_select` emit valid results and can be rerun.
- LTP IPC cases are promoted one syscall family at a time.
- Unsupported IPC syscall families return explicit errors, not success.
