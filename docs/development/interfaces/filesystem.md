# Filesystem And File Descriptor Interfaces

Read this document when changing fd tables, open files, path resolution,
directory iteration, metadata, mount/umount, dev nodes, or file I/O syscalls.

Also read:

- `../policies/compatibility.md`
- `../policies/errno.md`
- `syscall-inventory.md` when changing syscall handlers or numbers.

## Workloads

- `basic`: `chdir`, `close`, `dup`, `dup2`, `fstat`, `getcwd`, `getdents`,
  `mkdir_`, `mount`, `open`, `openat`, `pipe`, `read`, `umount`, `unlink`,
  `write`.
- `busybox`: `touch`, `cat`, `cut`, `od`, `head`, `tail`, `hexdump`,
  `md5sum`, `sort`, `uniq`, `stat`, `wc`, `more`, `rm`, `mkdir`, `mv`,
  `rmdir`, `cp`, `find`.
- `iozone`: sequential I/O, random read, reverse read, stride read,
  `fwrite/fread`, `pwrite/pread`, `pwritev/preadv`.
- `UnixBench fstime`: small, middle, and big file write/read/copy.
- `lmbench`: `lat_syscall` read/write/open/stat/fstat, `lmdd`, `lat_fs`,
  `bw_file_rd`, `bw_mmap_rd`.
- LTP: prioritize `fs`, `fs_bind`, `fs_perms_simple`, and `fs_readonly`.

## Current ArceOS Surfaces

- `examples/shell/src/uspace.rs`: current syscall dispatcher, `UserProcess`,
  `FdTable`, and `FdEntry`.
- `examples/shell/src/linux_fs/`: Linux ABI/semantic helpers used by
  `uspace.rs`. This is not a new VFS and must not wrap axfs backend
  capabilities. It owns current path normalization, compatibility mount state,
  and statx projection; real filesystem operations still go through existing
  `axfs::api` and `axfs::fops` call sites.
- `axfs::api`: `metadata`, `read_dir`, `create_dir`, `remove_dir`,
  `remove_file`, `rename`, `current_dir`, and `set_current_dir`.
- `axfs::api::File` and `OpenOptions`: open/create/truncate/append plus
  `Read`, `Write`, `Seek`, `flush`, `set_len`, and `metadata`.
- `axfs::api::ReadDir` and `DirEntry`: directory iteration.
- `axfs::root::RootDirectory`: internal mount table with `mount`, mounted-fs
  lookup, and mount-point directory entries.

Known gap: runtime `mount`/`umount` is not exposed as a public syscall-facing
API. `RootDirectory::mount` currently takes `&'static str`, `_umount` is
private, and there is no filesystem factory from Linux mount arguments to
`Arc<dyn VfsOps>`.

## Current `linux_fs` Wrapper Boundary

`examples/shell/src/linux_fs/` is the long-lived context entry for Linux
filesystem ABI semantics in the current shell syscall path. It is not a VFS,
does not own backend file data, and must not duplicate `axfs::api` or
`axfs::fops` capability.

Current module responsibilities:

- `mod.rs`: facade used by `uspace.rs`; keep exports small and syscall-facing.
- `path.rs`: pure path normalization helpers. It must not depend on
  `UserProcess`, `FdTable`, or `axfs` until a unified resolver is designed.
- `mount.rs`: `MountTable` plus narrow compatibility `mount`/`umount2`
  semantics. It records targets only to make the inverse operation meaningful.
- `stat.rs`: stat/statx flag validation and projection. It reports only
  supported fields through `requested_mask & supported_mask`.
- `fd.rs`: reserved destination for the future fd-table and
  open-file-description migration. Do not move fd ownership here without a
  separate OFD plan.
- `types.rs`: shared wrapper types only. Prefer keeping types in the module
  that owns the behavior.

When adding new filesystem syscall behavior:

- Put Linux ABI argument interpretation, flag policy, and errno-order rules in
  `linux_fs` only when the logic is independent of backend capability.
- Keep real filesystem operations in existing `axfs::api` / `axfs::fops` call
  sites until an explicit `axfs` interface exists.
- Do not modify `modules/axfs/**` for shell compatibility behavior unless the
  task is explicitly about exposing a real lower-level filesystem interface.
- If a compatibility path must stay, name the compatibility function or state
  with `compat_` and document the deletion condition in
  `../policies/compatibility.md`.

## Open-File-Description Model

This is a Phase 1A prerequisite. Linux semantics require fd slots and open file
descriptions to be separate.

Example model sketch:

```rust
pub struct FdTable {
    entries: Vec<Option<FdSlot>>,
}

pub struct FdSlot {
    pub fd_flags: FdFlags,
    pub desc: Arc<OpenFileDescription>,
}

pub struct OpenFileDescription {
    pub status_flags: Mutex<OpenStatusFlags>,
    pub offset: Mutex<u64>,
    pub backend: OpenFileBackend,
}
```

Required semantics:

- `FD_CLOEXEC` lives in `FdSlot`.
- `O_APPEND`, `O_NONBLOCK`, and shared file offset live in
  `OpenFileDescription`.
- `dup`, `dup2`, and `dup3` create a new fd slot pointing to the same open file
  description.
- `fork` copies fd slots, but the copied slots point to the same open file
  descriptions.
- `clone(CLONE_FILES)` shares the whole fd table; without `CLONE_FILES`, clone
  copies fd slots according to the process model.
- `execve` closes only slots with `FD_CLOEXEC`.
- Process exit closes all fd slots when the last user thread exits; `wait4`
  only reaps status and must not be required for pipe EOF or descriptor
  release.
- `read`, `write`, `lseek`, and `getdents64` use and update the shared offset.
- `pread`, `pwrite`, `preadv`, and `pwritev` use explicit offsets and do not
  update the shared offset.
- `O_APPEND` writes append atomically with respect to the open file
  description.

## Path Resolution Model

All path-taking syscalls should use one resolver. Ad hoc joins in syscall
handlers should be removed as the resolver becomes available.

Required semantics:

- Null pathname pointer returns `EFAULT`.
- Empty path returns `ENOENT` unless the syscall accepts `AT_EMPTY_PATH`.
- Absolute paths start at the process namespace root and ignore `dirfd`.
- Relative paths start at process cwd for `AT_FDCWD`; otherwise they start at
  the directory referenced by `dirfd`.
- Bad relative `dirfd` returns `EBADF`; non-directory `dirfd` returns
  `ENOTDIR`.
- `.` is ignored. `..` walks to the parent but cannot escape namespace root.
- Trailing slash requires the final object to be a directory, otherwise
  `ENOTDIR`.
- Unknown `AT_*` flags return `EINVAL`.
- `AT_SYMLINK_NOFOLLOW` is accepted for stat-like calls, but has no observable
  effect until symlink objects exist.
- If symlinks are unsupported, symlink creation returns `ENOSYS`; traversal
  returns `ELOOP` or `EOPNOTSUPP` consistently per syscall.
- `AT_EMPTY_PATH` is initially supported only for metadata queries such as
  `fstatat` and `statx`; other uses return `EINVAL`.
- Mount-point crossing is handled by VFS lookup after real runtime mounts
  exist, not by syscall-local path rewriting.

## Runtime Mount Contract

The syscall layer validates Linux ABI arguments, but the real mount table must
belong to `axfs` or an `axfs`-owned module.

Example API sketch:

```rust
pub struct MountRequest<'a> {
    pub source: &'a str,
    pub target: &'a str,
    pub fstype: &'a str,
    pub flags: MountFlags,
    pub data: Option<&'a [u8]>,
}

pub fn mount(request: MountRequest<'_>) -> AxResult<()>;
pub fn umount(target: &str, flags: UmountFlags) -> AxResult<()>;
pub fn is_mount_point(path: &str) -> bool;
```

Required semantics:

- `target` resolves to an existing directory.
- duplicate targets return `EBUSY`.
- unknown `fstype` returns `ENODEV` or `EOPNOTSUPP`.
- unsupported `flags` or incompatible `data` returns `EINVAL` or
  `EOPNOTSUPP`.
- unmount of a non-mounted target returns `EINVAL` or `ENOENT`.
- mounted contents are observable through VFS lookup before deleting
  `linux_fs::mount::MountTable` compatibility state.

## Metadata And Stat

- `stat`, `fstat`, `newfstatat`, and `statx` must be backed by real metadata
  where possible.
- `statx` must report only fields backed by real metadata in its returned mask.
- Missing or unsupported statx fields are omitted, not guessed.
- Current shell `statx` projection lives in `linux_fs::stat` and must keep
  returning `requested_mask & supported_mask`, not the raw user request.
- Device id, inode, mode, nlink, uid/gid, block size, block count, and
  timestamps should become filesystem metadata responsibilities rather than
  syscall-local constants.

## Promotion Gates

- Phase 1A: focused `basic` filesystem/fd subset on RISC-V64 and LoongArch64.
  The current shell path has this baseline for `chdir`, `close`, `dup`,
  `dup2`, `fstat`, `getcwd`, `getdents`, `mkdir_`, `mount`, `open`,
  `openat`, `pipe`, `read`, `umount`, `unlink`, and `write`. Keep it green
  while changing filesystem/fd code.
- Phase 1B: full busybox file commands, iozone functional commands,
  UnixBench `fstime`, selected lmbench filesystem/file tests.
- Phase 1C: real mount/devfs tests, LTP `fs_bind`, LTP `fs_readonly`, and
  mount-related busybox behavior.

Before promoting from Phase 1A to Phase 1B, make a separate OFD/path-resolver
plan. In particular, avoid growing ad hoc fd offset, dirfd, or runtime-library
path rules directly inside `uspace.rs`.
