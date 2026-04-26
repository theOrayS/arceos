# Agents Guidelines for ArceOS

This file is a local working contract for AI agents editing this `arceos/` tree.
It is intentionally stricter and more operational than the public `README.md`.
When repository docs and this file disagree, follow the repository's actual code,
build scripts, and CI configuration.

ArceOS is an experimental modular OS/unikernel written in Rust. This local tree
already contains generated kernels, logs, and in-progress user changes. Work
incrementally and avoid broad cleanups unless they are the task.

## Repository Layout

| Directory | Purpose |
|-----------|---------|
| `modules/` | Core runtime and subsystems: scheduler, memory, drivers, FS, net, sync, IPC |
| `api/` | Public ArceOS APIs and POSIX-facing APIs; `arceos_posix_api` contains the user-space boundary |
| `ulib/` | User-facing libraries such as `axstd` and `axlibc` |
| `examples/` | Rust and C example applications used by local builds and CI |
| `configs/` | Platform configuration files |
| `scripts/` | Make helpers, build flow, and QEMU helpers |
| `tools/` | Board-specific and utility tools |
| `doc/` | Build and platform documentation |
| `build/`, `target/` | Generated artifacts; avoid manual edits |

Important generated or local-only files at repo root include `kernel-rv`,
`kernel-la`, `*.log`, and `.axconfig.toml`. Do not edit or commit them unless
the task is explicitly about generated artifacts.

## Build and Run

All commands below are run from the `arceos/` root:

```bash
# Build a Rust example
make A=examples/helloworld ARCH=x86_64

# Build and run an example in QEMU
make A=examples/shell ARCH=riscv64 run

# Lint and docs
make clippy
make doc_check_missing

# Unit tests
make unittest_no_fail_fast

# Build the local kernel images used by this fork
make kernel-rv
make kernel-la
```
## Notes:

- ARCH must be one of x86_64, riscv64, aarch64, or loongarch64.
- Running apps or app tests requires QEMU.
- Building C examples requires the musl cross toolchains mentioned in README.md.
- This fork also provides make run-rv and make run-la for testsuite-backed boots.
## Toolchain

- Rust toolchain is pinned in rust-toolchain.toml to nightly-2025-05-20.
- Edition is 2024.
- Required Rust components: rust-src, llvm-tools, rustfmt, clippy.

- Configured Rust targets:
  - x86_64-unknown-none
  - riscv64gc-unknown-none-elf
  - aarch64-unknown-none-softfloat
  - loongarch64-unknown-none-softfloat
- C formatting follows the repo .clang-format.
- There is no repo-local rustfmt.toml; use the pinned toolchain's formatter.

## Hard Constraints
### General
- Work from the arceos/ root, not the outer workspace, unless the task clearly
spans sibling directories.
- Prefer minimal, subsystem-local patches. Do not refactor across modules/,
api/, ulib/, and examples/ unless the task requires it.
- Assume the Git worktree is dirty. Never revert unrelated user changes.
- Do not hand-edit generated outputs in build/, target/, or root-level kernel
binaries and logs.
- Preserve platform and feature structure. This repo is intentionally built across
four architectures and many #[cfg(...)] / FEATURES combinations.
- Do not perform repository-wide search/replace, mechanical renames, import
normalization, or bulk formatting unless the task explicitly requires it.

### Rust
- Follow the style already present in the touched file; do not import rules from
other projects unless ArceOS already does so.
- unsafe already exists in low-level modules, runtime code, drivers, and POSIX
boundary code. Do not impose blanket bans that the repository itself does not
follow.
- When adding new unsafe, keep it narrow and explain the invariant with a
// SAFETY: comment when the reason is not trivial from nearby code.
- Do not collapse architecture-specific code just to simplify control flow.
Preserve #[cfg(target_arch = ...)] behavior.
- Avoid unwrap() and expect() on fallible runtime, syscall, filesystem, or
networking paths unless the invariant is immediate and locally proven.
- Prefer small helpers and early returns over deeper nesting, especially in very
large files.
### POSIX and User-Space Boundary
- Treat raw user pointers, lengths, and ABI-visible structures as untrusted input.
- Validate before turning raw pointers into slices, strings, or structs.
- Keep copy-in/copy-out behavior explicit. Do not silently widen trust boundaries.
- Preserve Linux/POSIX-visible behavior when changing syscalls, errno mapping, or
struct layouts.
- In api/arceos_posix_api/src/uspace.rs, avoid broad rewrites. It is a large
integration file covering ELF loading, memory layout, FDs, signals, futexes,
and syscall handling.
- If a change modifies syscall behavior, errno mapping, ABI-visible struct layout,
user-visible return values, or other POSIX/Linux-observable semantics, the
final summary must explicitly list the visible behavior changes. If there is no
intended visible behavior change, say so clearly.
### Logging and Output
- In modules/, api/, and ulib/, prefer existing logging facilities such as
axlog macros over ad-hoc printing.
- In examples/, stdout/stderr-oriented behavior is acceptable when it is part
of the example's visible interface.
## Validation Rules

Pick the smallest check set that proves the change:

- Formatting-only or broad Rust edits: run cargo fmt --all.
- Library or module changes: run make clippy.
- examples/ changes: build the touched example for the affected architecture.
- api/arceos_posix_api or user-space changes: at minimum build
  make A=examples/shell ARCH=riscv64 or the closest relevant kernel target.
- Unit-testable code: run make unittest_no_fail_fast.

Additional validation guidance:

- For changes spanning tightly coupled boot, trap, scheduler, or user-task flow
code — especially across modules/axruntime, modules/axhal,
modules/axtask, and api/arceos_posix_api/src/uspace.rs — prefer staged
validation:
1. first perform the smallest relevant build-only validation,
2. then perform behavior or run-time validation after the build succeeds.
- Do not jump straight to behavior testing on a cross-cutting change if the
basic build path has not been revalidated first.

If QEMU, Docker, or cross toolchains are unavailable, state exactly which checks
could not be run instead of claiming full verification.

## CI Facts

Current CI verifies at least the following:

- cargo fmt --all -- --check
- make clippy and make clippy ARCH=<arch>
- Example builds for Rust and C apps on x86_64, riscv64, aarch64, and loongarch64
- make doc_check_missing
- make unittest_no_fail_fast
- QEMU-backed app tests via arceos-apps

Any change that only works on one local arch but breaks other configured targets,
feature combinations, or CI entry points should be treated as a regression.

## Subsystem Notes
- examples/shell is not just a demo shell; it is also a practical integration
- point for test and user-space flows in this tree.
- api/arceos_posix_api is ABI-sensitive. Avoid casual renames, layout changes,
or behavior changes that leak through libc/POSIX-facing APIs.
- modules/axruntime, modules/axhal, modules/axtask, and
api/arceos_posix_api/src/uspace.rs are tightly coupled in boot, trap, and
user-task flows. Cross-cutting changes there need extra care. Prefer build
validation before behavior validation when touching this chain.

## Change Summary Requirements

When reporting completed work, include:

- the files changed,
- the intent of each change,
- the validation commands actually run,
- the checks that could not be run, if any,
- any user-visible behavior change,
- any syscall / errno / ABI-visible change, or an explicit statement that there
was no intended visible ABI/POSIX behavior change.

Do not claim full verification unless the relevant checks were actually run.