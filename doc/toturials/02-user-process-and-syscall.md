# 02 | UserProcess 生命周期与系统调用分发

本章重点是 `uspace.rs` 中“进程对象”和 `user_syscall` 的主控流程。

## 1. `UserProcess` 新增字段：为什么加这几个？

### `mount_table: Mutex<crate::linux_fs::MountTable>`

- 进程级挂载状态，不是全局内核挂载树
- 与 `sys_mount/sys_umount2` 对应
- 退出时 `clear()` 回收

### `shm_attachments: Mutex<BTreeMap<usize, i32>>`

- key 是用户映射虚拟地址
- value 是 `shmid`
- 用于 `shmdt` 与进程退出时批量清理

### `itimer_real_deadline_us / itimer_real_interval_us`

- 以微秒为单位记录 compat `ITIMER_REAL` 的超时状态
- 与 `setitimer` / `user_return_hook` 走一个时序链

### `child_exit_seq: AtomicUsize` 与 `parent_exit_signal: i32`

- `child_exit_seq` 记录“子进程退出次数”的自增序号；
- `parent_exit_signal` 来自 `clone` 的 `exit_signal`，决定父进程退出通知：
  - `0`：不发信号（常见线程 clone 场景）
  - `SIGCHLD`：常见进程 clone 场景
- 这样可以把“父子关系下的退出通知”从裸返回值，转成更符合 POSIX 的信号/等待行为。

## 2. 进程生命周期关键函数

### `teardown()`

- 旧行为：`self.fds = FdTable::new()`
- 现行为：
  1. `compat_itimer_real_disarm(self)`
  2. `detach_all_compat_shm()`
  3. `aspace.clear()`
  4. `fds.close_all()`
  5. `mount_table.clear()`
- 类比：进程退出时，不只是“关门”，还要清理计时器、共享内存、虚拟内存、FD、挂载上下文。

### `note_thread_exit(code)`

- 当 `live_threads` 归零后才做一次完整清理：
  - 停止计时器
  - 解绑 shm
  - 清空 FD
  - `exit_wait.notify_all`
- 避免在有分支线程时过早释放共享资源

### `notify_parent_exit_signal()`

- 在最后一个线程退出前后调用；
- 找到 `ppid` 对应的线程条目；
- 如果 `parent_exit_signal == SIGCHLD` 且子线程在等待 `sigsuspend`，直接写 `pending_signal`，并 `notify_all`；
- 否则走 `deliver_user_signal` 直接派发；
- 这是 `sys_wait4`/`SIGCHLD` 场景更顺的关键。

### `detach_all_compat_shm()`

- 将 `self.shm_attachments` 整体拿走 (`take`)
- 逐个解除映射（若有映射长度）
- 对应 `shmid` 做 `compat_shm_detach`
- `BTreeMap` 的 key/val 记录确保可逆

### `fork()` 分叉流程中的变化

- 原本只克隆地址空间和 fd；现在新增：
  - 克隆父进程 `shm_attachments`
  - 调用 `compat_shm_clone_attachments` 增加子进程引用计数
  - `itimer` 计时器字段重置

## 3. 系统调用入口 `user_syscall`

`user_syscall(tf, syscall_num)` 是用户态进出口，新增/关注点：

- 新增系统调用映射：
  - `SYS_MOUNT`、`SYS_UMOUNT2`
  - `SYS_read/pwrite/`（含 `preadv/pwritev`）
  - `__NR_fsync`、`__NR_fdatasync`
  - shm 系列：`shmget/shmat/shmdt/shmctl`
- `user_return_hook` 中新增 `compat_itimer_real_poll(ext)`，每次返回前检查 SIGALRM

类比：`user_syscall` 像火车站检票闸机，新增闸道就是把新轨道接进总线路由。

## 4. 系统调用参数入口的设计原则（这版体现）

- 所有系统调用统一返回 `isize`：
  - 成功：实际返回值
  - 失败：`neg_errno(err)`
- 内存访问统一走：
  - `user_bytes` / `user_bytes_mut` 做地址可达性检查
  - 避免直接 deref 用户指针导致 UB
- 与 `linux_fs` 的职责边界：
  - `user_syscall`/`sys_*`：装配参数、分发、errno
  - `linux_fs`：统一语义规则
  - `axfs`：执行真实 I/O

## 5. 你会经常看到的行为变化（对比上一个版本）

- 进程退出不再是“清空 FD 表对象”而是“关闭所有映射对象”
- `FD`/挂载/定时器/共享内存都变成“需要显式回收”的生命周期资源
- `mount` 从“调用底层一次性动作”变成“记录到进程级表”

### 额外关键点：`fork()` 的签名变化

- `fork` 现在是 `fork(parent_exit_signal: i32)`;
- 克隆路径直接把 `parent_exit_signal` 写进子进程对象；
- `sys_clone` 里先把 `exit_signal = flags & 0xff` 解析出来；
- 目前只接收 `0` 或 `SIGCHLD` 两类可接受语义，其他直接走 `ENOSYS`。

## 6. 兼容函数 `sys_setitimer` 与返回行为对接

`sys_setitimer` 在这一节也会出现（细节见第 07 章）：  
它现在明确只支持 `ITIMER_REAL`，并把旧值写回 `itimerval`，新值装进 `UserProcess` 结构。

## 7. `sys_clone` 的分支式行为（fork 样式 vs thread 样式）

`sys_clone` 现在把“传入 flags”拆成两块理解：

- `exit_signal = flags & 0xff`：给 `wait/通知` 用，后续会传给 `process.fork(exit_signal)`。
- `clone_flags = flags & !0xff`：决定是否走进程克隆还是线程克隆分支。

### 分支 A：fork-like 分支

- 条件：`clone_flags` 只含允许集（基本可为 0 / `CLONE_VM|CLONE_VFORK` 组合），并满足 `CLONE_PARENT_SETTID/CHILD_SETTID/CLEARTID` 依赖参数合法。
- 关键行为：
  - 只接受 `exit_signal` 为 `0` 或 `SIGCHLD_NUM`，其他直接 `ENOSYS`；
  - `PTID/CTID` 空指针按对应 flag 检查，非法直接 `EFAULT`；
  - 调 `process.fork(exit_signal)` 生成新 `UserProcess`；
  - 子进程继承 `shm_attachments` 与信号遮罩；
  - 返回新 `pid` 给父进程。

类比：这像“复制一整套家庭资产”的开关（包括挂载表、fd 表、shm），只是把 `exit_signal` 带给子女以后让父进程能知道“哪种退出信号要发”。

### 分支 B：thread-like 分支

- 条件：`exit_signal == 0`，并且 `clone_flags` 必须包含 `THREAD_REQUIRED_FLAGS`；
- 不满足：返回 `ENOSYS`。
- 关键行为：
  - 不走 `process.fork`，复用同一个 `UserProcess`，新建 `Task`；
  - 仅创建 `UserTaskExt`，用于一个新 TID 的执行上下文；
  - `child_set_tid/child_clear_tid` 按 flag 写/回写；
  - 任务启动后返回 `tid`。

### 这层和之前版本差异

- 上一版只强调“能走就走线程/进程分支”，现在要求 `exit_signal` 进入了可控路径；
- `fork_like` 路径会把 `parent_exit_signal` 写入子进程对象，和 `notify_parent_exit_signal` 形成一条完整的 `wait/sigsuspend` 传播链；
- 这条链是本轮 `SIGCHLD` 正常性的关键。
