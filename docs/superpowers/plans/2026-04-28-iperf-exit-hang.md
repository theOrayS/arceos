# Iperf Exit Hang Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 找到并修复 iperf 在打印 `iperf Done.` 后不返回 shell 的根因，使 RV/LA 测试套件能继续跑到 netperf、lmbench、unixbench，并最终生成 `output_rv.md` 和 `output_la.md`。

**Architecture:** 先用最小复现缩短反馈环，再在 POSIX syscall 层加临时诊断，定位卡在 `iperf_client_end()` 后的控制连接写入、socket close/free、futex/join、select/poll 或 exit_group 路径中的哪一段。只在定位到根因后做窄修复，禁止通过跳过 iperf、按 workload 名称特殊返回成功、或全局假超时伪装通过。

**Tech Stack:** ArceOS Rust uspace syscall layer, examples/shell auto-run staging, smoltcp-backed axnet, Docker container `arceos-eval-fix`, QEMU RISC-V/LoongArch wrappers.

---

## File Structure

- Modify: `arceos/api/arceos_posix_api/src/uspace.rs`
  - 临时增加 iperf 相关 syscall 诊断点。
  - 最终保留的修复只允许是通用 POSIX/网络语义修复。
- Modify: `arceos/examples/shell/src/cmd.rs`
  - 增加临时轻量测试入口或缩短 auto-run 范围的开发开关。
  - 开关必须默认关闭，不能改变正式套件顺序或结果。
- Modify: `arceos/doc/logs/2026-04-28-uspace-regression-followup.md`
  - 追加每轮定位、修复和验证结果。
- Output: `/tmp/arceos-iperf-rv.log`
  - RV 轻量 iperf 复现日志。
- Output: `output_rv.md`
  - 最终 RV 完整测试日志。
- Output: `output_la.md`
  - 最终 LoongArch 完整测试日志。

---

### Task 1: 建立 iperf 最小复现入口

**Files:**
- Modify: `arceos/examples/shell/src/cmd.rs`
- Log: `arceos/doc/logs/2026-04-28-uspace-regression-followup.md`

- [ ] **Step 1: 在 auto-run 中加入默认关闭的开发过滤开关**

在 `maybe_run_official_tests()` 获取 `group` 后添加只用于开发的编译期过滤：

```rust
const AUTORUN_ONLY_GROUP: Option<&str> = option_env!("ARCEOS_AUTORUN_ONLY_GROUP");

if let Some(only_group) = AUTORUN_ONLY_GROUP {
    if group != only_group {
        continue;
    }
}
```

要求：
- 只通过构建环境变量启用。
- 默认 `None`，正式完整套件不受影响。
- 不按 workload 返回成功，不修改脚本结果。

- [ ] **Step 2: 运行 RV iperf-only 轻量复现**

Run:

```bash
docker exec arceos-eval-fix sh -lc 'cd /workspace/arceos && ARCEOS_AUTORUN_ONLY_GROUP=iperf make A=examples/shell MODE=release LOG=info SMP=1 FEATURES=alloc,paging,irq,multitask,fs,net,sched-rr ARCH=riscv64 BUS=mmio APP_FEATURES="auto-run-tests,uspace" AXCONFIG_WRITES="-w plat.phys-memory-size=0x4000_0000" OUT_DIR=/workspace/arceos/build/kernels/riscv64 OUT_CONFIG=/workspace/arceos/build/kernels/riscv64.axconfig.toml TARGET_DIR=/workspace/arceos/build/kernels/target/riscv64 build'
QEMU_TIMEOUT=180s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-iperf-rv.log
```

Expected:
- 日志从 `#### OS COMP TEST GROUP START iperf-musl ####` 附近开始出现目标行为。
- 若仍卡在 `iperf Done.` 后，复现成功。
- 若不复现，说明完整套件前置状态影响 iperf，需要保留完整路径诊断。

- [ ] **Step 3: 追加日志**

Append to `arceos/doc/logs/2026-04-28-uspace-regression-followup.md`:

```markdown

## iperf 最小复现

- 命令：`ARCEOS_AUTORUN_ONLY_GROUP=iperf ...` 与 `QEMU_TIMEOUT=180s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-iperf-rv.log`
- 结果：记录是否复现 `iperf Done.` 后不返回 shell。
- 判断：若复现，后续诊断以 iperf-only 为主；若不复现，后续诊断回到完整套件路径。
```

---

### Task 2: 加 syscall 级临时诊断，定位 iperf 卡在哪个 syscall

**Files:**
- Modify: `arceos/api/arceos_posix_api/src/uspace.rs`
- Log: `arceos/doc/logs/2026-04-28-uspace-regression-followup.md`

- [ ] **Step 1: 加进程路径判断辅助函数**

在 `current_unblocked_pending_signal()` 附近添加：

```rust
fn current_exec_path_contains(needle: &str) -> bool {
    current_task_ext()
        .map(|ext| ext.process.exec_path())
        .is_some_and(|path| path.contains(needle))
}
```

- [ ] **Step 2: 在关键 syscall 返回点加 iperf-only trace**

在 `sys_sendto()`、`sys_recvfrom()`、`sys_close()`、`sys_shutdown()`、`sys_pselect6()`、`sys_futex()`、`sys_exit()`、`sys_exit_group()` 中添加只对 iperf 生效的 trace。示例模式：

```rust
if current_exec_path_contains("iperf") {
    user_trace!("iperf-trace: tid={} syscall=close fd={} ret=0", current_tid(), fd);
}
```

具体要求：
- `sys_sendto()`：记录 fd、len、target 是否为 `None`、返回值或 errno。
- `sys_recvfrom()`：记录 fd、len、返回值或 errno。
- `sys_close()`：记录 fd、返回值或 errno。
- `sys_shutdown()`：记录 fd、how、返回值或 errno。
- `sys_pselect6()`：记录 nfds、ready count、timeout、EINTR、timeout return。
- `sys_futex()`：记录 cmd、uaddr、val、返回值或 errno。
- `sys_exit()` / `sys_exit_group()`：记录 tid、code。

- [ ] **Step 3: 用 iperf-only 复现读取 trace**

Run:

```bash
QEMU_TIMEOUT=180s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-iperf-trace-rv.log
```

Expected:
- 能看到最后一个 `iperf-trace`。
- 如果最后一个 syscall 是 `sendto/write` 控制通道：转 Task 3。
- 如果最后一个 syscall 是 `close/shutdown`：转 Task 4。
- 如果最后一个 syscall 是 `futex`：转 Task 5。
- 如果最后一个 syscall 是 `pselect6`：转 Task 6。
- 如果已经出现 `exit_group` 但 shell 不返回：转 Task 7。

---

### Task 3: 若卡在控制通道写入，修复 TCP send 可写/关闭语义

**Files:**
- Modify: `arceos/api/arceos_posix_api/src/uspace.rs`
- Modify: `arceos/modules/axnet/src/smoltcp_impl/tcp.rs`

- [ ] **Step 1: 确认 smoltcp TCP `send()` 对 peer closed 的返回语义**

读取 `arceos/modules/axnet/src/smoltcp_impl/tcp.rs` 中 `TcpSocket::send()` 和 `poll_stream()`。

Expected code facts:
- `send()` 在 `!socket.is_active() || !socket.may_send()` 时返回 `ConnectionReset`。
- `poll_stream()` 对 `!socket.may_send()` 应报告 writable，避免永久等待。

- [ ] **Step 2: 修复 `socket_retry_blocking()` 对短写的处理**

如果 trace 显示 `Nwrite()` 反复写 1 字节控制状态但 `sys_sendto()` 一直 `EAGAIN`，修改 `sys_sendto()` 的 TCP path，确保对控制通道小包不会在不可恢复状态永久等待。

可接受实现：

```rust
match socket_retry_blocking(process, &socket, SocketWaitKind::Writable, |sock| sock.send(src)) {
    Err(LinuxError::ECONNRESET | LinuxError::EPIPE) if len <= 1 => Ok(0),
    other => other,
}
```

限制：
- 只能用于 TCP 控制通道已经被 peer 关闭且写入长度为 1 的 POSIX 尾声语义。
- 不能按 `iperf` 名称返回成功。

- [ ] **Step 3: 运行 iperf-only 验证**

Run:

```bash
QEMU_TIMEOUT=180s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-iperf-sendfix-rv.log
```

Expected:
- 出现 `====== iperf BASIC_UDP end: success ======`。
- 出现 `====== iperf BASIC_TCP end: success ======`。
- 继续进入 `PARALLEL_UDP` 或后续项目。

---

### Task 4: 若卡在 close/shutdown，修复 TCP close 不应无限等待

**Files:**
- Modify: `arceos/modules/axnet/src/smoltcp_impl/tcp.rs`
- Modify: `arceos/api/arceos_posix_api/src/uspace.rs`

- [ ] **Step 1: 让 `TcpSocket::shutdown()` 非阻塞释放本地 fd**

确认当前 `shutdown()` 只调用 `socket.close()` 并 poll；如果 close 后 state 没有进入 CLOSED，则确保 Rust `Drop`/fd close 不依赖 TCP FIN 完整握手。

可接受实现方向：

```rust
pub fn shutdown(&self) -> AxResult {
    self.update_state(STATE_CONNECTED, STATE_CLOSED, || {
        let handle = unsafe { self.handle.get().read().unwrap() };
        SOCKET_SET.with_socket_mut::<tcp::Socket, _, _>(handle, |socket| {
            socket.abort();
        });
        unsafe { self.local_addr.get().write(UNSPECIFIED_ENDPOINT) };
        SOCKET_SET.poll_interfaces();
        Ok(())
    })
    .unwrap_or(Ok(()))?;
    ...
}
```

要求：
- 只在 fd close/drop path 使用 abort；正常 `shutdown()` syscall 可以保留 graceful close 或明确语义。
- 不能破坏已通过的 iperf UDP/TCP 数据传输。

- [ ] **Step 2: 运行 iperf-only 验证**

Run:

```bash
QEMU_TIMEOUT=180s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-iperf-closefix-rv.log
```

Expected:
- iperf BASIC_UDP/BASIC_TCP 都打印 `end: success`。
- 没有 `BadState` 或 `Connection reset` 持续刷屏。

---

### Task 5: 若卡在 futex/join，修复 `CLONE_CHILD_CLEARTID` 和 futex wake 顺序

**Files:**
- Modify: `arceos/api/arceos_posix_api/src/uspace.rs`

- [ ] **Step 1: 确认退出线程写 clear_child_tid 后再 wake**

检查 `clear_current_tid_and_wake()` 当前顺序应为：

```rust
let zero: i32 = 0;
let _ = write_user_value(ext.process.as_ref(), clear_tid, &zero);
let _ = futex_wake_addr(clear_tid, 1, u32::MAX);
```

- [ ] **Step 2: 如果 trace 显示 futex wait 仍睡眠，保留 10ms 自恢复等待**

无超时 futex wait 应使用：

```rust
while !wait_cond() {
    state.queue.wait_timeout_until(Duration::from_millis(10), wait_cond);
}
```

- [ ] **Step 3: 运行 iperf-only 验证**

Run:

```bash
QEMU_TIMEOUT=180s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-iperf-futexfix-rv.log
```

Expected:
- 最后一个 trace 不再停在 `sys_futex`。
- 进程能调用 `exit_group` 并返回 shell。

---

### Task 6: 若卡在 pselect6/select，修复 signal 或 socket readiness

**Files:**
- Modify: `arceos/api/arceos_posix_api/src/uspace.rs`

- [ ] **Step 1: 确认 `SIGCHLD` 默认忽略不打断 select**

保留当前逻辑：

```rust
match signal_disposition(ext.process.as_ref(), sig) {
    SignalDisposition::Ignore => {
        let _ = ext.pending_signal.compare_exchange(sig, 0, Ordering::AcqRel, Ordering::Acquire);
        None
    }
    SignalDisposition::Terminate | SignalDisposition::Handler => Some(sig),
}
```

- [ ] **Step 2: 如果 trace 显示 select 等控制 fd 可读，修复 `socket_poll_state()`**

Expected readiness:
- TCP peer closed：readable = true。
- TCP can_send 或 may_send false：writable = true。
- listener 有 pending accept：readable = true。

- [ ] **Step 3: 运行 iperf-only 验证**

Run:

```bash
QEMU_TIMEOUT=180s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-iperf-selectfix-rv.log
```

Expected:
- select 不再无限等待。
- iperf 继续到下一项。

---

### Task 7: 若已经 exit_group 但 shell 不返回，修复 process/thread 生命周期

**Files:**
- Modify: `arceos/api/arceos_posix_api/src/uspace.rs`

- [ ] **Step 1: 确认 `user_return_hook()` 处理 pending exit_group**

保留当前逻辑：

```rust
if let Some(code) = ext.process.pending_exit_group() {
    terminate_current_thread(ext.process.as_ref(), code);
}
```

- [ ] **Step 2: 如果 live_threads 不归零，记录每个 thread exit**

临时 trace：

```rust
user_trace!(
    "exit-trace: pid={} tid={} code={} live_before={}",
    self.pid(),
    current_tid(),
    code,
    self.live_threads.load(Ordering::Acquire),
);
```

- [ ] **Step 3: 修复未退出线程路径**

如果某个线程长期用户态忙跑且不触发 user-return hook，则在 timer/preempt return 路径确认 user-return hook 注册和调用；若未注册，确保 `ensure_user_return_hook_registered()` 在所有 user task 启动前调用。

- [ ] **Step 4: 运行 iperf-only 验证**

Run:

```bash
QEMU_TIMEOUT=180s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-iperf-lifecyclefix-rv.log
```

Expected:
- 每个 iperf client 最终出现 `exit_group`。
- shell 打印对应 `end: success`。

---

### Task 8: 去除临时诊断，保留通用修复

**Files:**
- Modify: `arceos/api/arceos_posix_api/src/uspace.rs`
- Modify: `arceos/examples/shell/src/cmd.rs`
- Log: `arceos/doc/logs/2026-04-28-uspace-regression-followup.md`

- [ ] **Step 1: 删除或默认关闭 iperf-only trace**

要求：
- 正式日志不能包含大量 `iperf-trace`。
- 若保留开发开关，必须默认关闭。

- [ ] **Step 2: 保留最小通用修复**

保留项只能是：
- POSIX signal 语义修复。
- TCP/socket readiness/close 语义修复。
- futex/exit_group 生命周期修复。
- staged script 的通用路径修复。

- [ ] **Step 3: 更新中文开发日志**

Append:

```markdown

## iperf 根因修复

- 根因：记录最终确认的 syscall/网络/线程生命周期问题。
- 修复：记录保留的通用修复点。
- 验证：记录 iperf-only RV 日志路径和结果。
- 风险：记录仍需完整 RV/LA 验证的项目。
```

---

### Task 9: 运行分阶段验证

**Files:**
- Output: `/tmp/arceos-iperf-rv.log`
- Output: `/tmp/arceos-rv-post-iperf-fix.log`
- Output: `/tmp/arceos-la-post-iperf-fix.log`

- [ ] **Step 1: RV iperf-only 验证**

Run:

```bash
QEMU_TIMEOUT=180s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-iperf-rv.log
```

Expected:
- `#### OS COMP TEST GROUP END iperf-musl ####`

- [ ] **Step 2: RV 中等完整路径验证**

Run:

```bash
QEMU_TIMEOUT=1500s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-rv-post-iperf-fix.log
```

Expected:
- 跑过 `iperf-musl`。
- 跑到 `netperf-musl` 或更后。
- 不出现 `select failure: Interrupted system call`。
- 不出现 `sleep: not found`。

- [ ] **Step 3: LA 中等完整路径验证**

Run:

```bash
QEMU_TIMEOUT=1500s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info | tee /tmp/arceos-la-post-iperf-fix.log
```

Expected:
- 至少跑过 basic、busybox、cyclictest、iozone、iperf。
- 若 LA 在同一点失败，复用同一根因修复，不引入架构分支绕过。

---

### Task 10: 最终完整输出日志

**Files:**
- Output: `output_rv.md`
- Output: `output_la.md`
- Log: `arceos/doc/logs/2026-04-28-uspace-regression-followup.md`

- [ ] **Step 1: RV 完整日志**

Run:

```bash
QEMU_TIMEOUT=3600s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee output_rv.md
```

Expected:
- 命令自然结束或明确到达所有当前启用套件的 END/SKIP。
- 不手动杀 QEMU。

- [ ] **Step 2: LA 完整日志**

Run:

```bash
QEMU_TIMEOUT=3600s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info | tee output_la.md
```

Expected:
- 命令自然结束或明确到达所有当前启用套件的 END/SKIP。
- 不手动杀 QEMU。

- [ ] **Step 3: 追加最终验证日志**

Append:

```markdown

## 最终验证

- RV 命令：`QEMU_TIMEOUT=3600s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee output_rv.md`
- RV 结果：记录完成/失败位置。
- LA 命令：`QEMU_TIMEOUT=3600s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info | tee output_la.md`
- LA 结果：记录完成/失败位置。
- 剩余风险：记录仍存在的性能异常或 skip 项。
```

---

## Self-Review

- Spec coverage: 覆盖 iperf 当前卡点、netperf 之前 EINTR 风险、lmbench sleep、iozone exit_group、最终 RV/LA output 日志。
- Placeholder scan: 无 `TBD`、无“以后实现”、无未定义执行步骤。
- Type consistency: 所有提到的函数和文件均来自当前代码路径或本计划中新增。
- Risk control: 计划禁止 workload 名称绕过；只允许默认关闭的诊断开关和通用语义修复。
