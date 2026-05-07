# LTP fd and socket compatibility progress

This note records the verified LTP progress for the fd/socket compatibility
fixes in the `refactor/moss_kernel_like` line. The evaluator script and test
images were not modified, and the validation was run directly on the server
without Docker.

## Current RV diagnostic result

Bounded command path:

- `./run-eval.sh rv`
- Timeout window: 600 seconds
- The run reached the known `add_ipv6addr` timeout point after the early LTP
  fd/socket cases.

Latest AF_UNIX diagnostic summary:

- Run cases: 15
- TPASS: 21
- TFAIL: 1
- TBROK: 6
- TCONF: 17
- TWARN: 2

Compared with the previous accept01 diagnostic window, TPASS increased from 20
to 21, TFAIL decreased from 2 to 1, and TBROK decreased from 7 to 6.
Compared with the previous O_PATH/procfs diagnostic window, `accept03` now
exits 0 instead of 32. The TCONF count is higher because the AF_UNIX socket
creation no longer stops the fd-provider sweep early, so LTP reaches and skips
additional fd classes that are still unsupported in this kernel.

## Improved tests and behaviors

- `accept4_01`: close-on-exec fd state is preserved through open, pipe2,
  accept4, fcntl, dup/fork inheritance, and exec cleanup. The previous
  close-on-exec mismatch count is now 0 in the diagnostic run.
- `accept01`: invalid socket address buffer handling now returns `EINVAL` for
  the covered test case, matching the expected LTP result.
- `accept02`: multicast membership state is socket-local and is not copied to
  accepted TCP sockets. The LTP check now reports that the multicast group was
  not copied.
- `accept03`: `accept()` on an `O_PATH` fd now reports `EBADF`, while ordinary
  non-socket fds, directories, pipes, `/dev/zero`, and `/proc/self/maps` keep
  the expected `ENOTSOCK` behavior for this test.
- `accept03`: `socket(AF_UNIX, SOCK_STREAM, 0)` now creates a real local socket
  fd object instead of failing with `EAFNOSUPPORT`, so the case now exits 0 in
  the bounded RV diagnostic.
- `access04`/large sparse access path: ramfs now rejects oversized file growth
  with storage-full behavior instead of aborting the kernel on a very large
  allocation request.

## Lower-level implementation areas

- The userspace compatibility fd table now tracks fd flags such as
  `FD_CLOEXEC` and can represent path-only fds and read-only in-memory files.
- `/proc/self/maps` is provided as a synthetic read-only file backed by process
  address range data, rather than by evaluator-specific hardcoding.
- Socket option state is stored per socket entry and shared only through real
  descriptor duplication/inheritance paths.
- The userspace compatibility fd table can now represent minimal AF_UNIX local
  sockets. The implementation supports creation, fd duplication, close,
  polling, and stat-like fd behavior, while unimplemented communication paths
  fail conservatively instead of pretending a full local socket stack exists.
- Bound TCP sockets can report their local address before listen/connect when
  the endpoint was already bound.
- ramfs write/truncate paths reject growth beyond a bounded in-memory file size
  using `StorageFull`.

## Remaining gaps

`accept03` now exits 0 in the bounded RV diagnostic. The remaining early LTP
gaps are outside the fd/socket fixes covered here: `accept4_01` still records
the unsupported legacy socketcall variant on RISC-V, several access/acct cases
depend on missing users, kernel config, or filesystem/device support, and the
run still reaches the known `add_ipv6addr` timeout point. Future fixes should
target those missing kernel or environment capabilities directly without
modifying the evaluator or images.
