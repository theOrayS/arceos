# ArceOS BusyBox Filesystem Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a maintainable Linux filesystem ABI layer for ArceOS shell syscalls so BusyBox file-management commands use shared path, fd, offset, and stat semantics instead of command-specific patches.

**Architecture:** Keep syscall entry and user-memory copying in `examples/shell/src/uspace.rs`. Move Linux-facing path resolution, fd slot/OFD state, and stat/statx projection into `examples/shell/src/linux_fs/` while real file operations continue through existing `axfs::api` and `axfs::fops` call sites.

**Tech Stack:** Rust 2024, ArceOS shell example, `axfs::api`, `axfs::fops`, `axerrno::LinuxError`, `linux_raw_sys`, QEMU testsuite wrappers inside the existing `arceos-eval-fix` Docker container.

---

## Execution rules for this repository

- Do not modify `modules/axfs/**` in this plan.
- Do not add workload-name, command-name, or fixed-path special cases.
- Do not return success for unsupported state-changing behavior.
- Record every behavior-affecting change in `arceos/doc/logs/`.
- Update `arceos/docs/development/interfaces/syscall-inventory.md` when adding, removing, renaming, or behaviorally changing syscall handlers.
- In this Codex session, do not run tests, validation commands, or git commands unless the user explicitly asks.

## File responsibility map

- `examples/shell/src/linux_fs/path.rs`: Linux path normalization and dirfd-aware resolver helpers that do not own filesystem data.
- `examples/shell/src/linux_fs/fd.rs`: fd flags, open status flags, shared open-file-description state, and reusable file/directory offset helpers.
- `examples/shell/src/linux_fs/stat.rs`: stat/statx flag validation and honest metadata projection.
- `examples/shell/src/linux_fs/mod.rs`: small facade exporting only syscall-facing helpers.
- `examples/shell/src/uspace.rs`: syscall dispatch, user-memory copies, process state, and temporary adapters from `UserProcess`/`FdTable` to `linux_fs`.
- `docs/development/interfaces/filesystem.md`: long-lived filesystem/fd design status.
- `docs/development/interfaces/syscall-inventory.md`: syscall status changes and workload ownership.
- `docs/development/policies/compatibility.md`: any new `compat_*` path and deletion condition.
- `doc/logs/2026-04-26-busybox-filesystem.md`: Chinese development log for this work.

---

### Task 1: Add a dirfd-aware path resolver API

**Files:**

- Modify: `examples/shell/src/linux_fs/path.rs`
- Modify: `examples/shell/src/linux_fs/mod.rs`

- [ ] **Step 1: Add resolver types and tests to `path.rs`**

Insert these public types near the top of `path.rs` after imports:

```rust
use axerrno::LinuxError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolveOptions {
    pub allow_empty: bool,
}

impl ResolveOptions {
    pub const fn default() -> Self {
        Self { allow_empty: false }
    }

    pub const fn allow_empty() -> Self {
        Self { allow_empty: true }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPath {
    pub path: String,
    pub had_trailing_slash: bool,
}
```

Add this resolver below `resolve_cwd_path`:

```rust
pub fn resolve_at_path(
    cwd: &str,
    dirfd_base: Option<&str>,
    path: &str,
    options: ResolveOptions,
) -> Result<ResolvedPath, LinuxError> {
    if path.is_empty() {
        if options.allow_empty {
            return Ok(ResolvedPath {
                path: String::new(),
                had_trailing_slash: false,
            });
        }
        return Err(LinuxError::ENOENT);
    }

    let had_trailing_slash = path.len() > 1 && path.ends_with('/');
    let base = if path.starts_with('/') {
        "/"
    } else {
        dirfd_base.unwrap_or(cwd)
    };
    let Some(path) = normalize_path(base, path) else {
        return Err(LinuxError::EINVAL);
    };
    Ok(ResolvedPath {
        path,
        had_trailing_slash,
    })
}
```

Add these unit tests inside the existing `#[cfg(test)]` module:

```rust
#[test]
fn resolve_empty_path_requires_option() {
    assert_eq!(
        super::resolve_at_path("/", None, "", super::ResolveOptions::default()),
        Err(axerrno::LinuxError::ENOENT)
    );
    assert_eq!(
        super::resolve_at_path("/", None, "", super::ResolveOptions::allow_empty()),
        Ok(super::ResolvedPath {
            path: String::new(),
            had_trailing_slash: false,
        })
    );
}

#[test]
fn resolve_relative_path_uses_dirfd_base_before_cwd() {
    assert_eq!(
        super::resolve_at_path(
            "/cwd",
            Some("/dirfd"),
            "child",
            super::ResolveOptions::default()
        ),
        Ok(super::ResolvedPath {
            path: "/dirfd/child".into(),
            had_trailing_slash: false,
        })
    );
}

#[test]
fn resolve_absolute_path_ignores_dirfd_base() {
    assert_eq!(
        super::resolve_at_path(
            "/cwd",
            Some("/dirfd"),
            "/abs/file",
            super::ResolveOptions::default()
        ),
        Ok(super::ResolvedPath {
            path: "/abs/file".into(),
            had_trailing_slash: false,
        })
    );
}

#[test]
fn resolve_records_trailing_slash() {
    assert_eq!(
        super::resolve_at_path("/", None, "tmp/", super::ResolveOptions::default()),
        Ok(super::ResolvedPath {
            path: "/tmp".into(),
            had_trailing_slash: true,
        })
    );
}
```

- [ ] **Step 2: Export the resolver from `mod.rs`**

Change the path export in `examples/shell/src/linux_fs/mod.rs` to:

```rust
pub use path::{
    ResolveOptions, ResolvedPath, normalize_path, resolve_at_path, resolve_cwd_path,
};
```

- [ ] **Step 3: Verify the local unit test target when the user authorizes testing**

Run from the existing container:

```sh
docker exec arceos-eval-fix make -C /workspace/arceos unittest_no_fail_fast
```

Expected result: the `linux_fs::path` tests pass. Existing unrelated failures, if any, must be reported separately and not hidden.

---

### Task 2: Route existing `resolve_dirfd_path` through the shared resolver

**Files:**

- Modify: `examples/shell/src/uspace.rs`

- [ ] **Step 1: Add a local `AT_FDCWD` constant if the file does not already have an `i32` helper**

Add near the other syscall/path constants:

```rust
const AT_FDCWD_I32: i32 = -100;
```

- [ ] **Step 2: Add a directory fd base helper on `FdTable`**

Inside the existing `impl FdTable` block, add:

```rust
fn dirfd_base_path(&self, dirfd: i32) -> Result<Option<String>, LinuxError> {
    if dirfd == AT_FDCWD_I32 {
        return Ok(None);
    }
    match self.entry(dirfd)? {
        FdEntry::Directory(entry) => Ok(Some(entry.path.clone())),
        _ => Err(LinuxError::ENOTDIR),
    }
}
```

- [ ] **Step 3: Replace the body of `resolve_dirfd_path`**

Use the existing free function name and signature. Replace its internals with:

```rust
fn resolve_dirfd_path(
    process: &UserProcess,
    table: &FdTable,
    dirfd: i32,
    path: &str,
) -> Result<String, LinuxError> {
    let cwd = process.cwd();
    let dirfd_base = table.dirfd_base_path(dirfd)?;
    crate::linux_fs::resolve_at_path(
        cwd.as_str(),
        dirfd_base.as_deref(),
        path,
        crate::linux_fs::ResolveOptions::default(),
    )
    .map(|resolved| resolved.path)
}
```

- [ ] **Step 4: Preserve empty-path behavior for statx**

Add a second helper next to `resolve_dirfd_path`:

```rust
fn resolve_dirfd_path_allow_empty(
    process: &UserProcess,
    table: &FdTable,
    dirfd: i32,
    path: &str,
) -> Result<String, LinuxError> {
    let cwd = process.cwd();
    let dirfd_base = table.dirfd_base_path(dirfd)?;
    crate::linux_fs::resolve_at_path(
        cwd.as_str(),
        dirfd_base.as_deref(),
        path,
        crate::linux_fs::ResolveOptions::allow_empty(),
    )
    .map(|resolved| resolved.path)
}
```

- [ ] **Step 5: Verify path behavior when the user authorizes testing**

Run:

```sh
docker exec arceos-eval-fix make -C /workspace/arceos unittest_no_fail_fast
```

Expected result: path tests pass and the shell example still builds. If the broad unit target is noisy, stop and ask for a narrower command before continuing.

---

### Task 3: Make path-taking syscalls use one resolver path

**Files:**

- Modify: `examples/shell/src/uspace.rs`

- [ ] **Step 1: Update `sys_openat`, `sys_mkdirat`, `sys_unlinkat`, and `sys_renameat2` call paths**

Keep user string copying in the syscall handlers. Ensure the downstream methods receive already resolved absolute paths or delegate through `resolve_dirfd_path`, but do not normalize paths inline in each handler.

Expected handler shape:

```rust
fn sys_mkdirat(process: &UserProcess, dirfd: usize, pathname: usize, _mode: usize) -> isize {
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    let abs_path = {
        let table = process.fds.lock();
        match resolve_dirfd_path(process, &table, dirfd as i32, path.as_str()) {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        }
    };
    match axfs::api::create_dir(abs_path.as_str()) {
        Ok(()) => 0,
        Err(err) => neg_errno(LinuxError::from(err)),
    }
}
```

Apply the same resolver entry point to `unlinkat` and `renameat2`. Keep `AT_REMOVEDIR` validation in `unlinkat`; unsupported unknown flags return `EINVAL`.

- [ ] **Step 2: Update metadata syscalls**

Make `sys_faccessat`, `sys_newfstatat`, `sys_statx`, and `sys_utimensat` use `resolve_dirfd_path` or `resolve_dirfd_path_allow_empty`.

Expected `statx` empty-path branch:

```rust
let st = if path.is_empty() {
    if !crate::linux_fs::statx_accepts_empty_path(flags) {
        return neg_errno(LinuxError::ENOENT);
    }
    match process.fds.lock().stat(dirfd as i32) {
        Ok(st) => st,
        Err(err) => return neg_errno(err),
    }
} else {
    let abs_path = {
        let table = process.fds.lock();
        match resolve_dirfd_path(process, &table, dirfd as i32, path.as_str()) {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        }
    };
    match stat_path_abs(abs_path.as_str()) {
        Ok(st) => st,
        Err(err) => return neg_errno(err),
    }
};
```

If `stat_path_abs` does not exist, create a small free helper in `uspace.rs` that performs the existing absolute-path metadata conversion without dirfd logic.

- [ ] **Step 3: Verify basic filesystem subset when the user authorizes testing**

Run:

```sh
QEMU_TIMEOUT=240s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info
```

Expected result: the existing `basic` filesystem/fd cases remain green on RISC-V64. If the wrapper continues into unrelated suites after basic completes, stop only the QEMU process inside `arceos-eval-fix`.

---

### Task 4: Introduce open-file-description types without changing external syscall behavior

**Files:**

- Modify: `examples/shell/src/linux_fs/fd.rs`
- Modify: `examples/shell/src/linux_fs/mod.rs`

- [ ] **Step 1: Add fd and status flag wrappers**

Replace the placeholder contents of `fd.rs` with concrete shared types:

```rust
//! Linux fd-table and open-file-description helpers.

use axerrno::LinuxError;
use axfs::fops::{Directory, File, FileAttr};
use axsync::Mutex;
use std::string::String;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FdFlags {
    raw: u32,
}

impl FdFlags {
    pub const CLOEXEC: u32 = 0o2000000;

    pub const fn empty() -> Self {
        Self { raw: 0 }
    }

    pub const fn raw(self) -> u32 {
        self.raw
    }

    pub fn set_cloexec(&mut self, enabled: bool) {
        if enabled {
            self.raw |= Self::CLOEXEC;
        } else {
            self.raw &= !Self::CLOEXEC;
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OpenStatusFlags {
    raw: u32,
}

impl OpenStatusFlags {
    pub const APPEND: u32 = 0o2000;
    pub const NONBLOCK: u32 = 0o4000;

    pub const fn from_raw(raw: u32) -> Self {
        Self { raw }
    }

    pub const fn raw(self) -> u32 {
        self.raw
    }

    pub const fn append(self) -> bool {
        self.raw & Self::APPEND != 0
    }
}

pub struct FileBackend {
    pub file: Mutex<File>,
    pub path: String,
}

pub struct DirectoryBackend {
    pub dir: Mutex<Directory>,
    pub attr: FileAttr,
    pub path: String,
}

pub enum OpenFileBackend {
    File(FileBackend),
    Directory(DirectoryBackend),
}

pub struct OpenFileDescription {
    pub status_flags: Mutex<OpenStatusFlags>,
    pub offset: Mutex<u64>,
    pub backend: OpenFileBackend,
}

pub type SharedOpenFileDescription = Arc<OpenFileDescription>;

impl OpenFileDescription {
    pub fn new_file(file: File, path: String, status_flags: OpenStatusFlags) -> Self {
        Self {
            status_flags: Mutex::new(status_flags),
            offset: Mutex::new(0),
            backend: OpenFileBackend::File(FileBackend {
                file: Mutex::new(file),
                path,
            }),
        }
    }

    pub fn new_directory(dir: Directory, attr: FileAttr, path: String) -> Self {
        Self {
            status_flags: Mutex::new(OpenStatusFlags::default()),
            offset: Mutex::new(0),
            backend: OpenFileBackend::Directory(DirectoryBackend {
                dir: Mutex::new(dir),
                attr,
                path,
            }),
        }
    }

    pub fn path(&self) -> &str {
        match &self.backend {
            OpenFileBackend::File(file) => file.path.as_str(),
            OpenFileBackend::Directory(dir) => dir.path.as_str(),
        }
    }

    pub fn as_directory_attr(&self) -> Result<FileAttr, LinuxError> {
        match &self.backend {
            OpenFileBackend::Directory(dir) => Ok(dir.attr),
            OpenFileBackend::File(_) => Err(LinuxError::ENOTDIR),
        }
    }
}
```

- [ ] **Step 2: Export fd types from `linux_fs/mod.rs`**

Add:

```rust
pub use fd::{
    FdFlags, OpenFileBackend, OpenFileDescription, OpenStatusFlags,
    SharedOpenFileDescription,
};
```

- [ ] **Step 3: Verify compile when the user authorizes testing**

Run:

```sh
docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info
```

Expected result: RISC-V kernel build succeeds or reports type errors in only the newly introduced fd helper boundaries.

---

### Task 5: Migrate regular files and directories to shared OFD ownership

**Files:**

- Modify: `examples/shell/src/uspace.rs`

- [ ] **Step 1: Change fd entry shapes**

Change:

```rust
File(FileEntry),
Directory(DirectoryEntry),
```

to:

```rust
File(crate::linux_fs::SharedOpenFileDescription),
Directory(crate::linux_fs::SharedOpenFileDescription),
```

Keep `Stdin`, `Stdout`, `Stderr`, `DevNull`, and `Pipe(PipeEndpoint)` unchanged in this task.

- [ ] **Step 2: Remove clone-only file and directory entry structs from new allocation paths**

Keep the existing `FileEntry` and `DirectoryEntry` structs only until every constructor and match arm has been migrated in this task. Constructors should allocate:

```rust
let desc = Arc::new(crate::linux_fs::OpenFileDescription::new_file(
    file,
    abs_path.clone(),
    crate::linux_fs::OpenStatusFlags::from_raw(flags),
));
FdEntry::File(desc)
```

For directories:

```rust
let desc = Arc::new(crate::linux_fs::OpenFileDescription::new_directory(
    dir,
    attr,
    abs_path.clone(),
));
FdEntry::Directory(desc)
```

- [ ] **Step 3: Make `dup` and `dup3` share the same OFD**

Where `FdEntry` is cloned for `dup` or fork, ensure `Arc::clone(desc)` is used for file and directory descriptions.

Expected match shape:

```rust
let cloned = match entry {
    FdEntry::File(desc) => FdEntry::File(Arc::clone(desc)),
    FdEntry::Directory(desc) => FdEntry::Directory(Arc::clone(desc)),
    FdEntry::Pipe(pipe) => FdEntry::Pipe(pipe.clone()),
    FdEntry::Stdin => FdEntry::Stdin,
    FdEntry::Stdout => FdEntry::Stdout,
    FdEntry::Stderr => FdEntry::Stderr,
    FdEntry::DevNull => FdEntry::DevNull,
};
```

- [ ] **Step 4: Update directory fd path lookup**

Change `dirfd_base_path` to use `desc.path()`:

```rust
match self.entry(dirfd)? {
    FdEntry::Directory(desc) => Ok(Some(desc.path().to_string())),
    _ => Err(LinuxError::ENOTDIR),
}
```

- [ ] **Step 5: Verify fork/dup basic behavior when the user authorizes testing**

Run:

```sh
QEMU_TIMEOUT=240s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info
```

Expected result: `basic` `dup`, `dup2`, `open`, `read`, `write`, and `getdents` remain green on RISC-V64.

---

### Task 6: Move file and directory offset behavior onto OFD

**Files:**

- Modify: `examples/shell/src/linux_fs/fd.rs`
- Modify: `examples/shell/src/uspace.rs`

- [ ] **Step 1: Add OFD read/write/lseek helpers**

Add methods to `impl OpenFileDescription` in `fd.rs`:

```rust
pub fn read_file(&self, dst: &mut [u8]) -> Result<usize, LinuxError> {
    let OpenFileBackend::File(file) = &self.backend else {
        return Err(LinuxError::EISDIR);
    };
    let mut offset = self.offset.lock();
    let mut file = file.file.lock();
    file.seek(axio::SeekFrom::Start(*offset))
        .map_err(LinuxError::from)?;
    let n = file.read(dst).map_err(LinuxError::from)?;
    *offset += n as u64;
    Ok(n)
}

pub fn write_file(&self, src: &[u8]) -> Result<usize, LinuxError> {
    let OpenFileBackend::File(file) = &self.backend else {
        return Err(LinuxError::EBADF);
    };
    let append = self.status_flags.lock().append();
    let mut offset = self.offset.lock();
    let mut file = file.file.lock();
    if append {
        let end = file.seek(axio::SeekFrom::End(0)).map_err(LinuxError::from)?;
        *offset = end;
    } else {
        file.seek(axio::SeekFrom::Start(*offset))
            .map_err(LinuxError::from)?;
    }
    let n = file.write(src).map_err(LinuxError::from)?;
    *offset += n as u64;
    Ok(n)
}

pub fn seek_file(&self, offset: i64, whence: u32) -> Result<u64, LinuxError> {
    let OpenFileBackend::File(file) = &self.backend else {
        return Err(LinuxError::ESPIPE);
    };
    let mut file = file.file.lock();
    let base = match whence {
        0 => axio::SeekFrom::Start(offset as u64),
        1 => axio::SeekFrom::Current(offset),
        2 => axio::SeekFrom::End(offset),
        _ => return Err(LinuxError::EINVAL),
    };
    let new_offset = file.seek(base).map_err(LinuxError::from)?;
    *self.offset.lock() = new_offset;
    Ok(new_offset)
}
```

If `File` methods are provided through `axio::Read`, `axio::Write`, and `axio::Seek`, import those traits at the top of `fd.rs`.

- [ ] **Step 2: Route `FdTable::read`, `FdTable::write`, and `FdTable::lseek` through OFD helpers**

Expected match shape in `uspace.rs`:

```rust
match self.entry(fd)? {
    FdEntry::File(desc) => desc.read_file(dst),
    FdEntry::Directory(_) => Err(LinuxError::EISDIR),
    FdEntry::DevNull => Ok(0),
    FdEntry::Pipe(pipe) => pipe.read(dst),
    FdEntry::Stdin => Ok(0),
    FdEntry::Stdout | FdEntry::Stderr => Err(LinuxError::EBADF),
}
```

Use the analogous write and seek match arms. `lseek` on pipes and stdio should return `ESPIPE` or `EBADF` according to existing behavior and current testsuite expectations.

- [ ] **Step 3: Preserve `pread64` explicit-offset behavior**

Keep `sys_pread64` using an explicit file seek/read path that does not update `desc.offset`.

Expected helper:

```rust
pub fn pread_file(&self, dst: &mut [u8], offset: u64) -> Result<usize, LinuxError> {
    let OpenFileBackend::File(file) = &self.backend else {
        return Err(LinuxError::EISDIR);
    };
    let mut file = file.file.lock();
    file.seek(axio::SeekFrom::Start(offset))
        .map_err(LinuxError::from)?;
    file.read(dst).map_err(LinuxError::from)
}
```

- [ ] **Step 4: Verify BusyBox read-style commands when the user authorizes testing**

Run:

```sh
QEMU_TIMEOUT=240s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info
```

Expected result: BusyBox file commands that read sequentially, such as `cat`, `head`, `tail`, `od`, `hexdump`, `md5sum`, `wc`, `more`, and `grep`, no longer regress due to fd offset behavior.

---

### Task 7: Make directory iteration share OFD state for `find`

**Files:**

- Modify: `examples/shell/src/linux_fs/fd.rs`
- Modify: `examples/shell/src/uspace.rs`

- [ ] **Step 1: Add a directory iterator helper**

Add this helper to `fd.rs`:

```rust
pub fn directory_attr(&self) -> Result<FileAttr, LinuxError> {
    match &self.backend {
        OpenFileBackend::Directory(dir) => Ok(dir.attr),
        OpenFileBackend::File(_) => Err(LinuxError::ENOTDIR),
    }
}
```

Keep the existing low-level `getdents64` record packing code in `uspace.rs` during this task. Move only the directory ownership and offset state, not the Linux dirent layout.

- [ ] **Step 2: Update `FdTable::getdents64` to use shared directory desc**

Expected match shape:

```rust
let FdEntry::Directory(desc) = self.entry(fd)? else {
    return Err(LinuxError::ENOTDIR);
};
let crate::linux_fs::OpenFileBackend::Directory(dir_backend) = &desc.backend else {
    return Err(LinuxError::ENOTDIR);
};
let mut dir = dir_backend.dir.lock();
```

Keep the existing return convention: when there are no more entries, return `Ok(0)`.

- [ ] **Step 3: Verify BusyBox `find` when the user authorizes testing**

Run:

```sh
QEMU_TIMEOUT=240s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info
```

Expected result: `find -name "busybox_cmd.txt"` completes without directory offset loops or duplicate infinite output.

---

### Task 8: Add path `truncate` and keep unsupported fs syscalls explicit

**Files:**

- Modify: `examples/shell/src/uspace.rs`
- Modify: `docs/development/interfaces/syscall-inventory.md`
- Modify: `doc/logs/2026-04-26-busybox-filesystem.md`

- [ ] **Step 1: Add `truncate` syscall constants and dispatcher arm**

Add architecture-safe constants next to nearby filesystem syscall constants:

```rust
#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
const SYS_TRUNCATE: u32 = 45;
#[cfg(not(any(target_arch = "riscv64", target_arch = "loongarch64")))]
const SYS_TRUNCATE: u32 = general::__NR_truncate;
```

Add a dispatcher arm:

```rust
SYS_TRUNCATE => sys_truncate(process, a[0], a[1]),
```

- [ ] **Step 2: Implement `sys_truncate` using the shared resolver**

Add near `sys_ftruncate`:

```rust
fn sys_truncate(process: &UserProcess, pathname: usize, length: usize) -> isize {
    let path = match read_cstr(process, pathname) {
        Ok(path) => path,
        Err(err) => return neg_errno(err),
    };
    let abs_path = {
        let table = process.fds.lock();
        match resolve_dirfd_path(process, &table, AT_FDCWD_I32, path.as_str()) {
            Ok(path) => path,
            Err(err) => return neg_errno(err),
        }
    };
    match fops::OpenOptions::new()
        .write(true)
        .open(abs_path.as_str())
        .and_then(|file| file.set_len(length as u64))
    {
        Ok(()) => 0,
        Err(err) => neg_errno(LinuxError::from(err)),
    }
}
```

- [ ] **Step 3: Do not add fake `statfs`, `fstatfs`, `readlinkat`, `sync`, or `sendfile` success**

Keep missing syscall arms returning `ENOSYS` until a real implementation is needed by observed workload output. If a handler is added for these syscalls, it must either perform real behavior or return an explicit unsupported errno.

Document the decision in `syscall-inventory.md`:

```markdown
| `statfs` | 43 | none | Missing | busybox `df`, LTP | n/a | axfs | Needs real fs stat interface; do not add fake success. |
| `fstatfs` | 44 | none | Missing | busybox, LTP | n/a | axfs | Same as `statfs`. |
| `truncate` | 45 | `sys_truncate` | Real-partial | busybox, iozone, LTP | medium | syscall/fs | Path truncate uses resolver and file `set_len`; permission model remains incomplete. |
```

- [ ] **Step 4: Verify `touch`, shell redirection, and file rewrite commands when the user authorizes testing**

Run:

```sh
QEMU_TIMEOUT=240s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info
```

Expected result: `touch test.txt`, `echo "hello world" > test.txt`, append redirects, `cp`, and `rm` do not fail because `truncate` is missing.

---

### Task 9: Update long-lived documentation and Chinese development log

**Files:**

- Modify: `docs/development/interfaces/filesystem.md`
- Modify: `docs/development/interfaces/syscall-inventory.md`
- Modify: `docs/development/policies/compatibility.md`
- Modify: `doc/logs/2026-04-26-busybox-filesystem.md`

- [ ] **Step 1: Update filesystem interface status**

In `filesystem.md`, record that path resolver and OFD migration started in the shell `linux_fs` layer. State that `modules/axfs/**` remains unchanged in this phase.

Add this note under the `Current linux_fs Wrapper Boundary` section:

```markdown
Current Phase 1B filesystem work migrates dirfd-aware path resolution and
shared open-file-description state into `examples/shell/src/linux_fs/`.
The layer still delegates real file contents and metadata to existing
`axfs::api` and `axfs::fops` call sites.
```

- [ ] **Step 2: Update syscall inventory rows touched by implementation**

Ensure rows for `openat`, `mkdirat`, `unlinkat`, `renameat2`, `read`, `write`, `pread64`, `getdents64`, `lseek`, `faccessat`, `newfstatat`, `statx`, `truncate`, and `ftruncate` match the final handler names and status.

Use `Real-partial` when behavior is real but permissions, symlink, or mount semantics are incomplete.

- [ ] **Step 3: Update compatibility policy only if a new `compat_*` path was added**

If no new compatibility state was introduced, add this sentence to the gate notes or log, not to the compatibility table:

```markdown
No new filesystem `compat_*` state was added for BusyBox file commands.
```

If a new compatibility path was introduced, add a table row with its delete condition and unsupported-state errno behavior.

- [ ] **Step 4: Update the Chinese development log**

Append an entry with:

```markdown
## 2026-04-26 HH:MM CST BusyBox 文件管理语义层迁移

- 范围和目标：将 BusyBox 文件命令依赖的路径解析、fd/OFD offset、metadata 投影收敛到 shell `linux_fs` 语义层。
- 修改文件：列出本次实际修改的源码、文档和日志文件。
- 关键决策：不按 BusyBox 命令名特判；不修改 `modules/axfs/**`；不为 unsupported syscall 返回假成功。
- 验证结果：记录实际执行的 RV/LA 命令和结果；未执行时写明“未执行，等待用户授权”。
- 剩余风险：记录 symlink、权限、statfs、runtime mount/devfs、LTP fs 扩展风险。
```

Replace `HH:MM` with the actual timestamp at implementation time.

---

### Task 10: Cross-architecture verification gate

**Files:**

- Modify: `doc/logs/2026-04-26-busybox-filesystem.md`

- [ ] **Step 1: Run RISC-V64 basic and BusyBox gate when the user authorizes testing**

Run:

```sh
QEMU_TIMEOUT=240s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info
```

Expected result: focused `basic` filesystem/fd cases remain green and BusyBox file commands complete or produce actionable syscall gaps.

- [ ] **Step 2: Run LoongArch64 basic and BusyBox gate when the user authorizes testing**

Run:

```sh
QEMU_TIMEOUT=240s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info
```

Expected result: focused `basic` filesystem/fd cases remain green and BusyBox file commands match RISC-V64 behavior.

- [ ] **Step 3: Parse focused basic results if behavior changed**

Use the repository-mandated parser:

```sh
docker exec arceos-eval-fix python3 /workspace/testsuits-for-oskernel/basic/user/src/oscomp/test_runner.py
```

Expected result: report exact focused subset status rather than relying only on visible QEMU log snippets.

- [ ] **Step 4: Record verification in the Chinese log**

Append exact commands and results. If a wrapper continued beyond target suites and QEMU was stopped manually, record that only the QEMU process was stopped and the `arceos-eval-fix` container was preserved.

---

## Self-review

- Spec coverage: path resolver is covered by Tasks 1-3; OFD/fd offset is covered by Tasks 4-7; BusyBox syscall gaps are covered by Task 8; docs and logs are covered by Task 9; RV/LA validation is covered by Task 10.
- Placeholder scan: the plan contains no `TBD`, no broad "add error handling" step, and no command-name success shortcut.
- Type consistency: resolver types are defined before use; OFD types are defined before `uspace.rs` migration; syscall docs are updated after behavior changes.
- Scope check: the plan stays inside shell syscall ABI and docs, with no `modules/axfs/**` change.
