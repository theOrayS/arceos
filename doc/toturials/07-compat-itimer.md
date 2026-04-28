# 07 | 兼容定时器：ITIMER_REAL 与 SIGALRM

本轮只补了“窄兼容”路径：`setitimer(ITIMER_REAL)`。  
目标是让依赖 `alarm`/`setitimer` 的 workload 能获得可用行为，不引入完整 POSIX timer 子系统。

类比：  
你可以把它理解为“只有一个闹钟，指向 SIGALRM；只支持 wall-clock 计时，不做 POSIX 定时器全家桶”。

## 1. 现有场景下的需求

BusyBox / unixbench 里的某些循环会直接用 `setitimer` + `SIGALRM`。  
如果只返回 `ENOSYS`，这类测试很容易停在“计时逻辑等待”里。  
因此增加了一个进程内计时器状态，让 `user_return_hook` 上统一处理超时信号。

## 2. 数据字段新增

在 `UserProcess` 中新增：

- `itimer_real_deadline_us: AtomicU64`：下一次触发的绝对时间（微秒）；
- `itimer_real_interval_us: AtomicU64`：重复间隔（微秒），0 表示一次性。

它们在 `clone/fork` 时重置为 0，退出时清零，防止“僵尸定时器”污染下一次进程。

## 3. 时间转换工具

### 3.1 `timeval_to_duration(tv)`

输入 `timeval` 到 `Duration`：

- 校验 `tv_sec >= 0`；
- `tv_usec` 在 `[0, 1_000_000)`；
- 不满足 -> `EINVAL`；
- 输出 `Duration::new(tv_sec, tv_usec * 1000)`。

### 3.2 `duration_to_timeval(duration)`

输出 `timeval`（秒 + 微秒）；
用于 `getitimer` 风格返回和内部状态快照。

### 3.3 `duration_to_micros(duration)`

将 `Duration` 截断到 `u64` 微秒。

### 3.4 `compat_itimer_real_remaining(process)`

用当前时间 `wall_time` 计算剩余：

- `remaining = deadline - now`；
- 如果已到期 -> 0；
- 组装并返回 `itimerval{it_value, it_interval}`。

## 4. timer 生命周期函数

### 4.1 `compat_itimer_real_arm(process, initial, interval)`

设置下一次 deadline：

- `deadline = now + initial`;
- `interval = interval_us`。

注释里说明了这是兼容路径：当前只服务 alarm 风格需求。

### 4.2 `compat_itimer_real_disarm(process)`

将 deadline 和 interval 都清 0。  
`setitimer` 传 `it_value=0`、进程退出/结束都调用它。

### 4.3 `compat_itimer_real_poll(ext)`

挂在 `user_return_hook`。每次用户态返回到内核时检测：

1. 读 deadline；
2. `now < deadline` 则返回；
3. 到期：
   - 如果 `interval == 0`：一次性清零；
   - 如果 `interval > 0`：`deadline = now + interval` 做周期续跳；
4. 通过 `ext.pending_signal` 写入 `SIGALRM`。

这样信号送达在返回路径上进行，不改变 syscall 的主流程。

## 5. `sys_setitimer(process, which, new_value, old_value)`

核心规则：

- 只支持 `which == ITIMER_REAL`；
- `old_value != 0` 时先写回 `compat_itimer_real_remaining`；
- `new_value` 必须可读，否则 `EFAULT`；
- 解析 `it_value / it_interval`；
- `it_value == 0` => `compat_itimer_real_disarm`；
- 否则 `compat_itimer_real_arm(initial, interval)`；
- 成功返回 0。

### 为什么不支持别的 timer？

其它 timer 类型（`ITIMER_VIRTUAL`, `ITIMER_PROF`）需要与调度/CPU 时间耦合，不在当前 compat 目标内。  
所以本实现以明示失败策略降低行为错配风险：直接 `EINVAL`。

## 6. 与返回路径钩子绑定

#### `ensure_user_return_hook_registered`

进程首次运行时注册 `user_return_hook`。  
这样每次用户态返回都会检查定时器到期与信号注入。

#### `user_return_hook`

顺序逻辑是：

1. 先 `compat_itimer_real_poll(ext)`；
2. 再处理 `sigreturn` 恢复；
3. 再处理信号注入（平台差异路径）。

`SIGALRM` 的注入时机因此比较靠近用户态边界，兼容主循环等待行为。

## 7. 进程清理中的配套动作

- `UserProcess::teardown`：先 `compat_itimer_real_disarm` 再 `detach_all_compat_shm`；
- `UserProcess::note_thread_exit`（最后线程离场）也会 disarm。

这避免了“旧定时器状态被子任务复用”的典型竞态。

## 8. 本章函数速查

- `timeval_to_duration`
- `duration_to_timeval`
- `duration_to_micros`
- `compat_itimer_real_remaining`
- `compat_itimer_real_disarm`
- `compat_itimer_real_arm`
- `compat_itimer_real_poll`
- `sys_setitimer`
- `ensure_user_return_hook_registered`
- `user_return_hook`
