# 03 | FD 表与打开对象：从 fd 数字到真实读写对象

本章覆盖 `uspace.rs` 里 `FdEntry / FdSlot / FdTable` 的函数级行为（含新旧行为差异）。

## 1. 两层对象：`FdEntry` 与 `OpenFileDescription`

类比：你在工地拿到的“钥匙编号”是 FD；真正开门的是“钥匙卡权限对象”。

- `FdEntry` 是入口类型：
  - `Stdin/Stdout/Stderr/DevNull`
  - `File(SharedOpenFileDescription)`
  - `Directory(SharedOpenFileDescription)`
  - `Pipe(PipeEndpoint)`
- `OpenFileDescription` 在 `linux_fs::fd` 中定义，负责读写游标、状态位、底层锁。

## 2. `FdSlot`

### `FdSlot::new(entry, fd_flags)`
- 创建一个槽位，绑定 `FdEntry` 与 `FdFlags`
- 这个组合就是 `open`/`dup` 最终落位的最小单元

### `duplicate_for_fork()`
- 复制 fd 本身语义，不共享 thread-specific 的状态
- `File/Directory` 用 `Arc::clone`，`Pipe` 用 `clone`，标准 fd 直接复用常量对象

## 3. `FdTable::new()`

- 初始化标准输入输出：
  - fd0: `Stdin`
  - fd1: `Stdout`
  - fd2: `Stderr`
- 这就是 shell 与大多数进程运行前的最小“默认 FD 布局”。

## 4. `FdTable::fork_copy()`

- 全量克隆当前 FD 向量
- 每一项调用 `FdSlot::duplicate_for_fork`
- 对共享内存/文件：`Arc::clone` 共享同一描述（和真实 Unix 一致）

## 5. `FdTable::dirfd_base_path(dirfd)`

- 用于 `openat/mkdirat/unlinkat/statat` 这类相对路径的 dirfd 解析
- `AT_FDCWD` 时返回 `None`
- 非目录 FD 返回 `ENOTDIR`

## 6. `FdTable::is_stdio(fd)`

- 仅检查 0/1/2 且是标准流对象，常用于 `ioctl(TIOCGWINSZ)` 场景

## 7. `FdTable::poll(fd, mode)`

- `Read/Write/Except` 三种 select 模式判定
- `Pipe` 用 ring buffer 的 `poll`
- 普通文件默认可读写；`Directory` 不可写
- 类比：像“信号灯”，每个对象告诉你这个 fd 现在能不能被 `select` 上

## 8. `FdTable::read(fd, dst)` / `write(fd, src)`

- `read`：
  - `Stdin` 返回 0（空实现）
  - `DevNull` 返回 0
  - `File` 调 `desc.read_file`
  - `Pipe` 调 pipe 读端
  - 目录/非法 -> `EISDIR/EBADF`
- `write`：
  - `Stdout/Stdout` -> 控制台输出
  - `DevNull` -> 丢弃
  - `File` -> `desc.write_file`
  - `Pipe` -> pipe 写端

### PipeEndpoint（新增）：从“空转等待”到“阻塞队列”

管道内部不再只靠“缓冲区是否有数据”做快速返回，而是加了配套机制：

- `PipeEndpoint::new_pair()`
  - 生成 `readable`/`writable` 两端，共享 `PipeRingBuffer`
  - 缓冲区固定 4096，适合更多小包场景
- `PipeRingBuffer::new(readers, writers)` 同步初始化端点计数（初始都为 1）
- `PipeRingBuffer` 增加读写计数：
  - `readers`：当前还在用读端的数量
  - `writers`：当前还在用写端的数量
- `read_wait` / `write_wait`
  - 读端没数据时挂起等待 `read_wait.notify_all`
  - 写端没空间时挂起等待 `write_wait.notify_all`
- `Clone`/`Drop`
  - `Clone` 增加对应端计数
  - `Drop` 自动减计数，并通知对端让其尽快从阻塞态醒来
- `peer_closed` 判断逻辑改为计数法：
  - 读端发现 `writers=0`：返回 EOF 或 `EPIPE`
  - 写端发现 `readers=0`：立即触发 `EPIPE`

### `PipeEndpoint::read` 细节

- 先算 `available_read`，没数据时等待 `read_wait`，直到：
  - 新数据到达，或
  - 对端关闭（`peer_closed_locked` 为 true）。
- 一次把可读字节最多一次性读完；
- 每成功读到数据后，唤醒写端 `write_wait`，提示有空间。
- 读到 0 字节时通常意味着 EOF（写端真的都走了）或对端关闭。

### `PipeEndpoint::write` 细节

- 先判定写端是否可写（`peer_closed_locked`）；
- 缓冲满时进入 `write_wait` 等待；
- 写入后唤醒读端 `read_wait`；
- 如果对端没读者（`readers=0`），立即返回 `EPIPE`，且可能带部分已写字节。

### `PipeEndpoint::poll`

- `readable`：有数据可读 **或** 对端已关闭；
- `writable`：有空间可写 **或** 对端已关闭；
- 实现目标是：`select/poll` 不会因为对端关闭而卡住不返回。

## 9. `FdTable::open(process, dirfd, path, flags)`

- 仅做“打开语义入口”
- 实际文件候选解析在 `open_fd_entry`（见下一章）
- 把 `O_CLOEXEC` 提取为 `FdFlags`，再 `insert_with_flags`

## 10. `close(fd)` 与 `close_all()`

- `close(fd)`：
  - 下标越界/空槽 -> `EBADF`
  - 成功置位 `None`
- `close_all()`（本次新增）：
  - 直接 `clear()` 整个 vector
  - 注意副作用：不仅关闭文件，还移除标准 fd 位（exec/exit 后会重建上下文时再分配）
- 与旧实现差异：`teardown` 不再依赖 `FdTable::new` 重建，而是直接清空

## 11. `close_cloexec()`

- 进程 `execve` 时关闭所有 `FD_CLOEXEC` 的 fd
- 遍历 `entries`，清理需要关闭的 slot

## 12. `stat(fd)`, `truncate(fd, size)`, `sync(fd)`

- `stat`：
 - `Stdin/Stdout/Stderr/DevNull` 返回伪造 stat
 - `File/Directory` 从 `OpenFileDescription::attr` 回
 - `Pipe` 返回 pipe stat
- `truncate`：
 - 文件可改大小；`DevNull` 兼容地返回 OK；其他报 `EINVAL`
- `sync`：
 - 文件调用 `desc.sync_file()`
 - `DevNull` OK；目录返回 `EINVAL`

## 13. `file_status_flags(fd)` / `set_file_status_flags(fd, flags)`

- 查询和设置 `O_APPEND/O_NONBLOCK`（来自 `OpenStatusFlags`）
- 文件/目录可设置内部状态；标准流和管道通常接受但不作实际语义变更

## 14. `fcntl(fd, cmd, arg)` 的关键分支

- `F_DUPFD`、`F_DUPFD_CLOEXEC`：
  - 返回一个新的 fd，`min_fd` 由 `arg` 提供
- `F_GETFD`/`F_SETFD`：对 `FdFlags` 操作（如 `FD_CLOEXEC`）
- `F_GETFL`/`F_SETFL`：对 `OpenStatusFlags` 的只读/只写位控制

## 15. `lseek(fd, offset, whence)`

- 文件：调用 `desc.seek_file`
- 目录 -> `EISDIR`
- 管道/标准流 -> `ESPIPE`

## 16. `dup / dup_min / dup_min_with_flags`

- `dup`：`min_fd=0`
- `dup_min`：重复利用 `insert_min_with_flags`
- `dup_min_with_flags`：
  - `min_fd < 0` -> `EINVAL`
  - 先复制 `FdEntry` 再插槽

## 17. `dup3(oldfd,newfd,flags)`

- `flags` 必须仅含 `O_CLOEXEC`，否则 `EINVAL`
- `oldfd == newfd` -> `EINVAL`
- `newfd < 0` -> `EBADF`
- 不要求 `newfd` 预先可用：会自动 `resize_with` 到位

## 18. `getdents64(fd, dst)`

- 只对目录开放，其他 fd -> `ENOTDIR`
- 用 `Directory::read_dir` 拿到多项后，按 `linux_dirent64` 布局逐项写入用户缓冲区

## 19. `read_file_at(fd, offset, len)`

- 仅文件有效
- 代理到 `OpenFileDescription::read_file_at`

## 20. `insert / insert_with_flags / insert_min_with_flags`

- `insert` 保留 `O_CLOEXEC` 为空
- `insert_min_with_flags` 核心策略：
  - 先从 `min_fd` 起找空槽
  - 找不到则 `push` 新槽
- 与标准 Unix 类似：找到空位优先，不做无意义移动

## 21. `entry / entry_mut / slot / slot_mut`

- `entry`：只取 `&FdEntry`
- `entry_mut`：取可改 `&mut FdEntry`
- `slot`：取整槽位（含 `fd_flags`）
- 都会做下标和空位检查，失败返回 `EBADF`

## 22. 目录/文件统一入口：`open_fd_entry` 与 `open_fd_candidates`

这两个函数在下一章集中讲，原因是它们和路径解析耦合大。
