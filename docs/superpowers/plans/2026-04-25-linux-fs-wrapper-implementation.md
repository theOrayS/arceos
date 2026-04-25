# Linux FS Wrapper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a maintainable Linux filesystem semantics wrapper for `examples/shell` without modifying `modules/axfs`.

**Architecture:** `examples/shell/src/uspace.rs` stays responsible for syscall dispatch and user-memory copying. New files under `examples/shell/src/linux_fs/` own Linux ABI semantics for pure path normalization, mount compatibility state, and statx projection. The wrapper calls existing `axfs::api` and `axfs::fops` only through the existing `uspace.rs` call sites; it is not a new VFS and does not reimplement backend filesystem behavior.

**Tech Stack:** Rust 2024, ArceOS `examples/shell`, `axerrno::LinuxError`, `linux_raw_sys::general`, existing `axfs::api` and `axfs::fops`, Docker container `arceos-eval-fix` for builds and QEMU tests.

---

## File Structure

- Create `examples/shell/src/linux_fs/mod.rs`.
  Facade module. It declares submodules and re-exports only the small API used by `uspace.rs`.
- Create `examples/shell/src/linux_fs/types.rs`.
  Shared wrapper types. Phase one keeps it intentionally small.
- Create `examples/shell/src/linux_fs/path.rs`.
  Pure path normalization helpers. No `UserProcess`, no `FdTable`, no `axfs`.
- Create `examples/shell/src/linux_fs/mount.rs`.
  `MountTable` plus compatibility `mount`/`umount2` semantics. It records targets only to make `umount2` meaningful.
- Create `examples/shell/src/linux_fs/stat.rs`.
  `statx` supported-mask constants, flag validation, and `stat_to_statx`.
- Create `examples/shell/src/linux_fs/fd.rs`.
  Reserved target for a later FdTable/OFD migration. Phase one contains only a module comment.
- Modify `examples/shell/src/main.rs`.
  Register the new module under `#[cfg(feature = "uspace")]`.
- Modify `examples/shell/src/uspace.rs`.
  Replace direct `compat_mounts`, path-normalize helper use, and `stat_to_statx` implementation with calls into `linux_fs`. Keep `FdTable` in this file.
- Modify long-lived docs only if implementation reveals a contract mismatch:
  `docs/development/interfaces/filesystem.md`,
  `docs/development/policies/compatibility.md`,
  `docs/development/interfaces/syscall-inventory.md`.

Do not modify any path under `modules/axfs/**`.

## Execution Environment

Rust builds and QEMU runs must use the existing long-lived Docker container:

```bash
docker start arceos-eval-fix
```

Run ArceOS build commands with:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
docker exec arceos-eval-fix make -C /workspace/arceos kernel-la KERNEL_LOG=info
```

Do not delete, recreate, or replace `arceos-eval-fix`.

## Focused Basic Parser

Use this parser after captured QEMU logs:

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
        print(f"{name}: missing")
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

Expected final result: every listed test reports `passed == all`.

### Task 1: Establish Red Checks And Safety Baseline

**Files:**
- Modify: none
- Test: source-contract checks, clean build

- [ ] **Step 1: Confirm current working tree**

Run:

```bash
git status --short
```

Expected: no tracked modifications. Existing untracked `.codex`, `package-lock.json`, and `package.json` may be present and must not be added.

- [ ] **Step 2: Run the structural red check**

Run:

```bash
python3 - <<'PY'
from pathlib import Path

checks = []
checks.append(("linux_fs module exists", Path("examples/shell/src/linux_fs/mod.rs").exists()))
main = Path("examples/shell/src/main.rs").read_text()
uspace = Path("examples/shell/src/uspace.rs").read_text()
checks.append(("main registers linux_fs", "mod linux_fs;" in main))
checks.append(("uspace uses MountTable", "MountTable" in uspace and "compat_mounts: Mutex<Vec<String>>" not in uspace))
checks.append(("statx mask is not echoed", "stx.stx_mask = mask;" not in uspace))
failed = [name for name, ok in checks if not ok]
for name, ok in checks:
    print(f"{'PASS' if ok else 'FAIL'}: {name}")
raise SystemExit(1 if failed else 0)
PY
```

Expected before implementation: the command exits 1 and prints failures for the missing `linux_fs` module and current direct `compat_mounts` ownership.

- [ ] **Step 3: Build current RISC-V kernel before changes**

Run:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
```

Expected: build succeeds. This proves the starting point compiles before the refactor.

### Task 2: Add `linux_fs` Module Skeleton And Pure Path Helpers

**Files:**
- Create: `examples/shell/src/linux_fs/mod.rs`
- Create: `examples/shell/src/linux_fs/types.rs`
- Create: `examples/shell/src/linux_fs/path.rs`
- Create: `examples/shell/src/linux_fs/fd.rs`
- Modify: `examples/shell/src/main.rs`
- Test: source-contract check, RISC-V kernel build

- [ ] **Step 1: Add module declaration in `main.rs`**

Edit `examples/shell/src/main.rs` so the module section reads:

```rust
mod cmd;

#[cfg(feature = "uspace")]
mod linux_fs;

#[cfg(feature = "uspace")]
mod uspace;

#[cfg(feature = "use-ramfs")]
mod ramfs;
```

- [ ] **Step 2: Create `mod.rs`**

Create `examples/shell/src/linux_fs/mod.rs`:

```rust
//! Linux filesystem ABI helpers for the shell userspace syscall path.
//!
//! This module is not a VFS. It owns Linux-facing semantics and delegates real
//! filesystem capability to existing axfs call sites in `uspace.rs`.

pub mod fd;
pub mod mount;
pub mod path;
pub mod stat;
pub mod types;

pub use mount::{MountRequest, MountTable, UmountRequest};
pub use path::{normalize_path, resolve_cwd_path};
pub use stat::{stat_to_statx, statx_accepts_empty_path, validate_statx_flags};
```

- [ ] **Step 3: Create `types.rs`**

Create `examples/shell/src/linux_fs/types.rs`:

```rust
//! Shared Linux filesystem wrapper types.
//!
//! Keep this file small. Types used by only one submodule belong in that
//! submodule.
```

- [ ] **Step 4: Create `fd.rs`**

Create `examples/shell/src/linux_fs/fd.rs`:

```rust
//! Future home for Linux fd-table and open-file-description semantics.
//!
//! Phase one intentionally keeps `FdTable` in `uspace.rs` to avoid fd ownership
//! churn while path, mount, and statx semantics are split out.
```

- [ ] **Step 5: Create `path.rs`**

Create `examples/shell/src/linux_fs/path.rs`:

```rust
use std::string::{String, ToString};
use std::vec::Vec;

pub fn normalize_path(base: &str, path: &str) -> Option<String> {
    let mut parts = Vec::new();
    let input = if path.starts_with('/') {
        path.to_string()
    } else if base == "/" {
        format!("/{path}")
    } else {
        format!("{}/{}", base.trim_end_matches('/'), path)
    };
    for part in input.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    let mut normalized = String::from("/");
    normalized.push_str(&parts.join("/"));
    Some(normalized)
}

pub fn resolve_cwd_path(cwd: &str, path: &str) -> Option<String> {
    normalize_path(cwd, path)
}

#[cfg(test)]
mod tests {
    use super::{normalize_path, resolve_cwd_path};

    #[test]
    fn normalizes_absolute_components() {
        assert_eq!(normalize_path("/", "/a/./b/../c"), Some("/a/c".into()));
    }

    #[test]
    fn joins_relative_path_to_cwd() {
        assert_eq!(resolve_cwd_path("/tmp/test", "a/b"), Some("/tmp/test/a/b".into()));
    }

    #[test]
    fn parent_at_root_stays_at_root() {
        assert_eq!(normalize_path("/", "../../../a"), Some("/a".into()));
    }
}
```

- [ ] **Step 6: Build-check skeleton**

Run:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
```

Expected: build succeeds. No behavior should change yet.

- [ ] **Step 7: Commit skeleton**

Run:

```bash
git add examples/shell/src/main.rs examples/shell/src/linux_fs
git commit -m "refactor: add linux fs wrapper skeleton"
```

Expected: commit contains `main.rs` plus the new `linux_fs` files only.

### Task 3: Move Mount Compatibility State Into `linux_fs::mount`

**Files:**
- Create/Modify: `examples/shell/src/linux_fs/mount.rs`
- Modify: `examples/shell/src/uspace.rs`
- Test: source-contract check, RISC-V kernel build

- [ ] **Step 1: Create `mount.rs` with `MountTable`**

Create `examples/shell/src/linux_fs/mount.rs`:

```rust
use axerrno::LinuxError;
use std::string::{String, ToString};
use std::vec::Vec;

#[derive(Clone, Default)]
pub struct MountTable {
    targets: Vec<String>,
}

pub struct MountRequest<'a> {
    pub source: &'a str,
    pub target: &'a str,
    pub fstype: &'a str,
    pub flags: usize,
    pub data: usize,
}

pub struct UmountRequest<'a> {
    pub target: &'a str,
    pub flags: usize,
}

impl MountTable {
    pub fn new() -> Self {
        Self { targets: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.targets.clear();
    }

    pub fn mount(&mut self, request: MountRequest<'_>) -> Result<(), LinuxError> {
        validate_mount_request(&request)?;
        if self.targets.iter().any(|target| target == request.target) {
            return Err(LinuxError::EBUSY);
        }
        self.targets.push(request.target.to_string());
        Ok(())
    }

    pub fn umount(&mut self, request: UmountRequest<'_>) -> Result<(), LinuxError> {
        if request.flags != 0 {
            return Err(LinuxError::EINVAL);
        }
        if request.target.is_empty() {
            return Err(LinuxError::EINVAL);
        }
        let Some(idx) = self.targets.iter().position(|target| target == request.target) else {
            return Err(LinuxError::EINVAL);
        };
        self.targets.swap_remove(idx);
        Ok(())
    }
}

fn validate_mount_request(request: &MountRequest<'_>) -> Result<(), LinuxError> {
    if request.flags != 0 {
        return Err(LinuxError::EINVAL);
    }
    if request.data != 0 {
        return Err(LinuxError::EOPNOTSUPP);
    }
    if request.source.is_empty() || request.target.is_empty() || request.fstype.is_empty() {
        return Err(LinuxError::EINVAL);
    }
    compat_basic_mount(request.source, request.fstype)
}

fn compat_basic_mount(source: &str, fstype: &str) -> Result<(), LinuxError> {
    // compat(basic-fsfd): basic calls mount("/dev/vda2", "./mnt", "vfat", 0, NULL).
    // delete-when: block-device backed runtime mount exists above axfs.
    if fstype != "vfat" {
        return Err(LinuxError::EOPNOTSUPP);
    }
    if !source.starts_with("/dev/") {
        return Err(LinuxError::ENOENT);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MountRequest, MountTable, UmountRequest};
    use axerrno::LinuxError;

    #[test]
    fn duplicate_mount_returns_ebusy() {
        let mut table = MountTable::new();
        let request = MountRequest {
            source: "/dev/vda2",
            target: "/mnt",
            fstype: "vfat",
            flags: 0,
            data: 0,
        };
        assert_eq!(table.mount(request).err(), None);
        let request = MountRequest {
            source: "/dev/vda2",
            target: "/mnt",
            fstype: "vfat",
            flags: 0,
            data: 0,
        };
        assert_eq!(table.mount(request).err(), Some(LinuxError::EBUSY));
    }

    #[test]
    fn unmounted_target_returns_einval() {
        let mut table = MountTable::new();
        let request = UmountRequest {
            target: "/mnt",
            flags: 0,
        };
        assert_eq!(table.umount(request).err(), Some(LinuxError::EINVAL));
    }

    #[test]
    fn mount_data_is_not_fake_success() {
        let mut table = MountTable::new();
        let request = MountRequest {
            source: "/dev/vda2",
            target: "/mnt",
            fstype: "vfat",
            flags: 0,
            data: 1,
        };
        assert_eq!(table.mount(request).err(), Some(LinuxError::EOPNOTSUPP));
    }
}
```

- [ ] **Step 2: Replace `UserProcess` mount field**

In `examples/shell/src/uspace.rs`, change:

```rust
compat_mounts: Mutex<Vec<String>>,
```

to:

```rust
mount_table: Mutex<crate::linux_fs::MountTable>,
```

Update every initializer:

```rust
mount_table: Mutex::new(crate::linux_fs::MountTable::new()),
```

Update fork:

```rust
mount_table: Mutex::new(self.mount_table.lock().clone()),
```

Update teardown:

```rust
self.mount_table.lock().clear();
```

- [ ] **Step 3: Remove `add_compat_mount` and `remove_compat_mount`**

Delete these methods from `impl UserProcess`:

```rust
fn add_compat_mount(&self, target: String) -> Result<(), LinuxError> { ... }
fn remove_compat_mount(&self, target: &str) -> Result<(), LinuxError> { ... }
```

Keep `normalize_user_path` for now. It moves to `linux_fs::path` in Task 4.

- [ ] **Step 4: Rewrite `sys_mount` to call `MountTable`**

Replace the final mount-state mutation in `sys_mount` with:

```rust
let request = crate::linux_fs::MountRequest {
    source: source.as_str(),
    target: target.as_str(),
    fstype: fstype.as_str(),
    flags,
    data,
};
match process.mount_table.lock().mount(request) {
    Ok(()) => 0,
    Err(err) => neg_errno(err),
}
```

The target directory check with `open_dir_entry(target.as_str())` must remain in `uspace.rs` before calling `MountTable::mount`.

- [ ] **Step 5: Rewrite `sys_umount2` to call `MountTable`**

Replace the final `remove_compat_mount` call with:

```rust
let request = crate::linux_fs::UmountRequest {
    target: target.as_str(),
    flags,
};
match process.mount_table.lock().umount(request) {
    Ok(()) => 0,
    Err(err) => neg_errno(err),
}
```

- [ ] **Step 6: Run mount source-contract check**

Run:

```bash
python3 - <<'PY'
from pathlib import Path
uspace = Path("examples/shell/src/uspace.rs").read_text()
mount = Path("examples/shell/src/linux_fs/mount.rs").read_text()
assert "compat_mounts: Mutex<Vec<String>>" not in uspace
assert "fn add_compat_mount" not in uspace
assert "fn remove_compat_mount" not in uspace
assert "MountTable" in uspace
assert "compat_basic_mount" in mount
assert "delete-when: block-device backed runtime mount exists above axfs" in mount
print("mount wrapper contract ok")
PY
```

Expected: prints `mount wrapper contract ok`.

- [ ] **Step 7: Build-check mount migration**

Run:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
```

Expected: build succeeds.

- [ ] **Step 8: Commit mount migration**

Run:

```bash
git add examples/shell/src/linux_fs/mount.rs examples/shell/src/uspace.rs
git commit -m "refactor: move compat mount state to linux fs wrapper"
```

Expected: commit does not include `modules/axfs/**`.

### Task 4: Move Pure Path Normalization Calls Into `linux_fs::path`

**Files:**
- Modify: `examples/shell/src/linux_fs/path.rs`
- Modify: `examples/shell/src/uspace.rs`
- Test: source-contract check, RISC-V kernel build

- [ ] **Step 1: Replace `normalize_user_path` body**

In `impl UserProcess`, change `normalize_user_path` to:

```rust
fn normalize_user_path(&self, path: &str) -> Result<String, LinuxError> {
    let cwd = self.cwd();
    crate::linux_fs::resolve_cwd_path(cwd.as_str(), path).ok_or(LinuxError::EINVAL)
}
```

- [ ] **Step 2: Replace remaining calls to local `normalize_path` where no dirfd is involved**

For helper functions that only join `cwd` with a user path, call `crate::linux_fs::normalize_path` or `crate::linux_fs::resolve_cwd_path`.

Keep dirfd-dependent logic in `uspace.rs`. In particular, do not move `resolve_at_path`, `open_fd_entry`, or `FdTable` methods in this phase.

- [ ] **Step 3: Remove local `normalize_path` only after all call sites are migrated**

Run:

```bash
rg -n "normalize_path\\(" examples/shell/src/uspace.rs
```

Expected before deletion: only call sites that should be switched to `crate::linux_fs::*`. After switching, remove the local `fn normalize_path(base: &str, path: &str) -> Option<String>`.

- [ ] **Step 4: Run path source-contract check**

Run:

```bash
python3 - <<'PY'
from pathlib import Path
path = Path("examples/shell/src/linux_fs/path.rs").read_text()
uspace = Path("examples/shell/src/uspace.rs").read_text()
assert "pub fn normalize_path" in path
assert "pub fn resolve_cwd_path" in path
assert "fn normalize_path(base: &str, path: &str)" not in uspace
assert "crate::linux_fs::resolve_cwd_path" in uspace
print("path wrapper contract ok")
PY
```

Expected: prints `path wrapper contract ok`.

- [ ] **Step 5: Build-check path migration**

Run:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
```

Expected: build succeeds.

- [ ] **Step 6: Commit path migration**

Run:

```bash
git add examples/shell/src/linux_fs/path.rs examples/shell/src/uspace.rs
git commit -m "refactor: move pure path normalization to linux fs wrapper"
```

Expected: commit does not include `modules/axfs/**`.

### Task 5: Move Statx Projection Into `linux_fs::stat`

**Files:**
- Create/Modify: `examples/shell/src/linux_fs/stat.rs`
- Modify: `examples/shell/src/uspace.rs`
- Test: source-contract check, RISC-V kernel build

- [ ] **Step 1: Create `stat.rs`**

Create `examples/shell/src/linux_fs/stat.rs`:

```rust
use axerrno::LinuxError;
use linux_raw_sys::general;

pub const STATX_TYPE: u32 = 0x0001;
pub const STATX_MODE: u32 = 0x0002;
pub const STATX_NLINK: u32 = 0x0004;
pub const STATX_UID: u32 = 0x0008;
pub const STATX_GID: u32 = 0x0010;
pub const STATX_INO: u32 = 0x0100;
pub const STATX_SIZE: u32 = 0x0200;
pub const STATX_BLOCKS: u32 = 0x0400;

pub const STATX_SUPPORTED_MASK: u32 = STATX_TYPE
    | STATX_MODE
    | STATX_NLINK
    | STATX_UID
    | STATX_GID
    | STATX_INO
    | STATX_SIZE
    | STATX_BLOCKS;

const AT_SYMLINK_NOFOLLOW_FLAG: u32 = 0x0100;
const AT_NO_AUTOMOUNT_FLAG: u32 = 0x0800;
const AT_EMPTY_PATH_FLAG: u32 = 0x1000;
const AT_STATX_SYNC_TYPE_MASK: u32 = 0x6000;
const STATX_ALLOWED_FLAGS: u32 =
    AT_SYMLINK_NOFOLLOW_FLAG | AT_NO_AUTOMOUNT_FLAG | AT_EMPTY_PATH_FLAG | AT_STATX_SYNC_TYPE_MASK;

pub fn validate_statx_flags(flags: u32) -> Result<(), LinuxError> {
    if flags & !STATX_ALLOWED_FLAGS != 0 {
        Err(LinuxError::EINVAL)
    } else {
        Ok(())
    }
}

pub const fn statx_accepts_empty_path(flags: u32) -> bool {
    flags & AT_EMPTY_PATH_FLAG != 0
}

pub fn stat_to_statx(st: &general::stat, mask: u32) -> general::statx {
    let mut stx: general::statx = unsafe { core::mem::zeroed() };
    let requested = if mask == 0 { STATX_SUPPORTED_MASK } else { mask };
    stx.stx_mask = requested & STATX_SUPPORTED_MASK;
    stx.stx_blksize = st.st_blksize as _;
    stx.stx_nlink = st.st_nlink as _;
    stx.stx_uid = st.st_uid as _;
    stx.stx_gid = st.st_gid as _;
    stx.stx_mode = st.st_mode as _;
    stx.stx_ino = st.st_ino as _;
    stx.stx_size = st.st_size as _;
    stx.stx_blocks = st.st_blocks as _;
    stx.stx_dev_minor = st.st_dev as _;
    stx.stx_rdev_minor = st.st_rdev as _;
    stx
}

#[cfg(test)]
mod tests {
    use super::{stat_to_statx, validate_statx_flags, STATX_MODE, STATX_SIZE};
    use axerrno::LinuxError;
    use linux_raw_sys::general;

    #[test]
    fn mask_reports_only_supported_fields() {
        let st: general::stat = unsafe { core::mem::zeroed() };
        let stx = stat_to_statx(&st, STATX_MODE | STATX_SIZE | 0x8000_0000);
        assert_eq!(stx.stx_mask, STATX_MODE | STATX_SIZE);
    }

    #[test]
    fn invalid_flags_return_einval() {
        assert_eq!(validate_statx_flags(0x8000_0000).err(), Some(LinuxError::EINVAL));
    }
}
```

- [ ] **Step 2: Update `sys_statx` scalar validation**

In `sys_statx`, after converting `flags` to `u32`, add:

```rust
if let Err(err) = crate::linux_fs::validate_statx_flags(flags) {
    return neg_errno(err);
}
```

For empty pathname handling, replace:

```rust
let st = if path.is_empty() && flags & general::AT_EMPTY_PATH != 0 {
```

with:

```rust
let st = if path.is_empty() {
    if !crate::linux_fs::statx_accepts_empty_path(flags) {
        return neg_errno(LinuxError::ENOENT);
    }
```

Keep the existing fd stat lookup inside that branch.

- [ ] **Step 3: Replace local statx projection call**

Change:

```rust
let stx = stat_to_statx(&st, mask as u32);
```

to:

```rust
let stx = crate::linux_fs::stat_to_statx(&st, mask as u32);
```

Then delete the local `fn stat_to_statx` from `uspace.rs`.

- [ ] **Step 4: Run stat source-contract check**

Run:

```bash
python3 - <<'PY'
from pathlib import Path
stat = Path("examples/shell/src/linux_fs/stat.rs").read_text()
uspace = Path("examples/shell/src/uspace.rs").read_text()
assert "STATX_SUPPORTED_MASK" in stat
assert "stx.stx_mask = requested & STATX_SUPPORTED_MASK;" in stat
assert "pub fn validate_statx_flags" in stat
assert "fn stat_to_statx" not in uspace
assert "crate::linux_fs::stat_to_statx" in uspace
assert "stx.stx_mask = mask;" not in uspace
print("stat wrapper contract ok")
PY
```

Expected: prints `stat wrapper contract ok`.

- [ ] **Step 5: Build-check stat migration**

Run:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
```

Expected: build succeeds.

- [ ] **Step 6: Commit stat migration**

Run:

```bash
git add examples/shell/src/linux_fs/stat.rs examples/shell/src/uspace.rs
git commit -m "refactor: move statx projection to linux fs wrapper"
```

Expected: commit does not include `modules/axfs/**`.

### Task 6: Update Long-Lived Development Docs

**Files:**
- Modify: `docs/development/interfaces/filesystem.md`
- Modify: `docs/development/policies/compatibility.md`
- Modify: `docs/development/interfaces/syscall-inventory.md`
- Test: docs source check

- [ ] **Step 1: Update filesystem interface doc**

In `docs/development/interfaces/filesystem.md`, add this note under "Current ArceOS Surfaces":

```markdown
- `examples/shell/src/linux_fs`: Linux ABI semantics wrapper for the shell
  syscall path. It is not a VFS and must not reimplement `axfs` backend
  behavior.
```

Under "Runtime Mount Contract", replace references to `compat_mounts` with `MountTable`/`compat_basic_mount` wording:

```markdown
Current shell-syscall compromise: until a block-device backed runtime mount
bridge exists above `axfs`, `linux_fs::mount::MountTable` may keep only the
narrow `compat_basic_mount` path needed by the current basic test. Unsupported
flags, data, filesystem types, and unrecorded unmounts must return explicit
errors.
```

- [ ] **Step 2: Update compatibility policy**

In `docs/development/policies/compatibility.md`, replace the `compat_mounts` row with:

```markdown
| `linux_fs::mount::MountTable` / `compat_basic_mount` | block-device backed runtime mount bridge exists above `axfs` | only the basic `mount("/dev/vda2", target, "vfat", 0, NULL)` shape may succeed after target directory verification; duplicate target `EBUSY`; unmounted target `EINVAL`; unsupported flags/data/fs types must not succeed. |
```

- [ ] **Step 3: Update syscall inventory**

In `docs/development/interfaces/syscall-inventory.md`, update the `umount2` next action to:

```markdown
Replace `linux_fs::mount::MountTable` compatibility state with real runtime
unmount once a mount bridge exists above `axfs`.
```

- [ ] **Step 4: Run docs source check**

Run:

```bash
python3 - <<'PY'
from pathlib import Path
fs = Path("docs/development/interfaces/filesystem.md").read_text()
compat = Path("docs/development/policies/compatibility.md").read_text()
atlas = Path("docs/development/interfaces/syscall-inventory.md").read_text()
assert "examples/shell/src/linux_fs" in fs
assert "not a VFS" in fs
assert "compat_basic_mount" in compat
assert "MountTable" in atlas
print("docs wrapper contract ok")
PY
```

Expected: prints `docs wrapper contract ok`.

- [ ] **Step 5: Commit docs update**

Run:

```bash
git add docs/development/interfaces/filesystem.md docs/development/policies/compatibility.md docs/development/interfaces/syscall-inventory.md
git commit -m "docs: route shell fs semantics through linux fs wrapper"
```

Expected: commit contains only docs.

### Task 7: Final Verification

**Files:**
- Modify: none
- Test: formatting, source-contract checks, builds, focused parser

- [ ] **Step 1: Format Rust**

Run:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos fmt
```

Expected: command exits 0.

- [ ] **Step 2: Verify no `axfs` files changed**

Run:

```bash
git diff --name-only HEAD~5..HEAD
```

Expected: no path begins with `modules/axfs/`.

- [ ] **Step 3: Run final structural contract check**

Run:

```bash
python3 - <<'PY'
from pathlib import Path
main = Path("examples/shell/src/main.rs").read_text()
uspace = Path("examples/shell/src/uspace.rs").read_text()
mount = Path("examples/shell/src/linux_fs/mount.rs").read_text()
stat = Path("examples/shell/src/linux_fs/stat.rs").read_text()
assert "mod linux_fs;" in main
assert "mount_table: Mutex<crate::linux_fs::MountTable>" in uspace
assert "compat_mounts: Mutex<Vec<String>>" not in uspace
assert "fn add_compat_mount" not in uspace
assert "fn remove_compat_mount" not in uspace
assert "compat_basic_mount" in mount
assert "delete-when: block-device backed runtime mount exists above axfs" in mount
assert "stx.stx_mask = requested & STATX_SUPPORTED_MASK;" in stat
assert "stx.stx_mask = mask;" not in uspace
print("linux_fs wrapper structural contract ok")
PY
```

Expected: prints `linux_fs wrapper structural contract ok`.

- [ ] **Step 4: Build both evaluation kernels**

Run:

```bash
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
docker exec arceos-eval-fix make -C /workspace/arceos kernel-la KERNEL_LOG=info
```

Expected: both builds exit 0.

- [ ] **Step 5: Run RISC-V focused filesystem/fd tests**

Run from `/home/majiaqi/Github/OS_Projects`:

```bash
QEMU_TIMEOUT=240s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-basic-fsfd-after-rv.log
```

Expected: log reaches at least the full `basic-musl` and `basic-glibc` sections before timeout or normal exit.

- [ ] **Step 6: Parse RISC-V focused results**

Run the focused parser from this plan with `/tmp/arceos-basic-fsfd-after-rv.log`.

Expected: every filesystem/fd subset test prints `passed/all` with equal values.

- [ ] **Step 7: Run LoongArch focused filesystem/fd tests**

Run from `/home/majiaqi/Github/OS_Projects`:

```bash
QEMU_TIMEOUT=240s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info | tee /tmp/arceos-basic-fsfd-after-la.log
```

Expected: if the runner reaches the basic section, parse it with the same focused parser. If the local runner stops before the basic section because of an existing unrelated LoongArch issue, record the exact last visible error and keep the successful `kernel-la` build as the architecture compile check.

- [ ] **Step 8: Review final status**

Run:

```bash
git status --short
git log --oneline -6
```

Expected: no tracked uncommitted changes. Existing unrelated untracked files may remain untracked.

## Self-Review

- Spec coverage: module layout is implemented in Tasks 2-5; no `axfs` changes are guarded in Tasks 1 and 7; path/mount/stat migration boundaries match the spec; fd migration is explicitly deferred to a later spec with `fd.rs` reserved only.
- Completeness scan: this plan uses concrete file paths, commands, code snippets, expected outputs, and commit messages.
- Type consistency: `MountTable`, `MountRequest`, `UmountRequest`, `normalize_path`, `resolve_cwd_path`, `validate_statx_flags`, `statx_accepts_empty_path`, and `stat_to_statx` are defined before use in later tasks.
