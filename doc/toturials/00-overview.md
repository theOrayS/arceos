# 00 | 全局架构与系统调用执行路线

## 一句话总览

你可以把这次改造想成给 Shell 的“进程运行时”安装一个 **Linux 兼容翻译官**。

内核里本来已经能读写 `axfs`，但 Linux 用户程序发来的系统调用往往比 `axfs` 能力更细。  
这批改动的核心不是重写文件系统，而是把“Linux 用户接口语义”这层统一放在 `uspace.rs` + `linux_fs/*` 做适配。

## 主要角色（像图一样理解）

- `linux_fs`：负责“Linux 风格的小规则”，例如 `statx` flags、路径归一化、挂载参数校验、open 状态位定义。
- `uspace.rs`：负责“真的执行者”，包含：
  - 进程上下文（`UserProcess`）
  - 文件描述符表（`FdTable`/`FdEntry`）
  - 系统调用分发 (`user_syscall`)
  - 具体 syscall 处理函数（`sys_*`）
  - 与 `axfs` 的真正交互（打开文件、目录、映射、删除、创建）
- `axfs`：真实后端存储能力提供者（当前阶段只当作文件/目录引擎）

## 数据流（调用链）

1. 用户程序触发 trap 进入内核，参数落到寄存器
2. `user_syscall` 按号选择 `sys_*`
3. `sys_*` 先做参数/权限/路径解析
4. 需要文件语义时，走 `FdTable` 或 `open_fd_entry`
5. `FdTable` 操作底层 `OpenFileDescription`，后端由 `axfs::fops` 完成
6. 返回值统一转换成 `isize` + 错误码

## 为什么不是直接改 axfs

在本分支内，这一层有明确限制：

- `linux_fs` 明确声明不是 VFS
- 只包装 Linux ABI 兼容行为（包括一些 compat 行为）
- 不承诺完整 Linux 语义，只有“测试集合/兼容需求内必须过”的路径

类比一下：  
`axfs` 是“仓库里的仓位操作员”，`linux_fs + uspace` 是“前台导购”。  
导购负责把顾客的话术翻译成仓库能执行的格式，仓位不直接改成懂所有人类语言。

## 里程碑式变更速览（主线）

- 新增 `linux_fs` 目录：把文件状态位、路径、挂载、statx 投影抽离
- `uspace` 引入：
  - mount/umount 的 compat 路径
  - shm 三件套（`shmget`/`shmat`/`shmdt`/`shmctl`）
  - `ITIMER_REAL` 的 compat 定时器（`setitimer`）
  - `pread/pwrite`、`preadv/pwritev`、`readv/writev` 加强版实现
  - `fsync/fdatasync`、`statx`、空路径解析行为补齐
  - `FdTable` 的清理与状态位兼容增强

最新补充（当前工作区）：

- 管道环形缓冲区升级为“有读写计数 + 等待队列”
- `sys_execve` 读入 `envp`，并在构造用户栈时注入环境变量
- `clone` 增加 `exit_signal` 传播路径，`SIGCHLD` 与 `wait` 可配套触发
- `sys_rt_sigsuspend` 路径引入 `signal_wait`，并兼容 `sigsuspend` 语义
- LoongArch 走一套信号栈构造/恢复路径（与 riscv 类似）

## 你需要先确认的前提

以下几个点在后续章节会反复出现，先记住：

- `AT_FDCWD`（-100）是“以当前工作目录为基准”
- 这个 compat 层返回的错误码来自 `LinuxError`
- `compat_*` 前缀的函数都是“过渡方案”，并带删除条件注释
- `syscall` 里新增/修改的是“可观测行为”，不是全部语义覆盖
