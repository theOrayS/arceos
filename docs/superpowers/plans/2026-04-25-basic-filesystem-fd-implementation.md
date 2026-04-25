# Basic Filesystem/Fd Tests Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make ArceOS pass the focused `testsuits-for-oskernel/basic` filesystem and file-descriptor tests: `chdir`, `close`, `dup`, `dup2`, `fstat`, `getcwd`, `getdents`, `mkdir_`, `mount`, `open`, `openat`, `pipe`, `read`, `umount`, `unlink`, and `write`.

**Architecture:** Implement the missing and incorrect behavior in `examples/shell/src/uspace.rs`, because these tests are external ELF programs handled by the shell user-syscall layer. Keep `axfs` unchanged for this milestone; use a per-process compatibility mount set for `mount`/`umount2` rather than adding runtime VFS mount support.

**Tech Stack:** Rust 2024, ArceOS `examples/shell` user process runtime, `linux_raw_sys::general` syscall constants, `axfs` file and directory APIs, QEMU-based OS contest tests.

---

## File Structure

- Modify `arceos/examples/shell/src/uspace.rs`.
  This file owns the external ELF syscall dispatcher, `UserProcess`, `FdTable`, path resolution helpers, pipe endpoints, and the fd-backed filesystem behavior used by the basic tests.
- No `axfs` files are modified in this milestone.
  `axfs` already provides file, directory, metadata, remove, and current-dir primitives; runtime mount internals are intentionally out of scope.
- No testsuite source files are modified.
  The existing `testsuits-for-oskernel/basic/user/src/oscomp/*` programs and `*_test.py` assertions are the acceptance tests.

## Execution Environment

The host machine does not provide the Rust build environment needed for ArceOS.
All Rust, cargo, kernel build, and QEMU make targets must run inside the
long-lived Docker container named `arceos-eval-fix`.

`arceos-eval-fix` already exists and may be stopped. Do not delete it, recreate
it, run `docker rm` against it, or replace it with a temporary `--rm` container.
If it is stopped, start it with:

```bash
docker start arceos-eval-fix
```

Use these conventions throughout the plan:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
docker exec arceos-eval-fix make -C /workspace/arceos kernel-la KERNEL_LOG=info
```

Host-side `git` commands and Python log parsers can run from
`/home/majiaqi/Github/OS_Projects`. Host-side wrapper scripts such as
`./run-testsuite-bench-rv-direct.sh` are also allowed because they enter
`arceos-eval-fix` with `docker exec` for the build and QEMU launch path.

## Focused Result Parser

Use this parser after any captured QEMU serial log. It checks only the filesystem/fd subset and ignores the other `basic` tests owned by the rest of the team.

```bash
python3 - <<'PY'
import json
import subprocess
import sys

log = sys.argv[1] if len(sys.argv) > 1 else "/tmp/arceos-basic-fsfd.log"
subset = [
    "test_chdir",
    "test_close",
    "test_dup",
    "test_dup2",
    "test_fstat",
    "test_getcwd",
    "test_getdents",
    "test_mkdir",
    "test_mount",
    "test_open",
    "test_openat",
    "test_pipe",
    "test_read",
    "test_umount",
    "test_unlink",
    "test_write",
]
raw = subprocess.check_output(
    [
        "python3",
        "testsuits-for-oskernel/basic/user/src/oscomp/test_runner.py",
        log,
    ],
    text=True,
)
results = {item["name"]: item for item in json.loads(raw)}
failed = []
for name in subset:
    item = results.get(name)
    if item is None:
        failed.append((name, "missing result"))
        continue
    status = f'{item["passed"]}/{item["all"]}'
    print(f"{name}: {status}")
    if item["passed"] != item["all"]:
        failed.append((name, status))
if failed:
    print("filesystem/fd subset failures:", failed, file=sys.stderr)
    sys.exit(1)
PY
```

Expected final output: every listed test prints `all/all`, and the command exits 0.

### Task 1: Establish Baseline and Build Safety

**Files:**
- Modify: none
- Test: existing QEMU testsuite image under `testsuits-for-oskernel/sdcard-rv.img` or `sdcard-rv.img.xz`

- [ ] **Step 1: Confirm the current branch and untracked state**

Run:

```bash
git -C arceos status --short
git -C arceos branch --show-current
```

Expected: the only tracked changes are none before implementation starts. Existing untracked files such as `.codex`, `AGENTS.md`, `package-lock.json`, and `package.json` may remain untracked and must not be added unless the user asks.

- [ ] **Step 2: Build the RISC-V testsuite kernel before code changes**

Run:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
```

Expected: build succeeds and produces `/workspace/arceos/kernel-rv` inside the container, visible as `arceos/kernel-rv` on the host-mounted workspace.

- [ ] **Step 3: Capture a baseline RISC-V serial log**

Run from `/home/majiaqi/Github/OS_Projects`:

```bash
QEMU_TIMEOUT=240s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-basic-fsfd-before-rv.log
```

Expected: the log contains `#### OS COMP TEST GROUP START basic ####` and individual `========== START test_... ==========` sections. The command may report failures outside the focused subset.

- [ ] **Step 4: Parse the focused baseline**

Run:

```bash
python3 - <<'PY' /tmp/arceos-basic-fsfd-before-rv.log
import json
import subprocess
import sys

log = sys.argv[1]
subset = [
    "test_chdir", "test_close", "test_dup", "test_dup2", "test_fstat",
    "test_getcwd", "test_getdents", "test_mkdir", "test_mount",
    "test_open", "test_openat", "test_pipe", "test_read", "test_umount",
    "test_unlink", "test_write",
]
raw = subprocess.check_output(
    ["python3", "testsuits-for-oskernel/basic/user/src/oscomp/test_runner.py", log],
    text=True,
)
results = {item["name"]: item for item in json.loads(raw)}
for name in subset:
    item = results.get(name)
    if item is None:
        print(f"{name}: missing")
    else:
        print(f'{name}: {item["passed"]}/{item["all"]}')
PY
```

Expected: this prints the current pass/fail state for the focused subset and gives a baseline for later comparison.

### Task 2: Fix `getcwd` ABI Return Value

**Files:**
- Modify: `arceos/examples/shell/src/uspace.rs`
- Test: `testsuits-for-oskernel/basic/user/src/oscomp/getcwd.c`

- [ ] **Step 1: Verify the current incorrect return expression**

Run:

```bash
rg -n "bytes\\.len\\(\\) as isize" arceos/examples/shell/src/uspace.rs
```

Expected before this task: one match inside `fn sys_getcwd`.

- [ ] **Step 2: Replace `sys_getcwd` with pointer-return semantics**

In `arceos/examples/shell/src/uspace.rs`, replace the body of `fn sys_getcwd` with:

```rust
fn sys_getcwd(process: &UserProcess, buf: usize, size: usize) -> isize {
    let cwd = process.cwd();
    let mut bytes = cwd.into_bytes();
    bytes.push(0);
    if bytes.len() > size {
        return neg_errno(LinuxError::ERANGE);
    }
    let Some(dst) = user_bytes_mut(process, buf, bytes.len(), true) else {
        return neg_errno(LinuxError::EFAULT);
    };
    dst.copy_from_slice(&bytes);
    buf as isize
}
```

- [ ] **Step 3: Build-check the change**

Run:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
```

Expected: build succeeds.

- [ ] **Step 4: Commit the ABI fix**

Run:

```bash
git -C arceos add examples/shell/src/uspace.rs
git -C arceos commit -m "fix: return getcwd buffer pointer"
```

Expected: commit succeeds and contains only `examples/shell/src/uspace.rs`.

### Task 3: Align `close` and `dup3` fd Semantics

**Files:**
- Modify: `arceos/examples/shell/src/uspace.rs`
- Test: `testsuits-for-oskernel/basic/user/src/oscomp/close.c`, `dup.c`, `dup2.c`

- [ ] **Step 1: Verify current stdio-preserving close behavior**

Run:

```bash
rg -n "if fd <= 2" arceos/examples/shell/src/uspace.rs
```

Expected before this task: one match inside `FdTable::close`.

- [ ] **Step 2: Replace `FdTable::close`**

In `impl FdTable`, replace `fn close` with:

```rust
fn close(&mut self, fd: i32) -> Result<(), LinuxError> {
    if !(0..self.entries.len() as i32).contains(&fd) || self.entries[fd as usize].is_none() {
        return Err(LinuxError::EBADF);
    }
    self.entries[fd as usize] = None;
    Ok(())
}
```

- [ ] **Step 3: Replace `FdTable::dup3` with flag validation**

In `impl FdTable`, replace `fn dup3` with:

```rust
fn dup3(&mut self, oldfd: i32, newfd: i32, flags: u32) -> Result<i32, LinuxError> {
    if flags != 0 || oldfd == newfd {
        return Err(LinuxError::EINVAL);
    }
    let entry = self.entry(oldfd)?.duplicate_for_fork()?;
    if newfd < 0 {
        return Err(LinuxError::EBADF);
    }
    let newfd = newfd as usize;
    if self.entries.len() <= newfd {
        self.entries.resize_with(newfd + 1, || None);
    }
    self.entries[newfd] = Some(entry);
    Ok(newfd as i32)
}
```

- [ ] **Step 4: Build-check the fd changes**

Run:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
```

Expected: build succeeds.

- [ ] **Step 5: Commit fd semantics**

Run:

```bash
git -C arceos add examples/shell/src/uspace.rs
git -C arceos commit -m "fix: align fd close and dup3 semantics"
```

Expected: commit succeeds and contains only `examples/shell/src/uspace.rs`.

### Task 4: Add Per-Process Compatibility Mount State

**Files:**
- Modify: `arceos/examples/shell/src/uspace.rs`
- Test: `testsuits-for-oskernel/basic/user/src/oscomp/mount.c`, `umount.c`

- [ ] **Step 1: Add the `compat_mounts` field**

In `struct UserProcess`, add the field after `exec_root`:

```rust
compat_mounts: Mutex<Vec<String>>,
```

The struct section should include:

```rust
struct UserProcess {
    aspace: Mutex<AddrSpace>,
    brk: Mutex<BrkState>,
    fds: Mutex<FdTable>,
    cwd: Mutex<String>,
    exec_root: Mutex<String>,
    compat_mounts: Mutex<Vec<String>>,
    children: Mutex<Vec<ChildTask>>,
    rlimits: Mutex<BTreeMap<u32, UserRlimit>>,
    signal_actions: Mutex<BTreeMap<usize, general::kernel_sigaction>>,
    pid: AtomicI32,
    ppid: i32,
    live_threads: AtomicUsize,
    exit_group_code: AtomicI32,
    exit_code: AtomicI32,
    exit_wait: WaitQueue,
}
```

- [ ] **Step 2: Initialize the field in `load_program`**

In the `Arc::new(UserProcess { ... })` initializer inside `fn load_program`, add:

```rust
compat_mounts: Mutex::new(Vec::new()),
```

The initialization block around cwd and exec root should read:

```rust
fds: Mutex::new(FdTable::new()),
cwd: Mutex::new(cwd.into()),
exec_root: Mutex::new(image.exec_root.clone()),
compat_mounts: Mutex::new(Vec::new()),
children: Mutex::new(Vec::new()),
```

- [ ] **Step 3: Copy the field during `fork`**

In `UserProcess::fork`, add:

```rust
compat_mounts: Mutex::new(self.compat_mounts.lock().clone()),
```

The forked process initialization around cwd and exec root should read:

```rust
fds: Mutex::new(self.fds.lock().fork_copy()?),
cwd: Mutex::new(self.cwd()),
exec_root: Mutex::new(self.exec_root()),
compat_mounts: Mutex::new(self.compat_mounts.lock().clone()),
children: Mutex::new(Vec::new()),
```

- [ ] **Step 4: Clear compatibility mounts during teardown**

In `UserProcess::teardown`, add:

```rust
self.compat_mounts.lock().clear();
```

The function should read:

```rust
fn teardown(&self) {
    self.aspace.lock().clear();
    *self.fds.lock() = FdTable::new();
    self.compat_mounts.lock().clear();
}
```

- [ ] **Step 5: Add helper methods to `impl UserProcess`**

Add these methods inside `impl UserProcess`, after `set_exec_root` and before `teardown`:

```rust
fn normalize_user_path(&self, path: &str) -> Result<String, LinuxError> {
    let cwd = self.cwd();
    normalize_path(cwd.as_str(), path).ok_or(LinuxError::EINVAL)
}

fn add_compat_mount(&self, target: String) -> Result<(), LinuxError> {
    let mut mounts = self.compat_mounts.lock();
    if mounts.iter().any(|mounted| mounted == &target) {
        return Err(LinuxError::EBUSY);
    }
    mounts.push(target);
    Ok(())
}

fn remove_compat_mount(&self, target: &str) -> Result<(), LinuxError> {
    let mut mounts = self.compat_mounts.lock();
    let Some(idx) = mounts.iter().position(|mounted| mounted == target) else {
        return Err(LinuxError::EINVAL);
    };
    mounts.swap_remove(idx);
    Ok(())
}
```

- [ ] **Step 6: Build-check the state-only change**

Run:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
```

Expected: build succeeds. If `LinuxError::EBUSY` is not available in the linked `axerrno` version, replace the duplicate-mount error in `add_compat_mount` with `LinuxError::EINVAL` and keep the plan's state-machine behavior.

### Task 5: Implement `mount` and `umount2` Syscalls

**Files:**
- Modify: `arceos/examples/shell/src/uspace.rs`
- Test: `testsuits-for-oskernel/basic/user/src/oscomp/mount.c`, `umount.c`

- [ ] **Step 1: Add dispatcher entries**

In `fn user_syscall`, add these match arms near the existing filesystem syscall arms:

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

Place them after `general::__NR_unlinkat => ...` and before `general::__NR_pipe2 => ...` so the filesystem calls stay grouped.

- [ ] **Step 2: Add `sys_mount`**

Add this function after `sys_unlinkat` and before `sys_faccessat`:

```rust
fn sys_mount(
    process: &UserProcess,
    source: usize,
    target: usize,
    fstype: usize,
    flags: usize,
    data: usize,
) -> isize {
    if flags != 0 || data != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let source = match read_cstr(process, source) {
        Ok(source) => source,
        Err(err) => return neg_errno(err),
    };
    let target = match read_cstr(process, target) {
        Ok(target) => target,
        Err(err) => return neg_errno(err),
    };
    let fstype = match read_cstr(process, fstype) {
        Ok(fstype) => fstype,
        Err(err) => return neg_errno(err),
    };
    if source.is_empty() || target.is_empty() || fstype != "vfat" {
        return neg_errno(LinuxError::EINVAL);
    }
    let target = match process.normalize_user_path(target.as_str()) {
        Ok(target) => target,
        Err(err) => return neg_errno(err),
    };
    if let Err(err) = open_dir_entry(target.as_str()) {
        return neg_errno(err);
    }
    match process.add_compat_mount(target) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}
```

- [ ] **Step 3: Add `sys_umount2`**

Add this function immediately after `sys_mount`:

```rust
fn sys_umount2(process: &UserProcess, target: usize, flags: usize) -> isize {
    if flags != 0 {
        return neg_errno(LinuxError::EINVAL);
    }
    let target = match read_cstr(process, target) {
        Ok(target) => target,
        Err(err) => return neg_errno(err),
    };
    let target = match process.normalize_user_path(target.as_str()) {
        Ok(target) => target,
        Err(err) => return neg_errno(err),
    };
    match process.remove_compat_mount(target.as_str()) {
        Ok(()) => 0,
        Err(err) => neg_errno(err),
    }
}
```

- [ ] **Step 4: Build-check syscall constants and mount code**

Run:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
```

Expected: build succeeds. If `linux_raw_sys::general` for the target does not expose `__NR_mount` or `__NR_umount2`, confirm the constants in `testsuits-for-oskernel/basic/user/lib/syscall_ids.h` and add local constants near the other syscall handling code instead of changing testsuite sources.

- [ ] **Step 5: Commit compatibility mount support**

Run:

```bash
git -C arceos add examples/shell/src/uspace.rs
git -C arceos commit -m "fix: add compat mount syscalls"
```

Expected: commit succeeds and contains only `examples/shell/src/uspace.rs`.

### Task 6: Verify Directory `O_RDONLY` and Focused Tests

**Files:**
- Modify: none expected
- Test: `testsuits-for-oskernel/basic/user/src/oscomp/getdents.c`, full focused filesystem/fd subset

- [ ] **Step 1: Verify directory open fallback exists**

Run:

```bash
rg -n "LinuxError::EISDIR" arceos/examples/shell/src/uspace.rs
```

Expected: one match inside `open_fd_candidates`, with logic that calls `open_dir_entry(path.as_str())`. This is the behavior required for `open(".", O_RDONLY)` followed by `getdents64`.

- [ ] **Step 2: Build RISC-V and LoongArch kernels**

Run:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
docker exec arceos-eval-fix make -C /workspace/arceos kernel-la KERNEL_LOG=info
```

Expected: both builds succeed.

- [ ] **Step 3: Run RISC-V testsuite and capture log**

Run from `/home/majiaqi/Github/OS_Projects`:

```bash
QEMU_TIMEOUT=240s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-basic-fsfd-after-rv.log
```

Expected: the log contains `#### OS COMP TEST GROUP START basic ####`.

- [ ] **Step 4: Parse RISC-V focused subset**

Run:

```bash
python3 - <<'PY' /tmp/arceos-basic-fsfd-after-rv.log
import json
import subprocess
import sys

log = sys.argv[1]
subset = [
    "test_chdir", "test_close", "test_dup", "test_dup2", "test_fstat",
    "test_getcwd", "test_getdents", "test_mkdir", "test_mount",
    "test_open", "test_openat", "test_pipe", "test_read", "test_umount",
    "test_unlink", "test_write",
]
raw = subprocess.check_output(
    ["python3", "testsuits-for-oskernel/basic/user/src/oscomp/test_runner.py", log],
    text=True,
)
results = {item["name"]: item for item in json.loads(raw)}
failed = []
for name in subset:
    item = results.get(name)
    if item is None:
        failed.append((name, "missing"))
        continue
    status = f'{item["passed"]}/{item["all"]}'
    print(f"{name}: {status}")
    if item["passed"] != item["all"]:
        failed.append((name, status))
if failed:
    print("filesystem/fd subset failures:", failed, file=sys.stderr)
    sys.exit(1)
PY
```

Expected: all focused tests pass.

- [ ] **Step 5: Run LoongArch testsuite and capture log**

Run from `/home/majiaqi/Github/OS_Projects`:

```bash
QEMU_TIMEOUT=240s ARCH=loongarch64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-basic-fsfd-after-la.log
```

Expected: the log contains `#### OS COMP TEST GROUP START basic ####`. If the local container has the documented LoongArch shell-launch issue, run `QEMU_TIMEOUT=240s ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info | tee /tmp/arceos-basic-fsfd-after-la.log` from the host with the evaluation container running.

- [ ] **Step 6: Parse LoongArch focused subset**

Run the same parser from Step 4 with the LoongArch log path:

```bash
python3 - <<'PY' /tmp/arceos-basic-fsfd-after-la.log
import json
import subprocess
import sys

log = sys.argv[1]
subset = [
    "test_chdir", "test_close", "test_dup", "test_dup2", "test_fstat",
    "test_getcwd", "test_getdents", "test_mkdir", "test_mount",
    "test_open", "test_openat", "test_pipe", "test_read", "test_umount",
    "test_unlink", "test_write",
]
raw = subprocess.check_output(
    ["python3", "testsuits-for-oskernel/basic/user/src/oscomp/test_runner.py", log],
    text=True,
)
results = {item["name"]: item for item in json.loads(raw)}
failed = []
for name in subset:
    item = results.get(name)
    if item is None:
        failed.append((name, "missing"))
        continue
    status = f'{item["passed"]}/{item["all"]}'
    print(f"{name}: {status}")
    if item["passed"] != item["all"]:
        failed.append((name, status))
if failed:
    print("filesystem/fd subset failures:", failed, file=sys.stderr)
    sys.exit(1)
PY
```

Expected: all focused tests pass.

- [ ] **Step 7: Commit verification notes if code changed after Task 5**

Run:

```bash
git -C arceos status --short
```

Expected: no tracked modified files. If a small fix was required during verification, commit it with:

```bash
git -C arceos add examples/shell/src/uspace.rs
git -C arceos commit -m "fix: pass basic fs fd subset"
```

### Task 7: Final Review

**Files:**
- Modify: none
- Test: git diff, commit history, focused parser outputs

- [ ] **Step 1: Review final commit stack**

Run:

```bash
git -C arceos log --oneline -5
git -C arceos status --short
```

Expected: recent commits include the three implementation commits, and there are no tracked uncommitted changes.

- [ ] **Step 2: Verify no unrelated files were added**

Run:

```bash
git -C arceos show --stat --oneline --no-renames HEAD
git -C arceos diff --name-only HEAD~3..HEAD
```

Expected: implementation commits modify `examples/shell/src/uspace.rs`; documentation commits modify only `docs/superpowers/...`.

- [ ] **Step 3: Summarize verification evidence**

Prepare a final note with:

```text
RISC-V focused fs/fd subset: all selected tests pass from /tmp/arceos-basic-fsfd-after-rv.log
LoongArch focused fs/fd subset: all selected tests pass from /tmp/arceos-basic-fsfd-after-la.log
Builds run:
- docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
- docker exec arceos-eval-fix make -C /workspace/arceos kernel-la KERNEL_LOG=info
```

Expected: the note names any command that could not be run and why.

## Self-Review

- Spec coverage: `getcwd` pointer return is covered in Task 2; `close` and `dup3` semantics are covered in Task 3; compatibility mount state and syscalls are covered in Tasks 4 and 5; directory `O_RDONLY` for `getdents64` is covered in Task 6; focused RISC-V and LoongArch verification is covered in Task 6.
- Placeholder scan: this plan contains no placeholder tasks, no unnamed files, and no deferred implementation markers.
- Type consistency: all new fields and methods use existing imports already present in `uspace.rs`: `Mutex`, `Vec`, `String`, and `LinuxError`. No new module import is required.
