# uspace 回归修复跟进日志

- 日期时间：2026-04-28 约 12:00 CST
- 变更范围和目标：继续修复测试套件回归，目标是恢复 mount/umount、getdents、iozone、iperf、netperf、lmbench、unixbench 等套件的连续执行能力，避免通过宽泛 skip 或伪实现绕过问题。
- 修改文件：
  - `api/arceos_posix_api/src/uspace.rs`
  - `examples/shell/src/cmd.rs`
- 关键决策依据：
  - `netserver` 日志显示 `select failure: Interrupted system call (errno 4)` 后退出；代码确认子进程退出时无条件投递默认忽略的 `SIGCHLD`，会错误打断阻塞 syscall。因此调整为默认忽略信号不作为 syscall interruption，且 `SIGCHLD` 只有安装 handler 时才投递给父进程。
  - netperf 日志显示 `SO_DONTROUTE` 和 `IP_RECVERR` 返回 `EINVAL`；这两个选项在当前 loopback/smoltcp 兼容层中属于诊断或路由提示，不改变现有数据路径，因此做窄范围 no-op 兼容，未知选项仍返回错误。
  - iozone 在 `iozone test complete.` 后不回到脚本；补充 exit-group 在 user-return hook 的检查，使 `exit_group` 标记能在中断返回用户态前终止同进程线程，避免父进程长期等待 `live_threads` 归零。
  - futex 无超时等待只依赖 notify；为降低 `CLONE_CHILD_CLEARTID`/join 漏唤醒风险，改为短周期 timeout wait 并重新读取 futex 值，同时响应 exit-group。
  - lmbench 的 staged wrapper 不能依赖裸 `sleep`，改为通过套件 busybox 执行 sleep；去掉 `set -x` 减少完整日志噪声。
  - staged script 中 `./$i` 在 `$i` 为绝对路径时会变成 `.//musl/busybox`，改为按变量值是否为绝对路径选择直接执行或加 `./`。
- 验证结果：
  - RV 长探针 `/tmp/arceos-rv-1500-after-exitgroup-fix.log`：basic、busybox、cyclictest 通过；`mount return: 0`、`umount return: 0`、`getdents fd:456` 保持正常；iozone 能完整打印 `#### OS COMP TEST GROUP END iozone-musl ####` 并进入 iperf。
  - RV 长探针 `/tmp/arceos-rv-1500-after-futex-fix.log`：iozone 仍能进入 iperf；iperf 在 `BASIC_TCP` 打印 `iperf Done.` 后未返回 shell，QEMU guest 持续占用 CPU，需要停止该轮验证。
  - RV 长探针 `/tmp/arceos-rv-1500-after-socket-wait-fix.log`：尝试 socket 写等待兜底后未解决，且卡点提前到 `BASIC_UDP` 的 `iperf Done.` 后；该兜底改动已撤销。
- 剩余风险和后续任务：
  - 当前尚未完整跑通；iperf 客户端在打印 `iperf Done.` 后不退出，下一步应定位 `iperf_client_end()` 之后的控制通道 `Nwrite(IPERF_DONE)`、socket close/free 或相关 syscall 路径。
  - iozone 的 pwrite/pread 曾出现 `Min throughput = 0`；最新一轮有改善但仍需在完整稳定日志中复核。
  - netperf 的 `SO_DONTROUTE`/`IP_RECVERR` 兼容已补，但尚未在最新代码中跑到 netperf 阶段完成验证。
  - 最终仍需分别运行 RISC-V 和 LoongArch 完整套件，并将日志写入 workspace 根目录 `output_rv.md` 和 `output_la.md`。

## iperf 最小复现入口

- 命令计划：使用构建期环境变量 `ARCEOS_AUTORUN_ONLY_GROUP=iperf` 只运行 iperf 组，缩短复现反馈链路。
- 变更：在 `examples/shell/src/cmd.rs` 的 `maybe_run_official_tests()` 中加入默认关闭的组过滤开关。
- 约束：默认完整套件不受影响；该开关不改变测试结果、不跳过目标组内部命令、不返回伪成功。
- 下一步：用 RV 轻量运行确认 `iperf Done.` 后不返回 shell 是否可在 iperf-only 路径稳定复现。

## iperf 最小复现结果

- 命令：`docker exec arceos-eval-fix env ARCEOS_AUTORUN_ONLY_GROUP=iperf timeout 180s make -C /workspace/arceos run-rv ARCH=riscv64 SMP=1 MEM=1G RV_TESTSUITE_IMG=/workspace/testsuits-for-oskernel/sdcard-rv.img KERNEL_LOG=info | tee /tmp/arceos-iperf-rv.log`
- 结果：过滤开关生效，启动后直接进入 `#### OS COMP TEST GROUP START iperf-musl ####`。
- 复现：`BASIC_UDP` 打印 `iperf Done.` 后 60 秒无 `====== iperf BASIC_UDP end: success ======`，手动停止 QEMU。
- 判断：后续诊断可以使用 iperf-only 路径，不需要每次从 basic/iozone 跑到 iperf。

## iperf syscall 级诊断

- 时间：2026-04-28 晚间
- 范围：`arceos/api/arceos_posix_api/src/uspace.rs`
- 目标：在 iperf-only 最小复现中定位 `iperf Done.` 后不返回 shell 的最后 syscall。
- 变更：对当前进程路径包含 `iperf` 的任务临时打印 `sendto`、`recvfrom`、`close`、`shutdown`、`pselect6`、`futex`、`exit`、`exit_group` 返回路径。
- 决策：诊断只针对 iperf 进程，且不改变 syscall 返回值；正式修复后删除或保持默认关闭，避免污染完整测试日志。
- 待验证：运行 RV iperf-only trace，读取 `/tmp/arceos-iperf-trace-rv.log` 中最后一个 `iperf-trace`。

## iperf wait 唤醒修复

- 时间：2026-04-28 晚间
- 范围：`arceos/api/arceos_posix_api/src/uspace.rs`
- 目标：修复 iperf client 已 `exit_group` 后 shell 偶发无法继续 `wait4` 回收的问题。
- 根因：`wait_child()` 阻塞路径等待当前任务的 signal wait queue，而子进程最后一个线程退出时只唤醒子进程自己的 `exit_wait`；父进程在 SIGCHLD 默认忽略时没有可靠唤醒，只依赖 timeout 轮询，容易在单核/高频 select 场景下表现为卡住。
- 修复：子进程最后一个线程退出时无条件唤醒父进程 `exit_wait`；`wait_child()` 阻塞等待改为等待父进程 `exit_wait`，保留 SIGCHLD handler 时的信号投递。
- 诊断清理：删除 iperf-only syscall trace，正式日志不再输出 `iperf-trace`。
- 验证：`docker exec arceos-eval-fix env ARCEOS_AUTORUN_ONLY_GROUP=iperf timeout 240s make -C /workspace/arceos run-rv ARCH=riscv64 SMP=1 MEM=1G RV_TESTSUITE_IMG=/workspace/testsuits-for-oskernel/sdcard-rv.img KERNEL_LOG=info | tee /tmp/arceos-iperf-clean-rv.log` 自然结束，`iperf-musl` 与 `iperf-glibc` 的 `BASIC_UDP`、`BASIC_TCP`、`PARALLEL_UDP`、`PARALLEL_TCP`、`REVERSE_UDP`、`REVERSE_TCP` 均打印 `end: success`，未再出现 `iperf-trace`。
- 风险：glibc iperf server daemon 化仍打印 `unable to become a daemon: No such device`，但该轮不阻塞 iperf 组；该兼容问题暂不作为本次 wait 唤醒根因的一部分。
- 后续：进入 RV/LA 中等完整路径验证，确认能跑过 iperf 并继续到 netperf/lmbench/unixbench。

## lmbench 单组挂点与 msync 兼容

- 时间：2026-04-28 晚间
- 范围：`arceos/api/arceos_posix_api/src/uspace.rs`、`arceos/docs/development/policies/compatibility.md`
- 目标：按单组轻量验证策略修复 `lmbench-musl` 在 `lat_pagefault` 后不继续的问题。
- 现象：RV 中等完整路径和 RV `ARCEOS_AUTORUN_ONLY_GROUP=lmbench` 均在 `./lmbench_all lat_pagefault -P 1 -N 1 /var/tmp/XXX` 处打印 `msync: Function not implemented`，wrapper 打印 `TIMEOUT` 后无后续输出。
- 决策：补充 `compat_msync`，不按 workload 跳过；仅对页对齐、flag 合法、范围内页已映射的地址返回成功，非法 flag、未映射页或溢出返回明确 errno。当前 `mmap` 文件映射是拷贝到用户页，尚无共享脏页写回模型，因此合法范围视为已同步。
- 删除条件：当 `AddrSpace`/mmap 跟踪 file-backed VMA 并支持真实 `MS_SYNC`、`MS_ASYNC`、`MS_INVALIDATE` 写回/失效语义时删除 `compat_msync`。
- 验证进展：RV `lmbench-only` 重新运行后 `lat_pagefault` 已打印 `Pagefaults on /var/tmp/XXX`，后续 `lat_mmap` 与 `lat_fs` 继续执行，说明 `msync` 缺失挂点解除。
- 新挂点：`bw_pipe` 触发 `TIMEOUT: ./lmbench_all bw_pipe -P 1` 后无后续输出。当前 pipe 容量仅 4KiB，会显著拖慢带宽测试并增加阻塞/timeout 清理风险。
- 继续修复：将 shell syscall 层 pipe 缓冲容量调整为 64KiB，贴近 Linux 默认 pipe capacity，属于通用 pipe 吞吐修复，不按 lmbench 名称绕过。
- 修正：64KiB 缓冲不能作为大数组直接嵌入 `PipeRingBuffer` 初始化路径，否则会放大内核栈/临时对象压力；改为 `Box<[u8]>` 堆分配，保留 64KiB 容量同时避免栈溢出。
- 验证结果：`docker exec arceos-eval-fix env ARCEOS_AUTORUN_ONLY_GROUP=lmbench timeout 420s make -C /workspace/arceos run-rv ARCH=riscv64 SMP=1 MEM=1G RV_TESTSUITE_IMG=/workspace/testsuits-for-oskernel/sdcard-rv.img KERNEL_LOG=info | tee /tmp/arceos-lmbench-pipe-box-rv.log` 自然结束，`lmbench-musl` 与 `lmbench-glibc` 均打印 `#### OS COMP TEST GROUP END ... ####`；`bw_pipe` 打印 `Pipe bandwidth`，不再触发 timeout。
- 剩余风险：glibc `lat_proc shell` 仍打印 `/bin/sh: can't open '/bin/sh': No such file or directory`，但该项继续输出结果并未阻塞 lmbench；是否需要补 `/bin/sh` 兼容应由后续完整日志风险清单单独判断。

## netperf TCP 半关闭与 CRR 收尾定位

- 时间：2026-04-28 18:16:30 CST 起
- 范围与目标：修复 `netperf` 在 `UDP_STREAM` 表格后不返回 shell，以及后续 TCP stream/request-response 的退化。
- 修改文件：
  - `api/arceos_posix_api/src/uspace.rs`：`sys_shutdown` 开始区分 `how`，`SHUT_WR` 走 TCP 写端半关闭；`sys_pselect6` 在网络 fd readiness 检查前执行小轮数 `axnet::poll_interfaces()`/`yield_now()`；追加默认关闭的 netperf 临时 trace。
  - `modules/axnet/src/smoltcp_impl/tcp.rs`：新增 `TcpSocket::shutdown_write()`，保持读端可继续观察 EOF；`recv()` 改为先交付已排队数据，再在 `!may_recv()` 时返回 EOF 0，避免正常 FIN 被误报为 `ECONNREFUSED`。
  - `docs/development/interfaces/syscall-inventory.md`：补充 `shutdown` 行，并注明 `pselect6` 覆盖 netperf 控制连接关闭场景。
- 关键判断：
  - `netperf UDP_STREAM` 原卡点是 `shutdown(fd, SHUT_WR)` 被实现为整连接关闭，导致控制连接 EOF readiness 无法按 Linux 语义被 `select`/`recv` 观察。
  - `TCP_CRR` 是独立的收尾问题：trace 显示 connect/accept 本身持续成功，计时结束附近出现 `recv(fd=4, len=1) -> EBADF` 后用户态不再进入内核，疑似 SIGALRM 与连接测试断开/关闭 fd 的时序交错。
- 验证结果：
  - 命令：`docker exec arceos-eval-fix env ARCEOS_AUTORUN_ONLY_GROUP=netperf timeout 240s make -C /workspace/arceos run-rv ARCH=riscv64 SMP=1 MEM=1G RV_TESTSUITE_IMG=/workspace/testsuits-for-oskernel/sdcard-rv.img KERNEL_LOG=info 2>&1 | tee /tmp/arceos-netperf-final-rv.log`
  - RV 结果：`UDP_STREAM`、`TCP_STREAM`、`UDP_RR`、`TCP_RR` 已能打印 `end: success`；`TCP_CRR` 仍未自然结束。
  - 诊断命令：`docker exec arceos-eval-fix env ARCEOS_AUTORUN_ONLY_GROUP=netperf ARCEOS_TRACE_NETPERF=1 timeout 180s make -C /workspace/arceos run-rv ARCH=riscv64 SMP=1 MEM=1G RV_TESTSUITE_IMG=/workspace/testsuits-for-oskernel/sdcard-rv.img KERNEL_LOG=info 2>&1 | tee /tmp/arceos-netperf-crr-trace-rv.log`
- 剩余风险与下一步：
  - 临时 `ARCEOS_TRACE_NETPERF` 诊断仍在代码中，最终提交前必须删除或确认默认关闭且不影响正式日志。
  - 下一步应继续沿 `TCP_CRR` 的 SIGALRM/`rt_sigreturn`/fd close 时序定位，重点确认 EBADF 后是否进入信号 handler 或卡在用户态 final cleanup。

## real timer 主动轮询与 netperf TCP_CRR 后续收尾

- 时间：2026-04-28 19:45:00 CST
- 范围和目标：继续修复 `netperf`/`TCP_CRR` 在无 trace 运行时无法自然结束的问题，优先保持通用 Linux 语义，避免 workload 特判。
- 修改文件：
  - `api/arceos_posix_api/src/imp/time.rs`
  - `api/arceos_posix_api/src/uspace.rs`
- 关键决策：
  - 证据显示 `TCP_CRR` 客户端 1s real timer 能触发，但服务端 5s real timer 在 accept/recv 收尾路径中可能长期没有 fire，导致客户端等待控制连接响应。
  - 在 real timer 层增加 `poll_real_timers()`，由 socket wait 和 signal/exit pending 检查路径主动触发到期 timer；保留原 timer task，并在 timer task wake 后复查 deadline，避免主动 poll 与 timer task 对周期 timer 产生双触发。
  - 尝试过在 stdout/stderr write 后增加 yield 来模拟 trace 带来的调度窗口，但该方向使 UDP_STREAM 收尾更早卡住，已回退，不能作为修复路径。
- 验证结果：
  - `ARCEOS_AUTORUN_ONLY_GROUP=netperf ARCEOS_TRACE_NETPERF=1 timeout 180s make -C /workspace/arceos run-rv ARCH=riscv64 SMP=1 MEM=1G RV_TESTSUITE_IMG=/workspace/testsuits-for-oskernel/sdcard-rv.img KERNEL_LOG=info`：带 trace 构建下，musl/glibc `TCP_CRR end: success` 均出现，日志保存在 `/tmp/arceos-netperf-crr-timer-poll-fix-rv.log`。
  - `ARCEOS_AUTORUN_ONLY_GROUP=netperf timeout 180s make -C /workspace/arceos run-rv ARCH=riscv64 SMP=1 MEM=1G RV_TESTSUITE_IMG=/workspace/testsuits-for-oskernel/sdcard-rv.img KERNEL_LOG=info`：无 trace 构建可输出 musl `TCP_CRR` 结果表，但未回到脚本打印 `TCP_CRR end: success`，日志保存在 `/tmp/arceos-netperf-no-trace-after-timer-poll-rv.log`。
- 剩余风险和后续：
  - 还存在一个 trace 依赖的收尾时序问题，位置在 `TCP_CRR` 结果表输出之后、netperf 进程退出或父 shell `wait` 之前。
  - 下一步应继续用更窄的诊断定位无 trace 下结果表之后的 syscall/退出路径，不应保留输出调度类 workaround。

## 2026-04-28 20:35:00 CST - 用户返回与 futex/wait 阻塞路径补齐 real timer 轮询

- 变更范围与目标：继续修复 netperf no-trace 在 TCP_CRR 表格输出后收尾卡住的问题；将 real timer 主动轮询从 socket/select 路径扩展到用户态返回、futex wait 条件和 wait4 阻塞循环，避免没有额外日志调度时 SIGALRM/exit 状态无人推进。
- 修改文件：`api/arceos_posix_api/src/uspace.rs`。
- 关键决策：不添加 workload 名称判断，不对 netperf 返回伪成功；改为统一修补用户态返回和同步阻塞边界，使 timer/signal/exit_group 检查路径一致。`wait4` 仍保持原有 ECHILD/WNOHANG/子进程 reaping 语义，只有阻塞等待期间改用统一的 `current_exit_or_signal_pending()`。
- 验证结果：修改前使用 `ARCEOS_AUTORUN_ONLY_GROUP=netperf timeout 300s make -C /workspace/arceos run-rv ...` 复现 no-trace TCP_CRR 表格输出后超过 60 秒没有进入 success 或超时报错，确认不是短 timeout 误判。修改后的 RV no-trace netperf 分组待验证。
- 剩余风险：如果卡点实际位于控制连接 `shutdown_control()` 的 TCP teardown/EOF readiness，而非 timer/signal 推进，则仍需继续在 shutdown/select/recv 控制连接路径加更窄诊断。

## 2026-04-28 20:55:00 CST - blocking socket wait 改为短睡眠让出调度

- 变更范围与目标：继续定位 RV no-trace netperf TCP_CRR 表格输出后卡住；前一轮补齐用户返回/futex/wait4 的 timer 轮询后仍复现，因此将焦点收敛到 `shutdown_control()` 后续 socket 控制连接收尾路径。
- 修改文件：`api/arceos_posix_api/src/uspace.rs`。
- 关键决策：`socket_wait_interruptible()` 从纯 `yield_now()` 改为 `sleep(1ms)`。原因是该函数表示阻塞 socket 操作的等待边界，忙等式 yield 在单核 no-trace 下可能持续抢占 peer/timer/network progress；短睡眠更接近阻塞等待语义，且不按 workload 名称分支。
- 验证结果：修改前 RV no-trace netperf 在 TCP_CRR 表格后等待超过 70 秒仍无 `end: success`，说明 timer/futex/wait4 轮询不是唯一根因。修改后 RV no-trace netperf 分组待验证。
- 剩余风险：短睡眠可能降低网络 benchmark 吞吐/transaction rate；如果功能打通，后续需在 iperf/netperf 分组和全量 RV/LA 中确认没有新的超时退化。

## 2026-04-28 21:08:00 CST - 撤回 blocking socket 短睡眠尝试

- 变更范围与目标：验证 `socket_wait_interruptible()` 使用 `sleep(1ms)` 是否能解决 netperf TCP_CRR 收尾卡住。
- 修改文件：`api/arceos_posix_api/src/uspace.rs`。
- 关键决策：该尝试导致 RV no-trace netperf 在 UDP_RR 阶段更早停住，说明简单延长 socket wait 调度间隔会破坏请求/响应进度，不是正确修复方向。
- 验证结果：`ARCEOS_AUTORUN_ONLY_GROUP=netperf timeout 300s make -C /workspace/arceos run-rv ...` 在 UDP_RR 只打印 MIGRATED 行后长时间无结果；已停止 QEMU，仅撤回 `sleep(1ms)`，恢复 `yield_now()`。
- 剩余风险：当前仍保留用户返回/futex/wait4 的 real timer 轮询一致性改动；TCP_CRR 表格后卡住仍需继续定位到控制连接 `shutdown/select/recv` 或进程退出路径。

## 2026-04-28 21:30:00 CST - shutdown 成功后主动推进网络和调度

- 变更范围与目标：继续修复 RV no-trace netperf TCP_CRR 表格后不收尾；trace 成功路径显示表格后依赖控制连接 `shutdown(fd=3, SHUT_WR)` 后服务端立刻观察 EOF 并退出。
- 修改文件：`api/arceos_posix_api/src/uspace.rs`。
- 关键决策：在 `sys_shutdown()` 成功后执行一次 `axnet::poll_interfaces()` 和 `yield_now()`，让 FIN/half-close 状态及时进入网络栈并给 peer 处理机会。该修复作用于通用 socket shutdown 成功路径，不按 workload 名称分支；同时撤回了会扰动测试的长等待诊断日志和 `socket_wait` 短睡眠尝试。
- 验证结果：此前临时长等待日志会改变调度并导致 UDP_RR 后卡住，已移除。当前 shutdown 推进修复待 RV no-trace netperf 验证。
- 剩余风险：shutdown 后额外 yield 可能轻微影响网络 benchmark 性能；若通过，需要再跑 iperf/netperf 局部分组和后续全量 RV/LA 验证。

## 2026-04-28 21:45:00 CST - RV no-trace netperf 验证通过

- 变更范围与目标：验证 `sys_shutdown()` 成功后主动 `poll_interfaces()+yield_now()` 是否修复 netperf TCP_CRR 收尾卡住。
- 修改文件：`api/arceos_posix_api/src/uspace.rs`。
- 关键决策：保留 shutdown 成功后的网络推进；该行为匹配 Linux 可见语义中的 half-close/FIN 及时传播需求，不制造测试名分支，也不跳过 `shutdown_control()`。
- 验证结果：`docker exec arceos-eval-fix env ARCEOS_AUTORUN_ONLY_GROUP=netperf timeout 300s make -C /workspace/arceos run-rv ARCH=riscv64 SMP=1 MEM=1G RV_TESTSUITE_IMG=/workspace/testsuits-for-oskernel/sdcard-rv.img KERNEL_LOG=info` 通过。日志显示 `netperf-musl` 和 `netperf-glibc` 均完成 `UDP_STREAM`、`TCP_STREAM`、`UDP_RR`、`TCP_RR`、`TCP_CRR`，并分别打印 `#### OS COMP TEST GROUP END netperf-musl ####` 与 `#### OS COMP TEST GROUP END netperf-glibc ####`，最后 QEMU 正常 `Shutting down...`。
- 剩余风险：需要继续跑 `iperf` RV 分组确认网络 shutdown 推进没有引入其它网络吞吐/收尾退化；之后再推进 LA 与全量 RV/LA 输出日志。

## 2026-04-28 22:00:00 CST - RV iperf 网络回归验证通过

- 变更范围与目标：验证 `sys_shutdown()` 成功后推进网络/调度不会破坏 iperf TCP/UDP 基础、并发和反向传输。
- 修改文件：`api/arceos_posix_api/src/uspace.rs`。
- 关键决策：将 iperf 作为 netperf 修复后的同域轻量回归门禁；该测试覆盖 UDP/TCP、parallel、reverse，能较快暴露 socket wait/shutdown 的新问题。
- 验证结果：`docker exec arceos-eval-fix env ARCEOS_AUTORUN_ONLY_GROUP=iperf timeout 240s make -C /workspace/arceos run-rv ARCH=riscv64 SMP=1 MEM=1G RV_TESTSUITE_IMG=/workspace/testsuits-for-oskernel/sdcard-rv.img KERNEL_LOG=info` 通过。日志显示 `iperf-musl` 和 `iperf-glibc` 均完成 `BASIC_UDP`、`BASIC_TCP`、`PARALLEL_UDP`、`PARALLEL_TCP`、`REVERSE_UDP`、`REVERSE_TCP` 并打印 group end，最后 QEMU 正常 `Shutting down...`。
- 剩余风险：仍需在 LoongArch64 上验证 netperf/iperf 或至少 netperf TCP_CRR 收尾；之后再推进全量输出到 `output_rv.md` 与 `output_la.md`。

## 2026-04-28 22:20:00 CST - LA netperf 验证通过

- 变更范围与目标：验证 `sys_shutdown()` 成功后主动推进网络/调度在 LoongArch64 上同样修复 netperf TCP_CRR 收尾。
- 修改文件：`api/arceos_posix_api/src/uspace.rs`。
- 关键决策：由于 `run-testsuite-bench-la-direct.sh` 没有把 `ARCEOS_AUTORUN_ONLY_GROUP` 传入容器构建环境，本次使用 `docker exec -e ARCEOS_AUTORUN_ONLY_GROUP=netperf ... kernel-la` 显式构建，再直接启动 LoongArch QEMU。
- 验证结果：`docker exec -e ARCEOS_AUTORUN_ONLY_GROUP=netperf arceos-eval-fix make -C /workspace/arceos kernel-la ARCH=loongarch64 KERNEL_SMP=1 LA_MEM=2G KERNEL_LA_AXCONFIG_WRITES='-w plat.phys-memory-size=0x70000000' KERNEL_LOG=info` 后，使用 `qemu-system-loongarch64` 直接运行 `/tmp/arceos-sdcard-la.netperf.qcow2`。日志显示 `netperf-musl` 和 `netperf-glibc` 均完成 `UDP_STREAM`、`TCP_STREAM`、`UDP_RR`、`TCP_RR`、`TCP_CRR` 并打印 group end，最后 LoongArch QEMU 正常 `Shutting down...`。
- 额外观察：一次误启动的 LA 全量片段中 `basic-musl` 已通过，包含 `getdents fd:456`、`mount return: 0`、`umount return: 0`，未见先前 `getdents fd:-20` 或 `mount return:-38` 退化。
- 剩余风险：仍需继续验证 UnixBench/iozone/lmbench 等非网络分组，并最终生成 `output_rv.md` 与 `output_la.md`。

## 2026-04-28 结束前状态记录

- 时间：2026-04-28 晚间
- 变更范围与目标：继续修复 userspace 回归，重点处理网络长测卡死、真实定时器投递、等待路径可中断性，以及 UnixBench 后段卡住问题。
- 已改文件：
  - `api/arceos_posix_api/src/imp/time.rs`
  - `api/arceos_posix_api/src/uspace.rs`
  - `doc/logs/2026-04-28-uspace-regression-followup.md`
- 关键决策与理由：
  - 在用户态返回、socket 等待、futex 等待、wait4 等路径补充真实定时器轮询，避免 SIGALRM 只在少数 syscall 边界被观察到，影响 UnixBench looper/alarm 类负载。
  - 保留 socket 等待路径的 yield 行为，撤回 sleep(1ms) 尝试；该尝试会扰动调度并导致 netperf 更早卡住。
  - 在 `shutdown(2)` 成功后主动推进网络轮询并让出调度，解决 netperf TCP_CRR 结束阶段对端 FIN/半关闭状态不及时可见导致的无 trace 卡死。
  - 未采用 workload 名称硬编码或伪通过路径；所有改动都放在通用 syscall/等待/网络推进语义上。
- 验证结果：
  - RISC-V `netperf` 单组已通过：`docker exec arceos-eval-fix env ARCEOS_AUTORUN_ONLY_GROUP=netperf timeout 300s make -C /workspace/arceos run-rv ARCH=riscv64 SMP=1 MEM=1G RV_TESTSUITE_IMG=/workspace/testsuits-for-oskernel/sdcard-rv.img KERNEL_LOG=info`。
  - RISC-V `iperf` 单组已通过：`docker exec arceos-eval-fix env ARCEOS_AUTORUN_ONLY_GROUP=iperf timeout 240s make -C /workspace/arceos run-rv ARCH=riscv64 SMP=1 MEM=1G RV_TESTSUITE_IMG=/workspace/testsuits-for-oskernel/sdcard-rv.img KERNEL_LOG=info`。
  - LoongArch `netperf` 单组已通过，使用直接构建 `kernel-la` 并手动启动 QEMU 的方式传入 `ARCEOS_AUTORUN_ONLY_GROUP=netperf`。
  - LoongArch 全量片段中 basic/busybox 已观察到 `getdents fd:456`、`mount return: 0`、`umount return:0`，此前 mount/getdents 回归在该片段未复现。
  - RISC-V `unixbench` 单组已明显前进，不再停在最初“四个测试后卡住”；已经完成 DHRY2、WHETSTONE、SYSCALL、CONTEXT、PIPE、SPAWN、EXECL、FS_WRITE/READ/COPY SMALL/MIDDLE/BIG，并打印 `Unixbench SHELL1 test(lpm): 0`。
  - 当前 `unixbench` 单组仍未完成：`docker exec -e ARCEOS_AUTORUN_ONLY_GROUP=unixbench arceos-eval-fix timeout 1200s make -C /workspace/arceos run-rv ARCH=riscv64 SMP=1 MEM=1G RV_TESTSUITE_IMG=/workspace/testsuits-for-oskernel/sdcard-rv.img KERNEL_LOG=info` 在 SHELL 阶段后续测试无输出，最终被 `timeout` 终止。
- 当前判断：
  - 网络侧主要退化已经修复并通过轻量化单组验证。
  - UnixBench 剩余问题集中在 shell/multi.sh/looper 组合：`looper` 依赖 SIGALRM，`multi.sh` 会后台启动 `tst.sh` 并 `wait`，内部包含 sort/od/grep/tee/wc/rm 管线；当前表现更像进程生命周期、管线 EOF、wait/reparent/zombie 回收或后台子进程遗留问题，而不是前面 CPU/文件吞吐测试本身的问题。
- 剩余风险与后续任务：
  - 下一步优先做 UnixBench shell 阶段的轻量复现，避免直接跑完整长测。
  - 重点检查父进程因 SIGALRM 退出后，仍在运行的子进程是否正确 reparent、回收、通知等待者，以及管线 fd/pipe 是否能正确关闭并触发 EOF。
  - 修复后先跑 RISC-V `unixbench` 单组，再跑 LoongArch 对应轻量组，最后再进入全量输出日志 `output_rv.md` 和 `output_la.md`。

## 2026-04-29 UnixBench shell 阶段 reparent 修复尝试

- 时间：2026-04-29 上午
- 变更范围与目标：修复 UnixBench shell/multi.sh 阶段可能由父进程 SIGALRM 退出后孤儿子进程不可等待、不可通知、不可回收导致的堵塞。
- 已改文件：
  - `api/arceos_posix_api/src/uspace.rs`
  - `doc/logs/2026-04-28-uspace-regression-followup.md`
- 关键决策与理由：
  - 将 `UserProcess::ppid` 从不可变 `i32` 改为 `AtomicI32`，允许进程生命周期中更新父进程关系。
  - 在最后一个线程退出路径中新增子进程 reparent：优先转交给仍存在的父进程，找不到时转交给 PID 1；同时更新子进程 `ppid` 并唤醒接收方 `wait`。
  - 该改动补齐通用 Unix 进程生命周期语义，避免 UnixBench looper 因 SIGALRM 退出后留下无法被后续 shell/init 观察的后台管线子进程；没有按 workload 名称或路径做特殊处理。
- 验证结果：待执行 RISC-V `unixbench` 单组轻量验证。
- 剩余风险与后续任务：如果 `unixbench` 仍停在 SHELL 阶段，需要继续检查 pipe 引用计数、exec 后 fd 继承/关闭、BusyBox wait 语义与 SIGCHLD 投递路径。

## 2026-04-29 UnixBench shell 轻量复现入口

- 时间：2026-04-29 上午
- 变更范围与目标：为了避免每次定位 UnixBench SHELL 阶段都重复执行前面的长耗时基准，给自动测试入口增加默认关闭的单命令 autorun 调试能力。
- 已改文件：
  - `examples/shell/src/cmd.rs`
  - `api/arceos_posix_api/src/uspace.rs`
  - `doc/logs/2026-04-28-uspace-regression-followup.md`
- 关键决策与理由：
  - 新增 `ARCEOS_AUTORUN_COMMAND` 和 `ARCEOS_AUTORUN_CWD` 编译期环境变量；未设置时原有官方分组执行路径完全不变。
  - 新增默认关闭的 `ARCEOS_TRACE_PROCESS` 进程生命周期追踪，用于观察 add_child、exit、reparent、reap 边界，避免盲目猜测 wait/pipe 卡点。
  - 该入口只用于缩短诊断反馈周期，不改变 syscall 语义，也不按 workload 名称伪造成功。
- 验证结果：待运行 RISC-V shell 子命令复现。
- 剩余风险与后续任务：诊断完成后需要确认是否保留该通用调试入口；若不需要，应在最终提交前移除或明确记录用途。

## 2026-04-29 SIGCHLD 生成语义修复

- 时间：2026-04-29 上午
- 变更范围与目标：修复 UnixBench `multi.sh` 后台 job 在子进程全部退出后仍不继续的问题。
- 已改文件：
  - `api/arceos_posix_api/src/uspace.rs`
  - `doc/logs/2026-04-28-uspace-regression-followup.md`
- 关键决策与理由：
  - 轻量复现 `./looper 20 ./multi.sh 8 | ./busybox cat` 显示：`looper` 能按时输出 `COUNT|0|1|lps`，后台 `tst.sh` 子 shell 会退出并被各自父进程回收，但 `multi.sh` 在启动后台 job 后只做了一次 `wait4(-1, WNOHANG)`，随后没有再次进入 `wait4`。
  - 这说明 BusyBox shell 在后台 job 等待中依赖 SIGCHLD 唤醒，而原实现只有在父进程安装 SIGCHLD handler 时才调用 `deliver_user_signal`。
  - Linux 语义下，默认 SIGCHLD 也应被生成；若未阻塞则按默认忽略处理，若被阻塞或被同步等待则应可唤醒 `sigtimedwait`/相关等待路径。因此改为：除显式 `SIG_IGN` 外，子进程退出都向父进程投递 SIGCHLD。
- 验证结果：待重新运行 UnixBench shell 轻量命令。
- 剩余风险与后续任务：尚未实现 `SA_NOCLDWAIT` 的自动回收语义；如果后续测例覆盖该行为，需要单独补齐。
