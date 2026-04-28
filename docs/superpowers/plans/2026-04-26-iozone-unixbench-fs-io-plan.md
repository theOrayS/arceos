# Iozone UnixBench Filesystem IO Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add maintainable explicit-offset, vector file I/O, file sync, minimum shared-memory IPC, and alarm-driven timer support needed for the ArceOS shell syscall path so iozone and UnixBench `fstime` can exercise real filesystem behavior.

**Architecture:** Keep syscall dispatch and user-memory copying in `examples/shell/src/uspace.rs`. Keep Linux-visible open-file-description file operations in `examples/shell/src/linux_fs/fd.rs`, using `axfs::fops::File::{read_at,write_at}` for explicit offsets so `pread`, `pwrite`, `preadv`, and `pwritev` do not update the shared OFD offset.

**Tech Stack:** Rust 2024, ArceOS shell example, `axfs::fops::File`, `axerrno::LinuxError`, `linux_raw_sys`, contest testsuite scripts, QEMU via the existing `arceos-eval-fix` Docker container.

---

## File Responsibility Map

- `examples/shell/src/linux_fs/fd.rs`: explicit-offset file operations, file sync compatibility, and small offset arithmetic helper.
- `examples/shell/src/uspace.rs`: syscall arms, shared scalar/vector syscall argument handling, narrow `compat_shm_*` state, and `compat_itimer_real_*` for UnixBench `fstime`.
- `examples/shell/src/cmd.rs`: remove the UnixBench suite skip so filesystem results can run.
- `examples/shell/Cargo.toml`: expose `axalloc` to the uspace feature for `compat_shm_*` physical pages.
- `docs/development/interfaces/syscall-inventory.md`: syscall status updates for `pread64`, `pwrite64`, `preadv`, `pwritev`, `fsync`, `fdatasync`, and SysV shm handlers.
- `docs/development/policies/compatibility.md`: compatibility exit conditions for sync, shm, and `ITIMER_REAL` behavior.
- `doc/logs/2026-04-26-iozone-unixbench-fs-io.md`: Chinese development log with scope, rationale, validation, and risks.

## Tasks

### Task 1: Add explicit-offset backend operations

- [x] Add failing unit tests in `linux_fs/fd.rs` for offset addition overflow and normal advancement.
- [x] Run the focused Rust test command and record that host/x86 `arceos-shell --features uspace` is not a valid test target for this crate.
- [x] Add `advance_explicit_offset`, `pread_file`, and `pwrite_file` helpers using `read_at`/`write_at`.
- [x] Verify via target kernel builds and QEMU workload output instead of the invalid host test target.

### Task 2: Add scalar and vector syscall handlers

- [x] Add common iovec loading and offset validation helpers in `uspace.rs`.
- [x] Add `sys_pwrite64`, `sys_preadv`, and `sys_pwritev`.
- [x] Route syscall numbers 68, 69, and 70 through the new handlers.
- [x] Refactor `sys_readv` and `sys_writev` to use the shared iovec loader without changing their sequential-offset behavior.

### Task 2B: Add sync and private SysV shm compatibility

- [x] Add `sys_fsync` and `sys_fdatasync` over fd-table file sync.
- [x] Add `compat_sync_unsupported_flush` for current synchronous-write backends that lack real fsync.
- [x] Add bounded `compat_shm_*` state for `IPC_PRIVATE`, attach, detach, fork attachment accounting, and `IPC_RMID`.
- [x] Reject keyed shm, explicit attach addresses, and attach flags with explicit errno.
- [x] Remove the UnixBench auto-run skip in the shell command runner.
- [x] Add `compat_itimer_real_*` over `setitimer(ITIMER_REAL)` so `fstime` alarm-driven loops can exit.

### Task 3: Update docs and development log

- [x] Update `syscall-inventory.md` to reflect implemented handlers and explicit-offset behavior.
- [x] Update compatibility policy for sync and shm exits.
- [x] Add the required Chinese development log entry under `arceos/doc/logs/`.

### Task 4: Verify

- [x] Run formatting for touched Rust code.
- [x] Build the RISC-V kernel in `arceos-eval-fix`.
- [x] Run the focused testsuite path far enough to observe iozone and UnixBench filesystem lines, or report the first blocker with logs.
- [x] Repeat the build on LoongArch64.
