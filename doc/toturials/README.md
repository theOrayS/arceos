# Shell 兼容层学习手册总目录

本目录的文档按“课程式”拆分，目标是把 `main..HEAD` 这段提交里
`examples/shell/src/uspace.rs` 与 `examples/shell/src/linux_fs/*` 的改造，从全局架构讲到每个函数功能，全部说清楚。

> 写新文档前请先阅读：  
> [toturial 编写规范](./GUIDELINES.md)

## 你手上正在看的范围

- 起点提交：`6b2ccb4`（main）
- 终点提交：`776feee`（当前 HEAD）
- 涉及主文件：
  - `examples/shell/src/uspace.rs`
  - `examples/shell/src/linux_fs/fd.rs`
  - `examples/shell/src/linux_fs/path.rs`
  - `examples/shell/src/linux_fs/mount.rs`
  - `examples/shell/src/linux_fs/stat.rs`
  - `examples/shell/src/linux_fs/mod.rs`

说明：这些改动是在 `feat/filesystem` 分支里做的兼容性收口，目标仍是“门卫层”，不是完整 VFS 重写。

## 章节安排（按学习顺序）

1. [00-全局架构与调用链](./00-overview.md)
2. [01-linux_fs-兼容语义层](./01-linux-fs-wrapper.md)
3. [02-进程模型与系统调用主入口](./02-user-process-and-syscall.md)
4. [03-FD 表与文件描述对象](./03-fd-table-and-open-file-description.md)
5. [04-路径、stat、mount 兼容层](./04-path-stat-mount.md)
6. [05-I/O 向量与pread/pwrite 系列](./05-io-vector-and-posix-io.md)
7. [06-共享内存 compat 实现](./06-compat-shm.md)
8. [07-ITIMER/信号兼容路径](./07-compat-itimer.md)
9. [08-信号与等待同步（rt_sigsuspend / SIGCHLD）](./08-signal-and-wait.md)
10. [09-execve 与环境变量](./09-execve-and-envp.md)

## 统一的读法建议

我把“新函数”分成三层解释：

- 为什么加这个函数（问题）
- 函数做了什么（实现）
- 遇到的 Linux 与 Ax 的边界差异如何处理（兼容策略）

你可以先看第一、二章建立全局认知，再去看第五章和第六/七章做细节收敛。
