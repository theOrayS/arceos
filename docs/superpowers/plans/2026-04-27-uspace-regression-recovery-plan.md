# Uspace Regression Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore basic mount/getdents semantics and stop hiding selected suites behind broad autorun skips while preserving the previously working UnixBench completion path.

**Architecture:** Keep Linux ABI compatibility in `arceos_posix_api::uspace`, because the shell now calls that module directly. Add narrow per-process compatibility mount state for the existing basic-suite mount shape, and fix fd classification so read-only directory opens become `FdEntry::Directory`. Autorun should execute suites unless there is a documented subsystem boundary that still justifies skipping.

**Tech Stack:** Rust 2024, ArceOS `arceos_posix_api`, `axfs::fops`, linux raw syscall numbers, existing Docker container `arceos-eval-fix`, workspace-root testsuite wrappers.

---

### Task 1: RED baseline from existing logs

**Files:**
- Read: `output_la.md`
- Read: `output_rv.md`
- Read: `testsuits-for-oskernel/basic/user/src/oscomp/getdents.c`
- Read: `testsuits-for-oskernel/basic/user/src/oscomp/mount.c`
- Read: `testsuits-for-oskernel/basic/user/src/oscomp/umount.c`

- [ ] **Step 1: Confirm failing mount/getdents signatures**

Run: `rg -n "getdents fd:-20|mount return: -38|SKIP: cyclictest|SKIP: iozone|SKIP: iperf|SKIP: netperf" output_la.md output_rv.md`

Expected: matches exist in both RV and LA logs.

- [ ] **Step 2: Confirm test intent**

Run: `rg -n "open\(\"\.\", O_RDONLY\)|mount\(|umount\(" testsuits-for-oskernel/basic/user/src/oscomp/getdents.c testsuits-for-oskernel/basic/user/src/oscomp/mount.c testsuits-for-oskernel/basic/user/src/oscomp/umount.c`

Expected: `getdents.c` opens `.` with `O_RDONLY`; mount cases call `/dev/vda2`, `./mnt`, `vfat`, flags `0`, data `NULL`.

### Task 2: Restore narrow mount/umount syscall handling

**Files:**
- Modify: `api/arceos_posix_api/src/uspace.rs`
- Optional check: `docs/development/interfaces/syscall-inventory.md`

- [ ] **Step 1: Add per-process compatibility state**

Add a `compat_mounts: Mutex<Vec<String>>` field to `UserProcess`, initialize it with `Vec::new()` in `load_program`, copy it in `fork`, and clear it during teardown by process drop semantics.

- [ ] **Step 2: Add syscall dispatch arms**

In `user_syscall`, add:

```rust
general::__NR_mount => sys_mount(
    &process,
    tf.arg0(),
    tf.arg1(),
    tf.arg2(),
    tf.arg3(),
    tf.arg4(),
),
general::__NR_umount2 => sys_umount2(&process, tf.arg0(), tf.arg1()),
```

- [ ] **Step 3: Add minimal handlers**

Implement handlers that copy user strings, validate flags/data before state changes, record the normalized target on success, reject duplicates with `EBUSY`, and reject unmounted targets with `EINVAL`.

The accepted compatibility shape is exactly: source starts with `/dev/`, fstype is `vfat`, flags is `0`, data is `0`, and target is non-empty.

### Task 3: Fix read-only directory open classification

**Files:**
- Modify: `api/arceos_posix_api/src/uspace.rs`

- [ ] **Step 1: Convert successful directory file opens into Directory entries**

In `open_fd_candidates`, after `File::open(path, opts)` succeeds, call `file.get_attr()`. If `attr.is_dir()`, return `open_dir_entry(path)` instead of `FdEntry::File`.

- [ ] **Step 2: Preserve non-directory behavior**

Keep existing shadow-file handling and `EISDIR` fallback unchanged for non-successful `File::open` paths.

### Task 4: Remove broad autorun skips for suites under review

**Files:**
- Modify: `examples/shell/src/cmd.rs`

- [ ] **Step 1: Delete the four broad skip branches**

Remove only these branches from `maybe_run_official_tests`: `cyclictest`, `iozone`, `iperf`, and `netperf`.

- [ ] **Step 2: Keep documented non-target skips**

Do not remove the existing skips for `libcbench`, `glibc libctest`, or `ltp` in this patch.

### Task 5: GREEN verification and logs

**Files:**
- Modify: `api/arceos_posix_api/src/uspace.rs`
- Modify: `modules/axhal/src/lib.rs`
- Write: `output_la.md`
- Write: `output_rv.md`
- Create: `arceos/doc/logs/2026-04-27-uspace-regression-recovery.md`

- [ ] **Step 1: Fix rebase build blocker if encountered**

If RV build reports `E0255` for `init_early` / `init_later`, remove the direct `pub use axplat::init::{init_early, init_later};` from `modules/axhal/src/lib.rs` and keep the wrapper functions that initialize boot argument and CPU count.

- [ ] **Step 2: Fix default terminating signal cleanup if cyclictest exposes it**

If cyclictest reaches hackbench cleanup and stalls after `signaling ... worker threads to terminate`, update `kill/tkill/tgkill` so default terminating signals request process-group exit and wake all target process threads. Preserve `SIG_IGN` and explicit user handlers. If workers remain blocked in pipe waits, track each task's current pipe wait target and let signal delivery notify that specific wait queue; include pending signal/exit in pipe wait conditions.

- [ ] **Step 3: Run RV verification**

Run from workspace root: `QEMU_TIMEOUT=3600s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee output_rv.md`

Expected: basic `mount return: 0`, `umount return: 0`, `getdents fd:` positive; no `SKIP:` for cyclictest/iozone/iperf/netperf.

- [ ] **Step 4: Run LA verification**

Run from workspace root: `QEMU_TIMEOUT=3600s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info | tee output_la.md`

Expected: same acceptance points as RV.

- [ ] **Step 5: Write Chinese development log**

Record date/time, files changed, decisions, RV/LA commands/results, and remaining fork/mapping risks in `arceos/doc/logs/2026-04-27-uspace-regression-recovery.md`.

- [ ] **Step 6: Commit focused changes if requested**

Stage only the runtime files, spec/plan, and development log. Do not stage unrelated untracked files.
