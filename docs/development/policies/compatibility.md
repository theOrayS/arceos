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

## Current Known Compatibility Exits

| Compatibility path | Delete when | Interim behavior |
| --- | --- | --- |
| `linux_fs::mount::MountTable` / `compat_basic_mount` | runtime `axfs` mount/unmount exposes mounted contents | duplicate mount returns `EBUSY`; unmounted target returns `EINVAL` or `ENOENT`; unsupported flags/data must not succeed. |
| `linux_fs::stat` statx projection from `stat` | filesystem metadata returns statx-capable fields and mask | return only honest fields through `requested_mask & supported_mask`; invalid flags `EINVAL`; bad buffers `EFAULT`. |
| test staging cwd display rewrite | user process launch has a single namespace root | path resolution and `getcwd` observe the same namespace; no display-only behavior in broad gates. |
| `mlock* => 0` | page pinning exists or the workload explicitly accepts unsupported | replace with `ENOSYS` or `EOPNOTSUPP` before LTP mm gates. |
| ad hoc `/dev/*` fd entries | devfs or device registry exists | only known devices succeed; unknown devices return `ENOENT` or `ENODEV`. |
