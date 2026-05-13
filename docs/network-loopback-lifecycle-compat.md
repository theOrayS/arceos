# Network loopback and process lifecycle compatibility repairs

## Background

The RV evaluation runs local network workloads through BusyBox shell scripts,
`netserver`, `netperf`, and `iperf`. Before this repair set, the kernel could
build and complete the full RV test pass, but all netperf modes still failed
under both musl and glibc. Diagnostic runs later showed two concrete blockers:

1. loopback sockets did not preserve the shutdown and blocking semantics that
   netperf expects, and
2. a process exit could drop the file descriptor table without closing socket
   entries, leaving stale listener state behind for a later server.

The evaluation script and test images are unchanged. These repairs are generic
socket, timer, signal, wait, and process cleanup semantics; they do not depend
on benchmark names, command lines, port numbers, or special-case test data.

## Root causes

### TCP loopback shutdown state was too weak

`shutdown(SHUT_WR)` and `shutdown(SHUT_RD)` were not propagated with enough
directional state for local loopback TCP traffic. A benchmark can legally close
one direction while continuing to receive on the other. Treating shutdown as a
coarse close loses that half-close behavior and makes request/response style
traffic fail even when both endpoints are local.

### Blocking loopback receive could wait on the wrong source

Loopback packets can arrive through an internal queue instead of through the
normal smoltcp receive path. A receiver that checked the loopback queue only
once before blocking could miss packets delivered after the syscall entered the
wait loop.

### Process teardown did not close socket resources

Process teardown cleared the address space and replaced the file descriptor
table. That removed the process-local references, but it did not call the socket
close path for each live descriptor. Listener table entries could therefore
outlive the process that owned them, and a later server bind/listen could see a
false address-in-use state.

### Timed waits and signal wakeups needed wall-time semantics

The user-space compatibility layer needs POSIX-style timed sleep and signal
interrupt behavior for shell scripts that start and stop background network
servers. The old behavior could leave glibc `sleep(1)` or signal-driven waits
from advancing as expected under the RV evaluator.

## Change

- `shutdown` now preserves the caller's requested direction and applies it to
  TCP loopback streams as read-side, write-side, or full shutdown.
- TCP loopback state tracks both endpoints, per-direction shutdown flags, and
  buffered data so local half-close and request/response traffic can progress.
- UDP loopback blocking receive rechecks the loopback queue during the wait
  loop instead of relying on a one-time precheck.
- Process teardown closes all live file descriptors before replacing the
  descriptor table, which runs the socket close path and releases listener
  state.
- Timed sleep uses wall-clock progress with task yielding, and the shell
  compatibility layer supports the signal/timer wait paths needed by the test
  scripts.

## Behavior preservation

- No evaluator script, image, or testcase is modified.
- No Docker environment is used.
- Nonblocking and timeout behavior remains explicit: unavailable data still
  returns the existing would-block or timeout result.
- Socket cleanup happens through the normal socket close path, not by deleting
  listener entries out of band.
- The implementation is driven by syscall and socket semantics rather than by
  recognizing `netperf`, `iperf`, or fixed ports.

## Validation record

Validation used the unchanged RV evaluator:

```text
timeout 900 ./run-eval.sh rv
```

Result:

- Previous trusted RV result: 134 pass-like / 24 fail-like / 10 skipped.
- Current clean RV result: 142 pass-like / 16 fail-like / 10 skipped.
- Delta: +8 pass-like / -8 fail-like / 0 skipped.
- Groups started and ended: 24 / 24.
- Removed failure markers: `netperf UDP_STREAM`, `netperf TCP_STREAM`,
  `netperf UDP_RR`, and `netperf TCP_RR` under both musl and glibc.
- Remaining network failures: all iperf modes and `netperf TCP_CRR` under both
  musl and glibc.
- No new fail-like markers were added by the clean RV run.
