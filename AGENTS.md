# Repository Guidelines

## Project Structure & Module Organization

ArceOS is a Rust 2024 workspace for a modular OS/unikernel. Core crates live in `modules/` (`axhal`, `axtask`, `axfs`, `axnet`, etc.). Public API layers are in `api/`, user libraries in `ulib/`, and runnable Rust/C applications in `examples/`. Platform configuration lives in `configs/`, build logic in `scripts/make/`, board helpers in `tools/`, and documentation assets under `doc/`.

Tests are crate-local: look for `modules/*/tests/` integration tests and inline files such as `modules/axtask/src/tests.rs`.

## Build, Test, and Development Commands

- `make A=examples/helloworld ARCH=x86_64`: build an example app for the selected architecture.
- `make A=examples/httpserver ARCH=aarch64 LOG=info SMP=4 run NET=y`: build and run under QEMU with networking enabled.
- `make defconfig ARCH=riscv64`: generate the default `.axconfig.toml` for an architecture or platform.
- `make fmt`: run `cargo fmt --all`; use `make fmt_c` for C headers/sources in `ulib/axlibc`.
- `make clippy` or `make clippy ARCH=riscv64`: run clippy for the host/default target or a cross target.
- `make unittest_no_fail_fast`: run workspace unit tests without stopping at the first failure.
- `make doc_check_missing`: build docs with missing-doc checks enabled.

Use the pinned `rust-toolchain.toml` toolchain (`nightly-2025-05-20`).

## Local Docker Workflow

On this machine, do not assume ArceOS Rust builds or QEMU runs work on the host. Use the existing long-lived Docker container `arceos-eval-fix` for Rust, cross-architecture, kernel, and testsuite work.

- If the container is stopped, start it with `docker start arceos-eval-fix`.
- Never remove, recreate, or replace `arceos-eval-fix`; it is intentionally long-lived. Do not use `docker rm` or `docker run --rm --name arceos-eval-fix` as a substitute.
- Run ArceOS commands from the container path `/workspace/arceos`, usually via `docker exec arceos-eval-fix ...`.
- Build the evaluation kernels with:
  - `docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info`
  - `docker exec arceos-eval-fix make -C /workspace/arceos kernel-la KERNEL_LOG=info`
- Run testsuite wrappers from the workspace root, not from `arceos/`:
  - `QEMU_TIMEOUT=240s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info`
  - `QEMU_TIMEOUT=240s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info`
- If only a focused subset is needed, it is acceptable to stop the QEMU process after the relevant `basic` section has completed, then parse the saved log. Do not stop or delete the container itself.

For basic filesystem/fd work, verify the focused results with `testsuits-for-oskernel/basic/user/src/oscomp/test_runner.py` and report the exact subset status rather than relying only on visible log snippets.

## Development Context Index

Use `docs/development/README.md` as the canonical long-lived guide for ArceOS
testsuite and syscall-interface work. The older `docs/superpowers/specs/`
documents are historical design records; when they conflict with
`docs/development/`, the `docs/development/` documents win.

For filesystem/fd work, load `docs/development/interfaces/filesystem.md` before
editing `examples/shell/src/uspace.rs` or `examples/shell/src/linux_fs/**`.
`examples/shell/src/linux_fs/` is the Linux ABI/semantic layer for the current
shell syscall path; it is not a VFS and must not rewrap `axfs` backend
capability. Do not modify `modules/axfs/**` for shell compatibility behavior
unless the task explicitly requires a real lower-level filesystem interface.

Load only the documents needed for the current task:

- Filesystem, fd, paths, stat, mount, dev nodes:
  `docs/development/interfaces/filesystem.md`
- Memory management, VMA, page fault, file-backed mmap, shared memory:
  `docs/development/interfaces/memory.md`
- Process, threads, `clone/fork/exec/wait`, scheduler, signals:
  `docs/development/interfaces/process-scheduler.md`
- IPC, pipe, futex, select/poll/epoll, SysV IPC, eventfd, timerfd:
  `docs/development/interfaces/ipc-sync.md`
- Networking, sockets, TCP/UDP, iperf, netperf, LTP network:
  `docs/development/interfaces/network.md`
- Time, timers, system info, libc/runtime, Lua, CPU benchmarks:
  `docs/development/interfaces/time-system-runtime.md`
- Syscall numbers, current handlers, status, owners:
  `docs/development/interfaces/syscall-inventory.md`
- Compatibility shims, `compat_`, fake-state exit rules:
  `docs/development/policies/compatibility.md`
- Errno policy and user-memory validation order:
  `docs/development/policies/errno.md`

When adding, removing, renaming, or changing behavior of a syscall dispatcher
arm or `sys_*` handler, update
`docs/development/interfaces/syscall-inventory.md` in the same commit. Verify
syscall numbers against `examples/shell/src/uspace.rs` and
`../testsuits-for-oskernel/basic/user/lib/syscall_ids.h`.

Compatibility code must use `compat_` naming, include a milestone and deletion
condition, and must not return fake success for unsupported states.

Do not pass tests by opportunistic shortcuts. Avoid pseudo implementations,
broad hardcoded test special cases, workload-name/path/command branches, and
silent success for unsupported behavior. If a temporary compatibility path is
unavoidable, it must be narrow, named `compat_*`, reject unsupported states
with explicit errno, record enough state for inverse operations, and document
its deletion condition in `docs/development/policies/compatibility.md`.

For filesystem/fd changes, keep the focused `basic` subset green on both
RISC-V64 and LoongArch64. Use the workspace-root test wrappers, stop only QEMU
after the relevant `basic` section if needed, and parse saved logs with
`testsuits-for-oskernel/basic/user/src/oscomp/test_runner.py`.

## Design/Behavior Change Logging Rule

Any design decision or behavior-affecting code change must be recorded in
`doc/logs` (Chinese preferred) before or immediately after implementation.

Each log entry must include:

- Date/time
- Scope and objective
- Files changed and call-paths/entry points affected
- Decision rationale and tradeoff notes
- Validation commands/results (including RV/LA and workload slice when applicable)
- Open risks and next action

Do not use commit messages as the only record.

## Coding Style & Naming Conventions

Follow standard Rust formatting through `cargo fmt`. Keep names consistent: Rust crates use `ax*` prefixes, modules are snake_case, and Make options are uppercase (`ARCH`, `A`, `FEATURES`, `APP_FEATURES`). Prefer `no_std`-friendly code in kernel and library crates. Keep unsafe code narrow and document non-obvious invariants.

## Testing Guidelines

Add unit tests near the crate they cover, either in `src/tests.rs` or `tests/*.rs`. For device, filesystem, or network behavior, add or update an example/app test path and document required `ARCH`, `FEATURES`, `BLK`, `NET`, or `BUS` values. Before submitting, run `make fmt`, `make clippy`, and the smallest relevant build or test; run cross-architecture checks when touching `axhal`, platform config, or shared runtime code.

## Commit & Pull Request Guidelines

Recent history uses short imperative subjects and occasional conventional prefixes such as `fix:` and `docs:`; follow that style and include issue or PR references when relevant, for example `fix: complete rv/la ELF runtime support (#328)`.

Pull requests should describe affected modules, tested architectures, feature flags, and exact commands run. Include QEMU output or logs for runtime behavior changes. Keep unrelated formatting, dependency, and generated-file changes out of focused PRs.
