# 2026-04-27 UnixBench pipe/spawn 阻塞修复

- 时间：2026-04-27 11:34:19 CST
- 范围和目标：修复 `unixbench-musl` 在 `CONTEXT` 后进入 `PIPE` 子项时长期阻塞的问题，修复通过 `PIPE` 后 `SPAWN` 信号/等待交互卡住的问题，并补齐后续 `EXECL`、文件写入和 `/bin/true` 兼容路径，使 UnixBench 组可以执行到结束标记。
- 修改文件：`api/arceos_posix_api/src/uspace.rs`，调整当前 shell syscall 路径的 pipe 环形缓冲区容量，增加 pipe 本地等待队列，补齐 `CLONE_VFORK | CLONE_VM` fork-like 路径的父进程等待完成语义，补齐 `execve(envp)` 初始栈传递，增加只读 runtime 根的 `/tmp` 写入 shadow，以及 `/bin`/`/usr/bin` runtime binary 候选；`doc/logs/2026-04-27-unixbench-pipe-buffer.md`，记录本次行为变更。
- 关键决策：UnixBench `pipe.c` 是单进程吞吐测试，每轮先向同一根 pipe 写入 512 字节，再从读端读回 512 字节。原 `PIPE_BUF_SIZE = 256` 会让阻塞写在读操作前填满管道并等待读者，单线程无法继续执行读端，导致测试停在 `PIPE` 行。将容量调整为 4096，匹配 Linux 常见页大小和 `PIPE_BUF` 级别的最小原子写入规模，不引入 workload/path/command 特判。
- 关键决策：BusyBox shell 创建 UnixBench 管线时会走 `CLONE_VFORK | CLONE_VM | SIGCHLD` 形态。原实现把该组合当普通 fork-like 路径执行，但没有实现 vfork 的父进程阻塞到子进程 `execve` 或退出的完成点，导致 shell 管线创建与子进程 exec 时序竞争，表现为 `PIPE` 输出后还未进入 `spawn` 的告警/计时逻辑就卡住。新增 `VforkCompletion`，子进程在成功 `execve` 前或最后线程退出时完成，父进程在 `sys_clone` 返回 pid 前等待该完成点。当前实现仍使用复制地址空间而不是共享 VM，但补齐了 BusyBox vfork 管线依赖的父进程等待语义。
- 关键决策：没有在 `wait_child::reap_child` 中等待底层 task `join()`。UnixBench `spawn.c` 依赖 `SIGALRM` 中断高频 `fork/wait` 循环；如果 wait 在进程已退出后继续进入不可中断的 task join，SIGALRM 只能成为 pending signal，无法及时返回用户态执行 handler。当前 reap 仍以进程 `live_threads == 0` 和 exit code 为 wait 语义完成点，底层 task 生命周期回收留给调度器后续处理。
- 关键决策：pipe 读写在空/满时改用 `PipeState` 内的 `read_wait`/`write_wait`，并在读、写、端点关闭时唤醒对端，避免管线中 `grep`/`awk` 空读长期 runnable。`wait_child` 的阻塞等待改用当前 task 的 `signal_wait.wait_timeout()`，让 `SIGALRM`/`SIGCHLD` 能唤醒 wait 循环，再由用户返回 hook 注入 handler。
- 关键决策：`EXECL` 依赖 `UB_BINDIR=./` 环境变量；原 `sys_execve` 忽略 `envp`，导致程序内 `getenv` 失败并使用未初始化路径。新增 `read_execve_envp` 并把环境字符串写入初始用户栈的 `envp` 区域。
- 关键决策：competition ext4 根文件系统按 `axfs` 设计只读，UnixBench 的 `fstime` 和 shell 子项会在 `/musl` cwd 下创建临时文件。新增明确命名的 `compat_runtime_write_shadow_path`，仅对 `/musl`/`/glibc` runtime 根下的写创建失败映射到 `/tmp/.arceos-runtime-writes/...`，后续读和 unlink 使用同一确定性路径；不按 workload 文件名特判。
- 关键决策：`syscall exec` 子项使用 `/bin/true`。新增通用 `/bin/foo`、`/usr/bin/foo` exec 候选，优先尝试 runtime 根下的 `foo`，再尝试 runtime 根下的 `busybox` applet，不对 `true` 单独特判。
- 验证结果：已在 RISC-V 定向 QEMU 中运行 `runu /musl/busybox sh /musl/unixbench_testcode.sh`，输出包含 `PIPE`、`SPAWN`、`EXECL`、FS、SHELL、算术项、`EXEC`，并到达 `#### OS COMP TEST GROUP END unixbench-musl ####`，进程退出状态为 0。待执行正式 `QEMU_TIMEOUT=1500s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info` 和 `QEMU_TIMEOUT=1500s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info`。
- 剩余风险和后续任务：当前 vfork 兼容路径没有共享父子地址空间，只补齐父等待完成语义；若后续 workload 依赖 vfork 子进程在 exec 前对共享地址空间的副作用，应按 `docs/development/interfaces/process-scheduler.md` 完整实现共享地址空间生命周期或明确拒绝 unsupported 状态。`compat_runtime_write_shadow_path` 是针对只读 runtime 根的兼容层，后续如果 ext4fs 支持安全写入，应删除该 shadow 路径并改回真实文件系统写入。

## 追加验证记录 2026-04-27 23:55

- 范围：复核 RISC-V 与 LoongArch 两条 bench wrapper 中 `unixbench-musl` 是否能完整跑完。
- RISC-V：执行 `QEMU_TIMEOUT=1500s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-rv-iozone-unixbench-full.log`，wrapper 返回 0；日志中 `unixbench-musl` 从 DHRY2 跑到 EXEC，并输出 `#### OS COMP TEST GROUP END unixbench-musl ####`。
- LoongArch：执行 `QEMU_TIMEOUT=1500s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info | tee /tmp/arceos-la-iozone-unixbench-full.log`，wrapper 返回 0；日志中 `unixbench-musl` 从 DHRY2 跑到 EXEC，并输出 `#### OS COMP TEST GROUP END unixbench-musl ####`。
- 观察：LoongArch wrapper 在 `unixbench-musl` 之后继续进入 glibc/lmbench/unixbench-glibc，最终由 timeout 终止 QEMU，但目标 `unixbench-musl` 已完整结束。glibc 后续阶段不是本次阻塞修复目标。
- 剩余风险：当前 `vfork` 兼容路径仍采用复制地址空间并等待子进程 exec/exit 的实现，满足当前 UnixBench/shell 语义；后续若引入共享地址空间式 vfork，需要单独设计父子地址空间冻结和恢复机制。

## 追加验证记录 2026-04-28 00:45

- 范围：按“完整 bench wrapper 自然跑完”重新验证，而不仅是 `unixbench-musl`。
- LoongArch：执行 `QEMU_TIMEOUT=3600s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info | tee /tmp/arceos-la-full-3600.log`，进程返回 0；最终输出 `#### OS COMP TEST GROUP END unixbench-glibc ####`，随后内核打印 `Shutting down...`。
- RISC-V：执行 `QEMU_TIMEOUT=3600s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-rv-full-3600.log`，进程返回 0；最终输出 `#### OS COMP TEST GROUP END unixbench-glibc ####`，随后内核打印 `Shutting down...`。
- 结论：1500s 下后续 glibc UnixBench 未跑完属于全量 wrapper 时间预算不足；修复后使用 3600s 时，RISC-V 与 LoongArch 都可以从 basic-musl 跑到 unixbench-glibc 结束并正常关机。
- 观察：LoongArch 的 `libctest-musl` 中 `utime` 子项仍有断言失败输出，但 wrapper 不因此中断；该问题属于时间戳/utime 语义覆盖范围，不是本次 UnixBench 阻塞修复范围。
