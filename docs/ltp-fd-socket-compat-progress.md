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

Latest access permission metadata diagnostic summary:

- Run cases: 15
- TPASS: 57
- TFAIL: 1
- TBROK: 5
- TCONF: 17
- TWARN: 2

Compared with the previous synthetic user database and EFAULT diagnostic
window, TPASS increased from 54 to 57 and TFAIL decreased from 4 to 1. TBROK,
TCONF, and TWARN did not increase. The run still reaches the known
`add_ipv6addr` timeout point after the early LTP cases.

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
- `access01`/`access02`/`access03`: `/etc/passwd` and `/etc/group` are exposed
  through the existing read-only synthetic file path so standard `root`,
  `nobody`, and `nogroup` lookups work even though the supplied test images do
  not contain `/etc`. This is a generic compatibility file path, not a
  testcase-specific shortcut.
- `access03`: invalid user pointers now return `EFAULT` instead of allowing the
  kernel address-range helper to panic on overflow. The case now exits 0 and
  reports the expected EFAULT behavior for both root and nobody.
- `access01`: files and directories created by the Linux compatibility layer now
  keep their requested permission bits, and `chmod`/`fchmod` update that metadata.
  This turns the `accessfile_x` executable-file checks and the `accessfile_r`
  X_OK/W_OK denial checks into TPASS results under the bounded RV diagnostic.
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
- The readonly synthetic file path now also supplies a minimal user/group
  database for libc account lookup on images that omit `/etc`.
- User pointer range validation now rejects overflowed ranges before asking the
  address-space helper to validate them, so invalid pointers such as `(void *)-1`
  are converted to `EFAULT`.
- The Linux compatibility process state now records effective uid/gid and a
  path permission overlay for files and directories created through the
  userspace syscall path. `access`/`faccessat` uses the recorded mode bits plus
  parent-directory search permission instead of treating every existing path as
  readable, writable, and executable.
- Bound TCP sockets can report their local address before listen/connect when
  the endpoint was already bound.
- ramfs write/truncate paths reject growth beyond a bounded in-memory file size
  using `StorageFull`.

## Remaining gaps

`accept03` and the EFAULT portion of `access03` still exit 0 in the bounded RV
diagnostic. The remaining early LTP gaps are more specific: `access01` now
passes the newly covered permission-bit checks but still exits 2 because the
LTP harness reports "Test 12 haven't reported results" after a child-user
permission check, `access02` needs symlink support, several acct/device cases
still depend on kernel config or filesystem/device support, and the run still
reaches the known `add_ipv6addr` timeout point. Future fixes should target those
missing kernel or environment capabilities directly without modifying the
evaluator or images.
