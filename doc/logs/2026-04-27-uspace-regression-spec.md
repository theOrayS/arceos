# 2026-04-27 uspace 回归修复 Spec

## 背景

在 `test/merge-uspace-fix` 上，完整 RV/LA 测例已经能跑到结束，但日志暴露了若干语义退化。此次修复优先恢复已经明确退化的基础 syscall/autorun 行为，避免用 broad skip 或伪成功掩盖问题。

## 观察到的退化

- `mount/umount`：`output_la.md` 与 `output_rv.md` 中 basic-musl/basic-glibc 均出现 `mount return: -38`，对应 `ENOSYS`。
- `getdents64`：basic-musl/basic-glibc 均出现 `getdents fd:-20`，对应 `ENOTDIR`。测试源码实际执行 `open(".", O_RDONLY)` 后对 fd 调用 `getdents64`。
- autorun skip：`cyclictest`、`iozone`、`iperf`、`netperf` 被 `examples/shell/src/cmd.rs` 中显式跳过，不能证明继续通过。
- fork/mapping：日志中还有 `can't fork: Bad address`、`Mapping error: BadState` 类风险。该类问题涉及 clone/fork 地址空间复制和 VMA 语义，不能和 mount/getdents 基础退化混成一个猜测性补丁。

## 已确认根因

- `mount/umount` 的兼容实现存在于 `examples/shell/src/linux_fs/mount.rs`，但当前运行入口已经切到 `arceos_posix_api::uspace`，`api/arceos_posix_api/src/uspace.rs` 没有 `__NR_mount` / `__NR_umount2` 分发臂，因此返回默认 `ENOSYS`。
- `getdents64` 只接受 `FdEntry::Directory`，而当前 `open_fd_candidates` 在 `File::open(".")` 成功时直接返回 `FdEntry::File`。Linux 允许以只读方式打开目录，之后由 `getdents64` 读取目录项。
- autorun skip 是显式策略，不是运行失败后的自动降级。恢复这些 suite 的执行需要先移除非必要 skip，同时保留已明确未接线或越界的 suite skip。
- 执行 RV 构建时发现 `modules/axhal/src/lib.rs` 同时 `pub use axplat::init::{init_early, init_later}` 并定义同名 wrapper，导致 `E0255` 重复定义。该问题会阻塞所有后续验证，需作为 rebase 残留构建回归一并修复。
- 移除 `cyclictest` skip 后，RV 在 hackbench 清理阶段卡在 `signaling 160 worker threads to terminate`。`kill(SIGTERM)` 只向一个线程挂 pending signal，没有按默认致命信号语义请求进程退出并唤醒所有线程；进一步验证后发现 pipe wait queue 也没有被 signal 定向唤醒，worker 可停在 pipe 读写等待中。

## 本次范围

- 在 `api/arceos_posix_api/src/uspace.rs` 中接入 narrow `compat_basic_mount` 状态，恢复 basic 中 `mount("/dev/vda2", "./mnt", "vfat", 0, NULL)` 和对应 `umount2` 成功路径，并对不支持的 flags/data/fstype/source 返回明确 errno。
- 修改目录打开路径，使只读 `open(".", O_RDONLY)` 最终得到 `FdEntry::Directory`，从而 `getdents64` 返回正数目录项长度。
- 移除 `examples/shell/src/cmd.rs` 中 `cyclictest`、`iozone`、`iperf`、`netperf` 的 broad skip，让这些 workload 至少真实运行并暴露状态。
- 删除 `modules/axhal/src/lib.rs` 中重复的 `init_early/init_later` re-export，保留现有 wrapper 以继续初始化 boot argument 和 CPU 数量。
- 修正 `kill/tkill/tgkill` 对默认致命信号的处理：`SIG_IGN` 忽略，显式 handler 保持用户态 signal frame，默认致命信号触发 `request_exit_group` 并唤醒目标进程所有线程。
- 为 pipe wait 增加当前等待目标记录；signal 投递时定向唤醒目标任务所在 pipe wait queue，并把 pending exit/signal 纳入 pipe wait 条件。
- 记录 fork/mapping 风险和后续拆分计划，但本次不做大规模 VMA/COW 重构。

## 非目标

- 不实现真实 `axfs` runtime mount/umount。
- 不通过 workload 名称或测试命令返回伪成功。
- 不把 `fork/mmap` 高压稳定性作为同一个补丁的完成条件。
- 不删除 `ltp`、`libcbench`、`glibc libctest` 这类已有明确边界说明的 skip。

## 验收标准

- `output_la.md` 和 `output_rv.md` 中 basic-musl/basic-glibc 的 `mount return: 0` 与 `umount return: 0` 恢复。
- `output_la.md` 和 `output_rv.md` 中 basic-musl/basic-glibc 的 `getdents fd:<positive>` 恢复，不再出现 `getdents fd:-20`。
- `cyclictest`、`iozone`、`iperf`、`netperf` 不再打印 `SKIP:`，若失败应以真实失败日志呈现。
- 修改记录写入 `arceos/doc/logs`，包含 RV/LA 命令、结果和剩余风险。
