# iozone / UnixBench 文件 I/O 兼容开发日志

## 基本信息

- 时间：2026-04-26 12:04:35 CST
- 工作范围：ArceOS `examples/shell` 的用户态 Linux ABI/syscall 兼容路径。
- 目标脚本：
  - `testsuits-for-oskernel/scripts/iozone/iozone_testcode.sh`
  - `testsuits-for-oskernel/scripts/unixbench/unixbench_testcode.sh`
- 目标覆盖：
  - iozone：顺序读写、随机读写、反向读、跨步读、`fwrite/fread`、`pwrite/pread`、`pwritev/preadv`。
  - UnixBench：`fstime` 小/中/大文件的写、读、拷贝。
- 总体原则：
  - 不修改 `modules/axfs/**` 来伪造 shell 兼容行为。
  - 不按工作负载名、脚本名、固定路径或命令字符串返回硬编码成功。
  - 所有临时兼容路径使用 `compat_*` 命名，并在兼容策略文档中记录删除条件。

## 变更文件

- `Cargo.lock`
- `Makefile`
- `examples/shell/Cargo.toml`
- `examples/shell/src/cmd.rs`
- `examples/shell/src/linux_fs/fd.rs`
- `examples/shell/src/linux_fs/mod.rs`
- `examples/shell/src/uspace.rs`
- `../run-testsuite-bench-la-direct.sh`
- `../testsuits-for-oskernel/Makefile.sub`
- `docs/development/interfaces/process-scheduler.md`
- `docs/development/interfaces/syscall-inventory.md`
- `docs/development/policies/compatibility.md`
- `docs/superpowers/plans/2026-04-26-iozone-unixbench-fs-io-plan.md`
- `doc/logs/2026-04-26-iozone-unixbench-fs-io.md`

## 问题发现时间线

1. 初始 RV QEMU 运行进入 iozone 后，显式 offset 和 vector I/O 缺口暴露：
   - `pwrite64`、`preadv`、`pwritev` 未接入 syscall dispatcher。
   - 旧 `pread64` 通过 `seek + read` 实现，会扰动 open-file-description 的共享 offset。
2. 补齐显式 offset I/O 后，iozone 继续暴露同步和进程共享状态缺口：
   - automatic 模式报 `fsync: Function not implemented`。
   - throughput 模式报 `Unable to get shared memory segment(shmget)`。
3. 补齐 `fsync/fdatasync` 和最小 `IPC_PRIVATE` SysV shm 后，iozone throughput 能进入 parent/child 输出，但底层 VFS 默认 `fsync` 仍打印 `AxError::InvalidInput`。
4. 定位到当前 axfs 写路径没有独立脏页回写状态，后端 `flush/fsync` 不支持时不应让同步写入模型下的用户态看到失败；因此增加 `compat_sync_unsupported_flush`。
5. 移除 UnixBench skip 后，完整套件可进入 `unixbench-musl`，但前置 CPU benchmark 长时间运行，900 秒内未到 `fstime`。
6. 手动运行 `/musl/fstime` 原参数时发现 `fstime` 不退出；源码确认 `fstime` 依赖 `signal(SIGALRM, ...) + alarm(t)`，musl 走 `setitimer(ITIMER_REAL, ...)`。
7. 初版后台 timer task 在 FIFO/非抢占路径下不能打断 CPU 占用或高频 syscall 循环；最终改为在 user-return hook 中轮询 `ITIMER_REAL` deadline，向当前用户线程挂起 `SIGALRM`。

## 实现摘要

### 显式 Offset 和 Vector I/O

- dispatcher 新增：
  - `__NR_pwrite64`
  - `__NR_preadv`
  - `__NR_pwritev`
- `OpenFileDescription::pread_file` 改为使用 `File::read_at`。
- 新增 `OpenFileDescription::pwrite_file`，使用 `File::write_at`。
- 新增 `advance_explicit_offset`，集中处理显式 offset 加法和溢出。
- `sys_pwrite64`、`sys_preadv`、`sys_pwritev` 在完成部分 I/O 后推进局部 offset，但不更新 OFD 共享 offset。
- `sys_readv`、`sys_writev` 复用共享 iovec loader，顺序 I/O 仍沿用 OFD offset。

### fsync / fdatasync

- dispatcher 新增：
  - `__NR_fsync`
  - `__NR_fdatasync`
- `FdTable::sync` 负责 fd 类型检查。
- `OpenFileDescription::sync_file` 先调用真实后端 `flush`。
- `compat_sync_unsupported_flush` 仅在后端返回 `EINVAL` 或 `EOPNOTSUPP` 时视作当前同步写入模型下已清洁。
- 其他错误继续返回用户态。

### SysV Shm 兼容

- dispatcher 新增：
  - `__NR_shmget`
  - `__NR_shmctl`
  - `__NR_shmat`
  - `__NR_shmdt`
- 新增 `compat_shm_*` registry：
  - 支持 `IPC_PRIVATE` 匿名段。
  - 支持 `shmat(shmid, NULL, 0)`。
  - 支持 `shmdt(addr)`。
  - 支持 `shmctl(shmid, IPC_RMID, ...)` 删除标记。
  - fork 时复制 attachment 计数。
  - 进程退出和地址空间 teardown 时释放 attachment。
- 不支持状态明确拒绝：
  - keyed shm：`EOPNOTSUPP`
  - 显式 attach 地址：`EOPNOTSUPP`
  - `SHM_RDONLY`、`SHM_RND`、`SHM_REMAP`：`EOPNOTSUPP`
  - 未知 flags：`EINVAL`

### UnixBench fstime Timer 出口

- `sys_setitimer(ITIMER_REAL, ...)` 记录 deadline 和 interval。
- `compat_itimer_real_poll` 在 user-return hook 中检查 wall-time deadline。
- 到期后向当前用户线程挂起 `SIGALRM`，让 `fstime` 的 `while (!sigalarm)` 能正常退出。
- 该路径只解决 syscall-heavy 循环，例如 `fstime`；CPU-only UnixBench 前置项仍需要真实抢占式 timer/signal 支撑。

### UnixBench 自动运行

- 删除 `examples/shell/src/cmd.rs` 中对 `unixbench` group 的自动 skip。
- 完整脚本现在会进入 `unixbench-musl`。
- 2026-04-26 12:30 后复测修正了早先判断：RISC-V 下 `dhry2reg`、`whetstone`、`syscall` 可以前进；真正卡点是后续 `context1 | grep | tail | awk` 管道和 `pipe` 测项暴露出的 pipe/fd 生命周期语义缺口。
- 已补齐 pipe 缓冲容量和进程退出时 fd 释放后，手动完整脚本可打印到 `CONTEXT`、`PIPE`、`SPAWN`，不再停在 UnixBench 前半段。

## 关键决策

| 决策 | 理由 | 删除或迁移条件 |
| --- | --- | --- |
| 显式 offset I/O 放在 `linux_fs::fd::OpenFileDescription` | syscall 层继续负责用户内存和 ABI 参数，OFD 层负责 Linux 可见文件语义 | 后续整体迁移 `FdTable` 时保持同等接口语义 |
| `pread64` 不再使用 `seek + read` | Linux `pread*` 不应改变共享 OFD offset | 已完成，无需兼容保留 |
| `compat_sync_unsupported_flush` 只吞 `EINVAL/EOPNOTSUPP` | 当前后端写入同步完成，VFS 默认 `fsync` 不支持不代表数据未写入 | axfs 暴露真实 fsync/writeback 能力后删除 |
| `compat_shm_*` 只支持 private anonymous shm | iozone throughput 只需要父子进程共享一段匿名内存 | axmm/IPC 子系统提供真实 SysV shm 对象模型后删除 |
| `compat_itimer_real_*` 放在 user-return hook | FIFO/非抢占下后台 timer task 不能可靠打断 `fstime` 循环 | timer/signal 子系统支持真实 interval timer 和抢占式 signal delivery 后删除 |
| pipe 缓冲提升到 4096 字节 | UnixBench `pipe` 会在单进程内先 `write(512)` 再 `read(512)`；256 字节容量会让写端先阻塞，读端永远无法执行 | 后续实现真实 pipe capacity、`PIPE_BUF`、partial write 和 `O_NONBLOCK` 语义时迁移 |
| 最后一个用户线程退出时立即关闭 fd 表 | Linux 进程退出应释放描述符；若等到 `wait4` 才释放，shell pipeline 会因未回收的 zombie 继续持有管道写端而等不到 EOF | 迁移到正式 process resource teardown 状态机时保留同等语义 |
| 不修改测试脚本绕过前置 UnixBench benchmark | 保持对真实测试路径的可解释性，不引入脚本名特判 | 后续补齐 CPU benchmark 所需 timer/signal 能力后完整脚本应自然前进 |

## 验证记录

### 构建验证

- `docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info`
  - 结果：通过，生成 `/workspace/arceos/kernel-rv`。
- `docker exec arceos-eval-fix make -C /workspace/arceos kernel-la KERNEL_LOG=info`
  - 结果：通过，生成 `/workspace/arceos/kernel-la`。
- `git -C arceos diff --check`
  - 结果：通过。
- `docker exec -w /workspace/arceos arceos-eval-fix cargo test -p arceos-shell --features uspace linux_fs::fd::tests::explicit_offset -- --nocapture`
  - 结果：不可作为有效验证。
  - 原因：该 crate 的 host/x86 test 目标会进入既有 arch/uspace 条件编译错误；本轮以目标架构 kernel build 与 QEMU 工作负载作为有效验证。

### iozone RV QEMU 验证

命令：

```sh
QEMU_TIMEOUT=900s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info
```

日志位置：

- `/tmp/arceos-iozone-unixbench-rv.log`

结果摘要：

| 覆盖点 | 脚本命令 | 结果 |
| --- | --- | --- |
| automatic | `./iozone -a -r 1k -s 4m` | 完成，输出 write/rewrite/read/reread、random read/write、backward read、stride read、fwrite/fread 等列 |
| write/read throughput | `./iozone -t 4 -i 0 -i 1 -r 1k -s 1m` | 完成 |
| random read/write throughput | `./iozone -t 4 -i 0 -i 2 -r 1k -s 1m` | 完成 |
| read-backwards throughput | `./iozone -t 4 -i 0 -i 3 -r 1k -s 1m` | 完成 |
| stride-read throughput | `./iozone -t 4 -i 0 -i 5 -r 1k -s 1m` | 完成 |
| fwrite/fread throughput | `./iozone -t 4 -i 6 -i 7 -r 1k -s 1m` | 完成 |
| pwrite/pread throughput | `./iozone -t 4 -i 9 -i 10 -r 1k -s 1m` | 完成，看到 `pwrite writers` 和 `pread readers` 输出 |
| pwritev/preadv throughput | `./iozone -t 4 -i 11 -i 12 -r 1k -s 1m` | 脚本执行到该段，但当前 iozone 二进制打印 `Selected test not available on the version.`，不能作为内核运行覆盖证据 |

观察：

- `fsync` 不再以用户态错误阻断 iozone。
- `shmget/shmat/fork` 路径能支撑多进程 throughput 输出。
- 运行期间仍会看到底层 VFS 默认 `AxError::InvalidInput` 日志，这是 `compat_sync_unsupported_flush` 吞掉 unsupported flush 前的后端诊断，不代表用户态命令失败。

### UnixBench fstime RV QEMU 验证

完整自动脚本状态：

- `QEMU_TIMEOUT=900s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info`
- 结果：可进入 `#### OS COMP TEST GROUP START unixbench-musl ####`。
- 阻断点：前置 CPU benchmark 长时间占用，900 秒内未进入 `fstime` 段。
- 结论：不能声明完整 `unixbench_testcode.sh` 通过；本轮只验证 `fstime` 文件项。

聚焦验证方式：

```sh
docker exec -i arceos-eval-fix timeout 420s make -C /workspace/arceos run-rv \
  ARCH=riscv64 SMP=1 MEM=1G \
  RV_TESTSUITE_IMG=/workspace/testsuits-for-oskernel/sdcard-rv.img \
  KERNEL_LOG=info KERNEL_RV_APP_FEATURES=uspace
```

进入 shell 后在 `/tmp` 目录手动运行 `/musl/fstime`，参数与 `unixbench_testcode.sh` 相同。

结果摘要：

| 项目 | 命令 | 关键输出 |
| --- | --- | --- |
| FS_WRITE_SMALL | `/musl/fstime -w -t 20 -b 256 -m 500` | `WRITE COUNT|4968|0|KBps`, `TIME|20.0` |
| FS_READ_SMALL | `/musl/fstime -r -t 20 -b 256 -m 500` | `READ COUNT|5266|0|KBps`, `TIME|20.0` |
| FS_COPY_SMALL | `/musl/fstime -c -t 20 -b 256 -m 500` | `COPY COUNT|2412|0|KBps`, `TIME|20.0` |
| FS_WRITE_MIDDLE | `/musl/fstime -w -t 20 -b 1024 -m 2000` | `WRITE COUNT|19519|0|KBps`, `TIME|20.1` |
| FS_READ_MIDDLE | `/musl/fstime -r -t 20 -b 1024 -m 2000` | `READ COUNT|20642|0|KBps`, `TIME|20.0` |
| FS_COPY_MIDDLE | `/musl/fstime -c -t 20 -b 1024 -m 2000` | `COPY COUNT|9486|0|KBps`, `TIME|20.0` |
| FS_WRITE_BIG | `/musl/fstime -w -t 20 -b 4096 -m 8000` | `WRITE COUNT|66226|0|KBps`, `TIME|20.1` |
| FS_READ_BIG | `/musl/fstime -r -t 20 -b 4096 -m 8000` | `READ COUNT|66812|0|KBps`, `TIME|20.0` |
| FS_COPY_BIG | `/musl/fstime -c -t 20 -b 4096 -m 8000` | `COPY COUNT|31600|0|KBps`, `TIME|20.0` |

说明：

- read/copy 项前会出现短时 `WRITE COUNT` 或 `READ COUNT`，这是 `fstime` 准备输入文件的预热阶段；真正测项以对应的 `READ COUNT` 或 `COPY COUNT` 为准。
- 初次手动验证 `fstime` 时，`setitimer` 只返回成功但不投递 `SIGALRM`，导致 `fstime` 360 秒内不退出。补齐 `compat_itimer_real_*` 后，九项均按约 20 秒窗口退出。

### UnixBench 卡住问题复测

时间：2026-04-26 12:30-12:38 CST。

触发背景：

- 用户评测现场显示多个 `qemu-system-riscv64/loongarch64` 长时间高 CPU 运行，并停在 `#### OS COMP TEST GROUP START unixbench-musl ####` 后。
- 最初怀疑 CPU-only benchmark 的 `alarm()` 不能投递，但最小复现修正了该判断。

复现与根因：

| 复现命令 | 结果 | 结论 |
| --- | --- | --- |
| `runu /musl/dhry2reg 2` | 输出 `COUNT|530197|1|lps` | RISC-V timer IRQ 的 user-return hook 能让 CPU-only `alarm()` benchmark 退出 |
| `runu /musl/busybox sh ./unixbench_testcode.sh` | 输出 DHRY2、WHETSTONE、SYSCALL 后长时间无 `CONTEXT` | 卡点在 `context1` 或后续 pipe/pipeline，不是 UnixBench 脚本启动 |
| `runu /musl/pipe 2` | 修改前无输出并挂住 | `pipe.c` 单进程先 `write(512)` 再 `read(512)`；原 `PIPE_BUF_SIZE=256` 导致写端阻塞 |
| `runu /musl/context1 10` | 直连输出两个 `COUNT` 并返回 | benchmark 本体可退出，问题出在 shell pipeline/EOF 生命周期 |

修复：

- `examples/shell/src/uspace.rs`
  - `PIPE_BUF_SIZE` 从 `256` 提升到 `4096`。
  - 新增 `FdTable::close_all()`。
  - `UserProcess::note_thread_exit()` 在最后一个用户线程退出时立即：
    - disarm `compat_itimer_real_*`
    - detach `compat_shm_*`
    - close all fd slots
    - notify waiters
  - `UserProcess::teardown()` 改为调用 `close_all()`，避免 wait/reap 阶段重新保留 stdio fd。
- `docs/development/interfaces/filesystem.md`
  - 记录进程退出必须关闭 fd slots，`wait4` 不能成为 pipe EOF 的前置条件。
- `docs/development/interfaces/process-scheduler.md`
  - 记录 exit 路径释放 fd slots。
- `docs/development/policies/compatibility.md` 和 `docs/development/interfaces/syscall-inventory.md`
  - 修正 `compat_itimer_real_*` 的 RISC-V CPU-loop 能力描述。

验证：

| 验证命令 | 结果 |
| --- | --- |
| `docker exec -i arceos-eval-fix timeout 520s make -C /workspace/arceos run-rv ARCH=riscv64 SMP=1 MEM=1G RV_TESTSUITE_IMG=/workspace/testsuits-for-oskernel/sdcard-rv.img KERNEL_LOG=info KERNEL_RV_APP_FEATURES=uspace KERNEL_FEATURES=alloc,paging,irq,multitask,fs,net` 后执行 `cd /musl; runu /musl/pipe 2` | 输出 `COUNT|30017|1|lps`，不再挂住 |
| 同一 QEMU 会话执行 `runu /musl/busybox sh ./unixbench_testcode.sh` | 输出 `Unixbench CONTEXT test(lps): 11416`、`Unixbench PIPE test(lps): 152279`、`Unixbench SPAWN test(lps): 54`，确认越过原卡点 |
| `docker exec arceos-eval-fix cargo fmt --manifest-path /workspace/arceos/examples/shell/Cargo.toml` | 通过 |
| `docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info` | 通过 |
| `docker exec arceos-eval-fix make -C /workspace/arceos kernel-la KERNEL_LOG=info` | 通过 |
| `git diff --check` | 通过 |

备注：

- 手动从 `/musl` 直接跑脚本时，后续 `fstime` 会报 `Read-only file system`；这是手动验证 cwd 与 autorun staging 不同导致的环境差异。autorun 会将脚本和依赖 staging 到 `/tmp/testsuite/...`，`fstime` 文件项仍以 `/tmp` 聚焦验证为准。
- 本次复测只证明 UnixBench 前半段不再卡在 `context1/pipe`；完整 autorun 的全部 UnixBench 输出仍需在更长 QEMU 窗口中单独收集。

### UnixBench execl 与 fstime 管线二次定位

时间：2026-04-26 16:16:00 CST。

变更范围：

- `examples/shell/src/uspace.rs`
  - `execve` 读取用户态 `envp`，并在新进程初始栈上布置 `argv[]`、`envp[]` 和 auxv。
  - pipe 从空读/满写时的 `yield_now()` 忙轮询改为 pipe-local `WaitQueue` 阻塞/唤醒。
  - pipe 共享状态显式记录 `readers/writers`，关闭最后一个写端唤醒读端，关闭最后一个读端唤醒写端并让写端返回 `EPIPE`。
- `docs/development/interfaces/process-scheduler.md`
  - 记录 `execve` 初始栈包含 `argv/envp/auxv`。
- `docs/development/interfaces/ipc-sync.md`
  - 记录 pipe 空读/满写必须通过 wait queue 阻塞，而不是在 syscall 内忙等。
- `docs/development/interfaces/syscall-inventory.md`
  - 更新 `pipe2` 与 `execve` 当前语义。

根因与决策：

- `EXECL` 失败的直接原因是 `sys_execve` 忽略 `envp`。UnixBench `execl.c` 通过 `getenv("UB_BINDIR")` 构造被执行路径，`UB_BINDIR` 丢失会走到未初始化路径并报 `Exec format error`。
- `fstime | grep | ...` 卡住不是 `fstime` 文件 I/O 本体问题。直接运行 `/musl/fstime -w -t 20 -b 256 -m 500` 能按 20 秒退出；通过 busybox shell 管线运行时，临时诊断显示 `fstime` 卡在测试前的 `sleep(2)`。
- 根因是下游 `grep` 对空 pipe 调用 `read` 后在 syscall 内循环 `yield_now()`，在 FIFO/IRQ 组合下会破坏睡眠任务的 timer wakeup 前进性。pipe 的阻塞条件必须进入 wait queue，由写入、读取和端点关闭进行显式唤醒。
- 该修复不按 UnixBench、`fstime`、路径或命令字符串特判；它补的是 pipe 的通用阻塞/唤醒语义。

聚焦验证：

| 验证命令 | 结果 |
| --- | --- |
| `runu /musl/busybox env UB_BINDIR=. /musl/execl 2` | 输出 `COUNT|4|1|lps`，证明 `execve` envp 已传入 |
| `runu /musl/busybox env UB_BINDIR=./ /musl/execl 2` | 输出 `COUNT|4|1|lps` |
| 手工创建 `/tmp/fspipe.sh`，内容为 `/musl/fstime -w -t 20 -b 256 -m 500 \| /musl/busybox grep -o 'WRITE COUNT\|[[:digit:]]\\+\|'`，然后执行 `runu /musl/busybox sh /tmp/fspipe.sh` | 输出 `WRITE COUNT|5004|`，`runu: exited with status 0` |
| `docker exec arceos-eval-fix cargo fmt --manifest-path /workspace/arceos/examples/shell/Cargo.toml` | 通过 |
| `docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_RV_APP_FEATURES=uspace KERNEL_LOG=info` | 通过；仅有 `maybe_run_official_tests` 在非 autorun 构建中的既有 dead_code warning |

待验证：

- 需要重新跑 autorun 版本的完整 RISC-V wrapper，确认 `EXECL` 后能继续输出 UnixBench `FS_WRITE_* / FS_READ_* / FS_COPY_*`。
- 需要在 LoongArch64 上完成同一路径验证；若 LA signal frame 仍不足以投递 `SIGALRM`，需要单独补 LA 信号帧。

### RISC-V / LoongArch64 完整目标段收敛

时间：2026-04-26 18:41:45 CST。

变更范围：

- `examples/shell/src/uspace.rs`
  - 新增 LoongArch64 用户态 signal frame 注入路径，使用 on-stack trampoline 执行 `rt_sigreturn`。
  - `sys_rt_sigreturn` 增加 LoongArch64 恢复路径，恢复 pending trap frame 和旧 signal mask。
  - user-return hook 在 RISC-V 与 LoongArch64 上统一检查 pending signal。
  - 新增 `rt_sigsuspend` 的 `SIGCHLD` 唤醒路径：只在父线程正处于 `rt_sigsuspend` 且未屏蔽 `SIGCHLD` 时置 pending signal，避免广泛打断普通 wait/pipe 路径。
- `../testsuits-for-oskernel/Makefile.sub`
  - 将 UnixBench `sort.src` 一并复制到 musl/glibc staging 目录，满足 `multi.sh -> tst.sh` 的真实输入文件依赖。
- `Makefile` 与 `../run-testsuite-bench-la-direct.sh`
  - LoongArch64 默认 QEMU 内存调整为 `2G`。
  - LA wrapper 按 `MEM - 256M` 自动设置 `KERNEL_LA_AXCONFIG_WRITES=-w plat.phys-memory-size=...`。
  - 根因是 QEMU LoongArch virt 将前 256M 暴露为低地址 RAM，其余内存才映射到 `0x80000000` 高地址 RAM；`-m 1G` 时高地址 RAM 只有 `0x80000000..0xafffffff`，旧内核配置误以为可用到 `0xbfffffff`，UnixBench shell 压力触到 `0xb0000000` 后发生 `MemoryAccessAddressError`。
- 文档：
  - `docs/development/interfaces/process-scheduler.md` 记录当前 signal/sigsuspend 子集。
  - `docs/development/interfaces/syscall-inventory.md` 记录 `rt_sigsuspend`、LoongArch64 `rt_sigreturn` 和 `setitimer` 状态。
  - `docs/development/policies/compatibility.md` 更新 `compat_itimer_real_*` 的 LA 删除条件。

关键决策：

- 没有通过跳过 UnixBench shell 项规避问题；`multi.sh` 的 `sort.src` 缺失按镜像构建输入修正。
- LoongArch64 signal frame 是最小兼容帧，不声明完整 Linux `ucontext_t` 布局；删除条件仍是真正的 signal/timer 子系统。
- LoongArch64 内存修正跟随 QEMU virt 的实际 RAM 拆分，而不是继续扩大 `plat.phys-memory-size` 的连续高地址假设。

验证结果：

| 架构 | 命令 | 目标结果 |
| --- | --- | --- |
| LoongArch64 | `QEMU_TIMEOUT=1500s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info \| tee /tmp/arceos-la-iozone-unixbench-full.log` | `iozone-musl` 到结束标记；`unixbench-musl` 输出 DHRY2、WHETSTONE、SYSCALL、CONTEXT、PIPE、SPAWN、EXECL、九项 `FS_*`、SHELL1/8/16、ARITHOH/SHORT/INT/LONG/FLOAT/DOUBLE/HANOI/EXEC，并到 `#### OS COMP TEST GROUP END unixbench-musl ####` |
| RISC-V64 | `QEMU_TIMEOUT=1500s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info \| tee /tmp/arceos-rv-iozone-unixbench-full.log` | `iozone-musl` 到结束标记；`unixbench-musl` 输出同一组 UnixBench 指标并到 `#### OS COMP TEST GROUP END unixbench-musl ####` |

日志摘录：

- LoongArch64：`LoongArch kernel AXCONFIG_WRITES: -w plat.phys-memory-size=0x70000000`，启动后 free memory 为 `[PA:0x80133000, PA:0xf0000000)`。
- LoongArch64 UnixBench FS 项：
  - `FS_WRITE_SMALL 14019`
  - `FS_READ_SMALL 14865`
  - `FS_COPY_SMALL 7077`
  - `FS_WRITE_MIDDLE 53293`
  - `FS_READ_MIDDLE 55832`
  - `FS_COPY_MIDDLE 26910`
  - `FS_WRITE_BIG 180788`
  - `FS_READ_BIG 181626`
  - `FS_COPY_BIG 86176`
- RISC-V64 UnixBench FS 项：
  - `FS_WRITE_SMALL 6668`
  - `FS_READ_SMALL 7358`
  - `FS_COPY_SMALL 3379`
  - `FS_WRITE_MIDDLE 29298`
  - `FS_READ_MIDDLE 31935`
  - `FS_COPY_MIDDLE 15087`
  - `FS_WRITE_BIG 102243`
  - `FS_READ_BIG 111758`
  - `FS_COPY_BIG 53107`

备注：

- 两个 wrapper 在目标 `unixbench-musl` 结束后都会继续进入 glibc/其他非本轮目标套件；RISC-V 在 glibc busybox 运行中手动停止了 QEMU，LoongArch64 在 glibc 非目标段自然关机。
- UnixBench `EXEC` 仍打印若干 `exec /bin/true failed`，但脚本最终输出 `Unixbench EXEC test(lps)`；这不是本轮文件 I/O 覆盖点。
- iozone `-i 11/-i 12` 仍由当前二进制打印 `Selected test not available on the version.`；syscall handler 已存在，但需要后续用支持该测项的 iozone 或最小程序单独验证。

## 当前能力边界

- iozone 的 pwritev/preadv syscall handler 已实现，但当前测试镜像中的 iozone 二进制不提供 `-i 11/-i 12` 测项，仍需后续独立运行验证。
- `compat_shm_*` 不是完整 SysV IPC：
  - 没有 key namespace。
  - 没有权限模型。
  - 没有 `shmctl` metadata 查询。
  - 不支持显式 attach 地址。
  - 不支持只读 attach。
- `compat_itimer_real_*` 不是完整 POSIX interval timer：
  - 只支持 `ITIMER_REAL`。
  - 依赖 user-return hook 轮询；RISC-V timer IRQ 会触发 user-return，因此已能覆盖 `dhry2reg` 这类 CPU-only alarm 循环。
  - LoongArch64 当前有最小 signal frame/trampoline，已覆盖 UnixBench alarm handler；完整 signal frame 布局仍需后续替换。
- pipe 当前已具备 4096 字节容量、读写端计数、空读/满写 wait queue 阻塞和端点关闭唤醒；`O_NONBLOCK`、大写入 partial 行为、`SIGPIPE` 和 poll/epoll 级语义仍是后续工作。
- `readv/writev`、`preadv/pwritev` 的完整 Linux errno 顺序、partial iovec 行为和 `preadv2/pwritev2` flags 仍是 LTP 级别后续工作。

## 后续任务

1. 用支持 `-i 11/-i 12` 的 iozone 构建，或写最小用户程序，补 `pwritev/preadv` 的运行验证。
2. 为 `ITIMER_REAL` 增加正式 timer/signal 子系统实现，替换 RISC-V/LoongArch64 当前最小 signal frame 后删除 `compat_itimer_real_*`。
3. 将 `compat_shm_*` 迁移到真实 SysV shm/共享 VM object 模型。
4. 补 pipe 的 `O_NONBLOCK`、大写入 partial 行为、poll/epoll 可观察状态和更完整的 EOF/`SIGPIPE` 语义。
5. 梳理完整 Linux signal frame/ucontext 布局，补 LTP signal/timer 级验证。
6. 在 fd/OFD 迁移阶段继续收敛 `FdTable` 和 path resolver 到 `linux_fs` 边界，避免 `uspace.rs` 继续膨胀。
