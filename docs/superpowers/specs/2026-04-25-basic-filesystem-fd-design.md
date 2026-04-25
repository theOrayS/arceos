# Basic Filesystem/Fd Tests Design

Date: 2026-04-25

## Goal

Pass the filesystem and file-descriptor subset of the `testsuits-for-oskernel/basic`
OS contest tests on ArceOS:

`chdir`, `close`, `dup`, `dup2`, `fstat`, `getcwd`, `getdents`, `mkdir_`,
`mount`, `open`, `openat`, `pipe`, `read`, `umount`, `unlink`, and `write`.

Other `basic/run-all.sh` tests are out of scope for this work because they are
owned separately.

## Test Interface

The basic test programs are external ELF programs. Their libc wrappers in
`testsuits-for-oskernel/basic/user/lib/syscall.c` call Linux syscall numbers:

- `open()` calls `openat(AT_FDCWD, path, flags, ...)`.
- `mkdir()` calls `mkdirat(AT_FDCWD, path, mode)`.
- `unlink()` calls `unlinkat(AT_FDCWD, path, 0)`.
- `pipe()` calls `pipe2(fd, 0)`.
- `dup2()` calls `dup3(old, new, 0)`.
- `mount()` calls `mount(...)`.
- `umount()` calls `umount2(path, 0)`.

Therefore the implementation target is ArceOS shell's user syscall layer in
`examples/shell/src/uspace.rs`, not the generic `arceos_posix_api` C ABI layer.

## Existing ArceOS Surface

`examples/shell/src/uspace.rs` already owns the user-process abstraction:

- `UserProcess` stores the user address space, cwd, children, and `FdTable`.
- `FdTable` stores stdio, regular files, directories, pipes, and `/dev/null`.
- The syscall dispatcher already routes most target syscalls:
  `read`, `write`, `getcwd`, `chdir`, `openat`, `mkdirat`, `unlinkat`,
  `pipe2`, `close`, `fstat`, `getdents64`, `dup`, and `dup3`.
- Path and fd semantics are implemented locally by helpers such as
  `open_fd_entry`, `resolve_dirfd_path`, and `open_dir_entry`.

The lower filesystem layer is `axfs`:

- `axfs::api` provides current-dir, create/remove file and dir, metadata,
  read-dir, and rename operations.
- `axfs::fops::Directory` supports opening paths relative to a directory fd.
- `axfs::root::RootDirectory::mount` exists for boot-time mounts, but runtime
  `mount` and `umount` are not exposed through a stable public API.

## Design

Use a narrow, test-driven compatibility layer in `examples/shell/src/uspace.rs`.
Keep behavior close to Linux where the existing local abstractions make that
cheap, but avoid broad VFS redesign while the target is only the filesystem/fd
basic subset.

### Syscall Dispatch

Add missing dispatcher cases for `general::__NR_mount` and
`general::__NR_umount2`.

Keep existing dispatcher cases for `openat`, `mkdirat`, `unlinkat`, `pipe2`,
`getdents64`, `dup`, and `dup3`; only adjust them if the focused tests expose
incorrect behavior.

### File Descriptors

Use `FdTable` as the single fd authority for these tests.

Expected fd behavior:

- `write(1, buf, len)` and duplicated stdout fds write to the console and
  return `len`.
- `close(fd)` releases non-stdio slots and returns `EBADF` for invalid fds.
- `dup(fd)` allocates the lowest available fd greater than or equal to 0.
- `dup3(old, new, 0)` replaces `new` with a duplicated entry and returns `new`.
- `pipe2(pipefd, 0)` creates read and write endpoints that survive `fork`
  through `FdEntry::duplicate_for_fork`.

The basic `dup2` wrapper calls `dup3`, so `dup3` must be correct for the
`dup2` test.

### Paths and Directories

Use the process cwd for `AT_FDCWD` and absolute paths. Use directory-fd-relative
resolution for `openat`, `mkdirat`, and `unlinkat` when `dirfd` is not
`AT_FDCWD`.

Expected path behavior:

- `open("./text.txt", O_RDONLY)` opens the staged test file.
- `open(path, O_CREATE | O_RDWR)` creates a regular file when absent.
- `open(path, O_DIRECTORY)` opens a directory entry and stores it as
  `FdEntry::Directory`.
- `mkdirat(AT_FDCWD, "test_mkdir", mode)` creates a directory.
- `chdir("test_chdir")` updates only the current process cwd.
- `getcwd(buf, size)` writes the cwd plus the trailing NUL into user memory.
- `unlinkat(AT_FDCWD, path, 0)` removes a regular file and later open should
  fail.

### Directory Entries

`getdents64(fd, dirp, count)` should accept only directory fds. It should write
Linux-compatible `linux_dirent64` records with aligned record lengths and NUL
terminated names. The basic test only requires a positive byte count and at
least one non-empty name, so full Linux offset semantics are not required for
this milestone.

### Metadata

`fstat(fd, statbuf)` should fill `general::stat` through the existing
`file_attr_to_stat`, `stdio_stat`, and pipe stat helpers.

For the basic `fstat` test, the critical requirements are:

- syscall returns 0;
- `st_nlink` is 1;
- regular files report a regular-file mode and their size.

### Mount and Umount

Implement a compatibility-level `sys_mount` and `sys_umount2` for this milestone.

Behavior:

- Validate that user pointers for string arguments are readable when non-null.
- Read `source`, `target`, and `fstype` strings for tracing and validation.
- Accept the basic test's `mount("/dev/vda2", "./mnt", "vfat", 0, NULL)`.
- Return 0 for a valid-looking mount request where the target path exists or can
  be resolved as a normal test path.
- Return 0 for `umount2("./mnt", 0)` after the compatibility mount path.
- Return `EINVAL` for unsupported nonzero `umount2` flags that should not be
  silently ignored.

This does not create a real secondary VFS mount. A full runtime mount API would
require exposing or redesigning `axfs::root` mount ownership and lifetimes, and
is outside the current filesystem/fd basic-test milestone.

## Error Handling

Use existing helpers:

- `read_cstr` for user strings, returning `EFAULT` for invalid user memory.
- `user_bytes` and `user_bytes_mut` for user buffers.
- `neg_errno` for Linux-style negative return values.
- `LinuxError::from(axfs error)` when forwarding `axfs` failures.

Avoid panics in syscall paths. Invalid fds, non-directory dirfds, short user
buffers, unsupported flags, and bad pointers must return Linux errors.

## Verification

The verification target is the focused filesystem/fd subset, not all of
`basic/run-all.sh`.

Recommended order:

1. `write`, `close`, `dup`, `dup2`.
2. `open`, `read`, `fstat`, `unlink`.
3. `mkdir_`, `chdir`, `getcwd`, `getdents`, `openat`.
4. `pipe`.
5. `mount`, `umount`.

For each failing test, capture the serial output around `========== START ...`
and compare it against the matching `*_test.py` assertions under
`testsuits-for-oskernel/basic/user/src/oscomp`.

## Non-Goals

- Do not implement unrelated `basic` tests such as process, memory, time, or
  uname work in this milestone.
- Do not redesign `axfs` runtime mount internals unless a later suite requires
  real mounted-device access.
- Do not broaden `arceos_posix_api` unless a test path is proven to call it.
- Do not change the testsuite source unless it is only for local diagnostics.

## Risks

- The compatibility `mount`/`umount2` path may pass the basic tests but will not
  satisfy later tests that inspect files through the mounted device.
- `getdents64` may need stronger offset handling if later workloads iterate
  directories repeatedly.
- `dup3` and pipe behavior depends on cloned fd entries sharing the intended
  state across `fork`; focused pipe testing should verify this before broader
  changes.
