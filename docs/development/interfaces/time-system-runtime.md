# Time, System Info, Runtime, And CPU Benchmark Interfaces

Read this document when changing clocks, timers, sleeps, system information,
resource usage, libc/runtime compatibility, language runtime behavior, or CPU
benchmark support.

Also read:

- `../policies/errno.md`
- `process-scheduler.md` for sleep interruption, scheduler, and accounting.
- `filesystem.md` for `df`, `du`, Lua file I/O, and runtime file operations.

## Workloads

- `basic`: `gettimeofday`, `sleep`, `times`, `uname`.
- `busybox`: `date`, `df`, `dmesg`, `du`, `uname`, `uptime`, `ps`, `pwd`,
  `free`, `hwclock`.
- `cyclictest`: timer precision and realtime scheduling latency.
- LTP timer families: `getitimer*`, `setitimer*`, `timerfd_*`,
  `clock_nanosleep*`, `alarm*`, `nanosleep*`.
- `libctest`: static/dynamic libc interface compatibility.
- `libcbench`: libc microbenchmarks and runtime hot paths.
- `lua`: `date.lua`, `file_io.lua`, `random.lua`, `remove.lua`, `sort.lua`,
  `strings.lua`.
- `UnixBench`: `dhry2reg`, `whetstone-double`, `arithoh`, `short`, `int`,
  `long`, `float`, `double`, `hanoi`.

## Required Semantics

- clock syscalls must validate clock ids and user buffers.
- `nanosleep` and `clock_nanosleep` should integrate with signal interruption
  semantics when signals are enabled.
- `times` and `getrusage` should report real process and child accounting once
  process accounting exists.
- system information syscalls and pseudo-files must return stable, documented
  values rather than workload-specific strings.
- libc and Lua failures should be reduced to the underlying syscall family
  before adding compatibility behavior.
- CPU microbenchmarks should not require syscall-specific hacks. They mainly
  depend on correct runtime startup, time measurement, FPU/context handling,
  and scheduler fairness.

## Promotion Gates

- Basic time/system tests pass before cyclictest and LTP timer gates.
- busybox system-info commands complete without fake success paths.
- libcbench/libctest failures are triaged by syscall family.
- Lua file and date tests pass through the filesystem/time interfaces rather
  than Lua-specific compatibility code.
- UnixBench CPU tests emit valid counts; performance tuning is separate from
  functional syscall correctness unless counts are zero/invalid or time out.
