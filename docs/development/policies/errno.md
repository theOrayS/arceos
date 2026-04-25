# Errno And User-Memory Policy

Linux compatibility tests often check both errno value and validation order.
This policy applies to the shell/testsuite syscall path and should become
project-wide when other syscall entry paths adopt the same contracts.

## Default Validation Order

1. Validate syscall number and architecture dispatch.
2. Validate scalar flags and impossible argument combinations that do not touch
   user memory.
3. Validate fd, pid, and tid handles before user data buffers for fd/process
   syscalls.
4. Copy input user memory before performing visible state changes.
5. Perform the operation.
6. Copy output user memory last. Roll back state when the syscall contract
   requires atomicity.

## Unsupported Feature Strategy

- A syscall with no dispatcher arm returns `ENOSYS`.
- A known syscall with an unsupported flag, option, or invalid flag combination
  usually returns `EINVAL`.
- A known syscall whose backend capability does not exist returns
  `EOPNOTSUPP`, `ENODEV`, or a more specific backend errno.
- A syscall handler that exists only for future integration must not return `0`
  for unimplemented behavior.
- `statx` unsupported fields are omitted from the returned mask; invalid flags
  still return `EINVAL`.
- `mount` with an unknown filesystem type returns `ENODEV` or `EOPNOTSUPP`;
  unsupported mount flags return `EINVAL` or `EOPNOTSUPP`.
- NUMA and page-pinning calls return explicit unsupported errors until real
  NUMA or pinning state exists.

## High-Frequency Syscall Rules

| Syscall | Validation order | Required errno behavior |
| --- | --- | --- |
| `openat` | flags/mode, pathname pointer/string, dirfd if relative, path resolution, create/open | bad pathname pointer `EFAULT`; unknown flags `EINVAL`; bad relative dirfd `EBADF`; non-directory dirfd `ENOTDIR`; missing path `ENOENT`; unsupported create/symlink/perms `EOPNOTSUPP` or real fs errno. |
| `read` | fd lookup, readability, user buffer if count > 0, backend read | bad fd before bad buffer gives `EBADF`; non-readable fd `EBADF`; directory fd `EISDIR`; bad buffer `EFAULT`. |
| `write` | fd lookup, writability, user buffer if count > 0, backend write | bad fd before bad buffer gives `EBADF`; non-writable fd `EBADF`; bad buffer `EFAULT`; closed pipe should become `EPIPE` plus signal when signal delivery exists. |
| `statx` | flags/mask, pathname unless `AT_EMPTY_PATH`, dirfd/path resolution, metadata query, output buffer | unknown flags `EINVAL`; bad output pointer `EFAULT`; unsupported fields are omitted from returned mask. |
| `mmap` | length, flags, alignment, fd/mode for file mapping, address range, VMA install | length 0 `EINVAL`; unsupported flags `EINVAL` or `EOPNOTSUPP`; bad fd `EBADF`; offset alignment `EINVAL`; permission mismatch `EACCES`; no address space `ENOMEM`. |
| `clone` | flag combination, required stack/tid pointers, memory/fd/process creation, tid writes | unsupported flag combination `EINVAL`; bad `ptid/ctid` pointers `EFAULT`; resource exhaustion `EAGAIN` or `ENOMEM`; do not ignore flags. |
| `wait4/waitid` | option flags, child selection, wait state, output write | unknown options `EINVAL`; no matching child `ECHILD`; `WNOHANG` with no exited child returns 0; bad output pointer `EFAULT` when writing a result. |
| `futex` | aligned user address, op command, op-specific pointers, value check, queue operation | unaligned/null futex address `EINVAL`; bad user address `EFAULT`; unsupported op `ENOSYS` or `EOPNOTSUPP`; value mismatch `EAGAIN`; timeout `ETIMEDOUT`; signal interruption `EINTR`. |

Any syscall-local deviation must be documented next to the handler with the
workload that requires it.
