# Linux Filesystem Wrapper Design

## Status

Design spec for the first implementation phase. This is a one-off
implementation design record. Long-lived interface rules still belong under
`docs/development/`.

## Goal

Introduce a maintainable Linux filesystem semantics layer for
`examples/shell` without modifying `modules/axfs` or treating the wrapper as a
new VFS.

The wrapper should let `uspace.rs` delegate Linux ABI behavior such as path
normalization, mount compatibility state, and statx projection to focused
modules while continuing to use `axfs::api` and `axfs::fops` as the only
filesystem backend capability providers.

## Non-Goals

- Do not modify `modules/axfs`, `axfs::api`, `axfs::fops`, or
  `axfs::root`.
- Do not implement a second VFS or re-wrap all `axfs` file operations.
- Do not migrate the full `FdTable` or open-file-description model in this
  phase.
- Do not claim real runtime mount support while mounted contents are not
  observable through `axfs`.

## Design Constraints

### Linux ABI Layer, Not VFS

`examples/shell/src/linux_fs` is a Linux ABI and semantics layer. It may own:

- Linux path rules and normalization helpers.
- Linux errno ordering for filesystem-facing syscall decisions.
- Mount and umount compatibility state.
- `statx` projection and supported-mask policy.
- The future fd and open-file-description model.

It must not own:

- Real filesystem traversal or storage.
- File creation, deletion, read, write, seek, or directory iteration backend
  logic beyond calling existing `axfs` surfaces.
- Internal `axfs` mount table behavior.

### Low-Risk First Phase

The first phase migrates only logic without fd ownership risk:

- `compat_mounts` becomes `linux_fs::mount::MountTable`.
- `mount` and `umount2` business validation moves into `mount.rs`.
- Pure path normalization moves into `path.rs`.
- `stat_to_statx` and `statx` supported-mask policy move into `stat.rs`.
- `fd.rs` is created as the future FdTable/OFD migration target, but the full
  table stays in `uspace.rs`.

Dirfd-aware path resolution should not be moved yet because it depends on the
current `FdTable` shape. `path.rs` should start with pure helpers such as
`normalize_path(base, path)` and `resolve_cwd_path(cwd, path)`.

## File Layout

```text
examples/shell/src/linux_fs/
  mod.rs
  types.rs
  path.rs
  mount.rs
  stat.rs
  fd.rs
```

### `mod.rs`

Facade for `uspace.rs`. It should expose only the functions and types that the
syscall layer needs.

Initial exports:

```rust
pub use mount::{MountRequest, MountTable, UmountRequest};
pub use path::{normalize_path, resolve_cwd_path};
pub use stat::{stat_to_statx, STATX_SUPPORTED_MASK};
```

### `types.rs`

Shared request/result types used by the wrapper. It should stay small. If a
type is only used by one module, it belongs in that module instead.

Initial content can be minimal or empty if `mount.rs` and `stat.rs` do not need
shared types yet.

### `path.rs`

Owns pure Linux path helper functions.

Initial responsibilities:

- Normalize `.` and `..`.
- Join relative paths against a supplied base cwd.
- Keep root escape impossible.
- Return `None` for invalid paths that cannot be normalized.

Initial non-responsibilities:

- No dependency on `UserProcess`.
- No dependency on `FdTable` or `FdEntry`.
- No dirfd lookup.
- No `axfs` calls.

### `mount.rs`

Owns mount and umount compatibility semantics outside `axfs`.

Initial public shape:

```rust
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
```

Required first-phase behavior:

- `flags != 0` returns `EINVAL`.
- `data != 0` returns `EOPNOTSUPP`, because the syscall exists but the wrapper
  has no mount-data backend.
- Empty `source`, `target`, or `fstype` returns `EINVAL`.
- Only the current basic-test compatibility shape may succeed:
  `source` under `/dev/`, `fstype == "vfat"`, and target verified by the caller
  as an existing directory.
- Duplicate normalized targets return `EBUSY`.
- `umount2` succeeds only for targets recorded in `MountTable`.
- Unmounted targets return `EINVAL`, matching the current compatibility
  behavior.

Compatibility code must include the standard comment form:

```rust
// compat(basic-fsfd): basic calls mount("/dev/vda2", "./mnt", "vfat", 0, NULL)
// delete-when: block-device backed runtime mount exists above axfs.
```

The wrapper records state only to make the matching `umount2` meaningful. It
must not pretend that mounted contents exist.

### `stat.rs`

Owns Linux stat/statx projection.

Required first-phase behavior:

- `statx` must not echo the caller's requested `mask` as fully supported.
- Return `requested_mask & STATX_SUPPORTED_MASK`.
- If `mask == 0`, return the default supported basic fields.
- Unsupported fields stay zero and are omitted from `stx_mask`.
- Invalid `statx` flags return `EINVAL` before filesystem metadata lookup.

The initial supported fields are the fields already available from
`general::stat` in `uspace.rs`:

- type and mode
- nlink
- uid/gid, if present in the source stat structure
- inode
- size
- blocks
- block size
- device minor/rdev minor where currently available

Timestamps may be copied only if the source stat structure contains meaningful
values. Otherwise, leaving them zero is acceptable, but the mask must not claim
unsupported timestamp fields.

### `fd.rs`

Reserved target for a later FdTable/OFD migration. The file may contain only a
module-level comment in phase one:

```rust
//! Future home for Linux fd-table and open-file-description semantics.
```

Moving `FdTable` requires a separate spec because it affects `dup`, `fork`,
`clone`, `execve`, `read`, `write`, `lseek`, `pread`, `pwrite`, `getdents64`,
`O_APPEND`, and `FD_CLOEXEC`.

## Data Flow

### `mount`

1. `uspace.rs` copies user strings from user memory.
2. `uspace.rs` normalizes the target path through `linux_fs::path`.
3. `uspace.rs` verifies the target is an existing directory using current
   `axfs` calls.
4. `uspace.rs` passes scalar/string values to `MountTable::mount`.
5. `MountTable` validates Linux compatibility rules and records the normalized
   target.
6. `uspace.rs` maps `LinuxError` to negative errno.

### `umount2`

1. `uspace.rs` copies the target string.
2. `uspace.rs` normalizes the target path through `linux_fs::path`.
3. `uspace.rs` calls `MountTable::umount`.
4. `MountTable` removes only a previously recorded target.
5. `uspace.rs` maps `LinuxError` to negative errno.

### `statx`

1. `uspace.rs` validates scalar statx flags through `linux_fs::stat`.
2. Existing fd/path metadata lookup still happens through current `FdTable` and
   `axfs` calls.
3. `linux_fs::stat::stat_to_statx` projects available fields and returns an
   honest supported mask.
4. `uspace.rs` writes the output struct to user memory.

## Error Policy

The wrapper returns `axerrno::LinuxError`. `uspace.rs` remains responsible for
turning that into the syscall return value.

Required validation order for this phase:

1. Validate scalar flags before filesystem mutation.
2. Copy user strings in `uspace.rs` before calling the wrapper.
3. Normalize target paths before mount-table mutation.
4. Verify mount target directory existence before recording mount state.
5. Mutate compatibility state only after all validation succeeds.

## Testing Strategy

Because this phase changes behavior but avoids broad fd movement, tests should
focus on:

- Unit tests for `linux_fs::path::normalize_path`.
- Unit tests for `MountTable` duplicate mount and unmounted `umount2`.
- Unit tests for `stat_to_statx` supported mask.
- RISC-V and LoongArch kernel builds inside `arceos-eval-fix`.
- Focused basic filesystem/fd parser for:
  `chdir`, `close`, `dup`, `dup2`, `fstat`, `getcwd`, `getdents`, `mkdir_`,
  `mount`, `open`, `openat`, `pipe`, `read`, `umount`, `unlink`, and `write`.

Rust and QEMU commands must run inside the existing long-lived
`arceos-eval-fix` container. The container must not be deleted, recreated, or
replaced.

## Acceptance Criteria

- `modules/axfs/**` has no changes.
- `examples/shell/src/linux_fs/` exists with the agreed module layout.
- `uspace.rs` no longer owns `compat_mounts` business logic directly; it owns
  syscall argument copying and calls the wrapper.
- `statx` returns only supported mask bits.
- `mount` compatibility success is explicitly named `compat_*` and bounded to
  the basic-test shape.
- RISC-V and LoongArch kernel builds pass in `arceos-eval-fix`.
- Focused basic filesystem/fd subset still passes on RISC-V. LoongArch focused
  subset should be run when the local runner reaches the basic section; if a
  known unrelated runner issue blocks it, record the exact failure.

## Later Phases

1. Move `FdTable`, `FdEntry`, and open-file-description state into
   `linux_fs::fd`.
2. Add dirfd-aware path resolution once fd ownership boundaries are stable.
3. Replace `compat_basic_mounts` with a real runtime mount bridge when a
   block-device backed mount interface exists above `axfs`.
4. Move more metadata responsibility into the filesystem backend only when the
   team decides to expose it through `axfs`; until then, `stat.rs` must keep
   mask reporting honest.
