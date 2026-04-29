# Compatibility Policy

Compatibility code is allowed only as a bridge to a named milestone. It must be
easy to find, bounded, and safe to delete.

## Naming And Comments

Use `compat_` for functions, fields, modules, or data structures that hold fake
or compatibility-only state.

Every compatibility path must include a nearby comment with this shape:

```rust
// compat(<milestone>): <why this exists>
// delete-when: <real interface or test gate that removes it>
```

## Rules

- Compatibility code may emulate a narrow successful path, but it must return
  explicit errors for unsupported states.
- A compatibility path must not claim success for a state-changing operation
  unless it records enough state to make the matching inverse operation
  meaningful.
- Do not add new `Stub-success` syscall behavior after Phase 0.
- Before any broad suite gate, scan for `compat_`, `Stub-success`, and direct
  syscall arms that return `0`; justify every hit in gate notes.
- Compatibility state must not be copied into real subsystem APIs. When the
  real interface exists, delete the compatibility state rather than
  synchronizing both states.
- Do not pass tests by detecting test names, workload names, fixed staging
  paths, or command strings and returning canned success. Test-driven fixes
  must implement the Linux-visible behavior that the test exercises.
- Do not hardcode broad allowlists such as one filesystem type, one device
  path, or one flag combination unless the code is explicitly marked
  `compat_*`, rejects unsupported states with errno, and has a documented
  deletion condition.
- Do not return success for unsupported state-changing operations unless the
  compatibility layer records enough state for later validation and inverse
  operations.

## Filesystem Compatibility Guardrails

- `linux_fs::mount::MountTable` may record targets only for the current shell
  syscall path. It is not an `axfs` mount table and must not be used by lower
  filesystem layers.
- `compat_basic_mount` is allowed only for the narrow basic-suite shape that
  mounts a block device path such as `/dev/vda2` with `fstype == "vfat"` and
  zero flags/data. Unsupported flags, non-null data, unsupported filesystems,
  or invalid sources must return explicit errors.
- A successful compatibility mount must be removable through `umount2`; an
  unmounted target must not return success.
- `linux_fs::stat` may project existing `stat` metadata into `statx`, but it
  must not advertise fields that are not backed by data.

## Current Known Compatibility Exits

| Compatibility path | Delete when | Interim behavior |
| --- | --- | --- |
| `linux_fs::mount::MountTable` / `compat_basic_mount` | runtime `axfs` mount/unmount exposes mounted contents | duplicate mount returns `EBUSY`; unmounted target returns `EINVAL` or `ENOENT`; unsupported flags/data must not succeed. |
| `linux_fs::stat` statx projection from `stat` | filesystem metadata returns statx-capable fields and mask | return only honest fields through `requested_mask & supported_mask`; invalid flags `EINVAL`; bad buffers `EFAULT`. |
| `compat_sync_unsupported_flush` | `axfs` exposes real per-filesystem fsync/writeback capability | `fsync`/`fdatasync` validate fd type and call backend `flush`; only unsupported/default `EINVAL` or `EOPNOTSUPP` flush results are treated as already clean for current synchronous writes. |
| `compat_empty_dir_rename` | `axfs::api::rename` supports directory rename semantics | only after real rename returns unsupported, empty directory rename is emulated with create-destination/remove-source and rollback on remove failure; existing destination and non-directory source return explicit errno. |
| `compat_shm_*` registry | a real SysV shared-memory object model exists in the memory subsystem | supports only `IPC_PRIVATE` segments, `shmat(..., NULL, 0)`, `shmdt`, fork attachment accounting, and `IPC_RMID`; keyed shm, explicit attach addresses, and attach flags return explicit errno. |
| `compat_msync` | `AddrSpace`/mmap tracks file-backed VMAs and exposes real writeback/invalidation semantics | validates page alignment, supported `MS_*` flags, overflow, and mapped pages; current copied file mappings have no dirty shared writeback, so valid mapped ranges are treated as already synchronized. |
| `compat_itimer_real_*` | POSIX interval timers and signal delivery are owned by the timer/signal subsystem | supports `setitimer(ITIMER_REAL)` for alarm-driven loops by polling deadline on user return and delivering `SIGALRM`; RISC-V and LoongArch64 have arch-specific signal-frame/trampoline paths for the current benchmark handlers, while other timer classes remain rejected. |
| test staging cwd display rewrite | user process launch has a single namespace root | path resolution and `getcwd` observe the same namespace; no display-only behavior in broad gates. |
| `mlock* => 0` | page pinning exists or the workload explicitly accepts unsupported | replace with `ENOSYS` or `EOPNOTSUPP` before LTP mm gates. |
| ad hoc `/dev/*` fd entries | devfs or device registry exists | only known devices succeed; unknown devices return `ENOENT` or `ENODEV`. |
