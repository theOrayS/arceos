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

Current early diagnostic summary:

- Run cases: 15
- TPASS: 21
- TFAIL: 1
- TBROK: 6
- TCONF: 2
- TWARN: 2

Compared with the previous accept01 diagnostic window, TPASS increased from 20
to 21, TFAIL decreased from 2 to 1, and TBROK decreased from 7 to 6.

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
- Bound TCP sockets can report their local address before listen/connect when
  the endpoint was already bound.
- ramfs write/truncate paths reject growth beyond a bounded in-memory file size
  using `StorageFull`.

## Remaining gap

`accept03` still exits nonzero because the final local socket setup reaches
`socket(1, 1, 0)` and the current stack reports `EAFNOSUPPORT` for AF_UNIX.
The next compatible improvement should add real minimal AF_UNIX socket creation
and conservative socket-state errors, rather than bypassing the test or
special-casing `accept03`.
