# 08 | 信号与等待：sigtimed/sigsuspend 和 SIGCHLD 传播

这一章专门处理本轮更新里新增的信号路径。

你可以把它想成两层：

- **事件生产者**：`kill/tkill/tgkill/sys_clone` 等地方把信号写进 `pending_signal`
- **事件消费者**：`user_return_hook`/`sigreturn` 在用户态返回时取出并注入

在中间，`sigsuspend` 负责把“线程先睡一会儿，等信号/子进程退出”这个行为做成可控。

## 1. 关键状态在哪

在 `UserTaskExt` 里新增了几个和等待相关的字段：

- `signal_wait: WaitQueue`
- `sigsuspend_active: AtomicBool`
- `signal_mask: AtomicU64`
- `pending_signal: AtomicI32`
- `pending_sigreturn: Mutex<Option<TrapFrame>>`

可以类比成：  
信号有一个“邮箱”（`pending_signal`），有一个“静音开关”（`signal_mask`），
有一个“排队/唤醒器”（`signal_wait`）。

## 2. 信号入队：`deliver_user_signal`

### 作用

- 给指定线程（`UserThreadEntry`）投递信号，路径是：
  - `sig == 0` 直接成功返回（等价空操作）
  - 否则写 `pending_signal`
- 立即调用 `signal_wait.notify_all(true)`，防止 `sigsuspend` 无法醒来
- `SIGCANCEL` 还会尝试唤醒 futex 等待队列（兼容取消语义）

### 类比

就像给某个人打了一个“未读消息提醒”，并且帮他按响铃，避免他在睡眠里卡住。

## 3. 什么时候算“有未屏蔽信号”

新增了一个小工具：

- `has_unblocked_pending_signal(ext)`
- 读 `pending_signal`，再检查当前掩码是否阻塞该信号

这个函数主要被 `sys_rt_sigsuspend` 用于醒来条件判断。

## 4. 返回路径挂钩：`ensure_user_return_hook_registered` / `user_return_hook`

### `ensure_user_return_hook_registered`

- 只在首次使用时注册一次全局返回钩子，避免重复注册。

### `user_return_hook(tf)`

每次从内核返回用户态前都会执行：

1. `compat_itimer_real_poll(ext)`（ITIMER_REAL）
2. 若有 `pending_sigreturn`，恢复 trapframe
3. 若信号帧空位，检查 `pending_signal`
4. 未被阻塞则注入信号帧（riscv64/loongarch64）

信号帧注入时会用架构相关的 `inject_pending_signal`（见下一节）。

## 5. 注入层：`make_riscv_siginfo` / `make_loongarch_siginfo` / `inject_pending_signal`

本轮新增了 LoongArch 分支，和 riscv 一致走同一套思路：

- 把 signal/tcode/tid 写进内核定义帧；
- 申请并清零内存页；
- 构造 `TrapFrame` 的返回寄存器布局；
- 把旧寄存器打包进 `pending_sigreturn`，等 `rt_sigreturn` 时恢复。

这里的核心差异是：**架构不同，信号帧结构不同，但逻辑一致**。

## 6. `sys_rt_sigreturn` 的恢复动作（LoongArch 兼容）

这个入口在不同架构上语义不完全一致：

- riscv64：走通用恢复逻辑，优先恢复上下文；
- loongarch64：新增了兼容实现：
  - 从 `process.signal_frame` 取出 `LoongArchSignalFrame` 地址；
  - 读取用户内核态保存的 `saved_mask`；
  - 从 `pending_sigreturn` 取出回退用 trapframe；
  - 恢复 `signal_mask` 为 `saved_mask`，清 `signal_frame`；
  - 继续返回 `rt_sigreturn` 到用户态。

类比：这像“把门牌再挂回原位”，不仅恢复寄存器，还把信号屏蔽掩码按保存副本恢复。

## 7. `sys_rt_sigsuspend`（本轮重要）

这个接口是“等待某个事件的安全入口”，当前实现是：

1. 校验输入：`mask != 0` 且 `sigsetsize` 不可过小；
2. 读取新掩码，暂存旧掩码；
3. 设置 `sigsuspend_active = true`；
4. `signal_wait.wait_until(...)` 等待两个条件之一：
   - 有未屏蔽待处理信号
   - 或父子进程退出事件（`SIGCHLD`）发生
5. 如有需要，构造一个 `SIGCHLD`；
6. 恢复旧 mask，清 `sigsuspend_active`；
7. 返回 `-EINTR`。

可以把它理解为：  
`sigsuspend` 是“换一个临时掩码进门铃模式”，等到有人按铃要么超时（当前代码里没有超时）后直接打断。

## 8. 进程退出信号传播：`notify_parent_exit_signal` 和 `child_exit_seq`

`UserProcess` 在 `note_thread_exit` 最后一线程离场前会执行：

- `compat_itimer_real_disarm`
- `detach_all_compat_shm`
- FD 清理
- 通知父子事件

`notify_parent_exit_signal` 的行为：

- 找到父 `Tid`；
- `parent_exit_signal == SIGCHLD` 时：
  - `child_exit_seq++`
  - 若父在 `sigsuspend` 且 SIGCHLD 未阻塞，给 `pending_signal` 直接塞 SIGCHLD 并唤醒
- 非 SIGCHLD 时，走 `deliver_user_signal`

`child_exit_seq` 是“变化号”，用于 `wait/sigsuspend` 判断是否有新子进程结束。

## 9. 与 `wait4` 的关系（建议联动阅读）

- `sys_wait4` 会在 `process.wait_child` 成功移除后返回；
- 这类场景下子进程退出前后的信号可见性由 `parent_exit_signal` + `sigsuspend_active`+`child_exit_seq` 三者共同决定。
- 如果你只看 `sys_wait4`，可能看不见 `SIGCHLD` 触发链；看完本章能串起来。

## 10. 函数速查（本章）

- `UserTaskExt::signal_wait`
- `UserTaskExt::sigsuspend_active`
- `deliver_user_signal`
- `has_unblocked_pending_signal`
- `ensure_user_return_hook_registered`
- `user_return_hook`
- `inject_pending_signal`
- `sys_rt_sigsuspend`
- `sys_rt_sigreturn`
- `UserProcess::notify_parent_exit_signal`
