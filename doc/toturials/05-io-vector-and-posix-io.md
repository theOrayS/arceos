# 05 | I/O 向量与 POSIX 显式偏移 I/O

这一章只讲本次提交新增/调整的 I/O 路径。  
关键词是：**文件偏移和用户向量边界**。

类比一句：  
普通 `read/write` 是“吃一根队列的当前指针”；而 `pread/pwrite` + `readv/writev` 是“你先给我拿刀具（偏移/向量）再开工”。

## 1. 设计目标：为什么加这些函数

以前的路径里，读写主要围绕：

- `sys_read / sys_write`：使用 `FdTable::read/write` 读写当前文件位移；
- `sys_lseek`：改变该位移。

但有些 workload 会同时用：

- `pread64/pwrite64`（带显式偏移，不动共享位移）；
- `readv/writev/preadv/pwritev`（一次系统调用处理多个缓冲区）。

如果不补齐，以下问题会出现：

- `pread/pwrite` 会误用共享偏移导致并发行为错误；
- `iovec` 长度超限时应当提前 `EINVAL`；
- 多段读写要有“短写/短读”语义：遇到半成功要返回已处理字节，不一定是全部。

## 2. 工具函数（I/O 基础组件）

### 2.1 `explicit_file_offset(offset)`

`pread/pwrite` 的入口校验：

- `offset` 不能超过 `i64::MAX`；
- 超过直接返回 `EINVAL`；
- 返回 `u64` 继续向下游处理。

**类比：** 相当于进门先查身份证，偏移不能装进“有符号时间戳”范围就拒绝进场。

### 2.2 `checked_io_total(total, delta)`

用于 `readv/writev/preadv/pwritev` 累加总量时的保护。

- `total + delta` 不能溢出；
- 也不能超 `isize::MAX`（上层返回值类型约束）。

失败 => `EINVAL`，避免“数值回卷”导致返回负值或截断。

### 2.3 `read_iovec_entries(process, iov, iovcnt)`

这是最核心的 `iovec` 解析器：

- `iovcnt` 必须 ≤ `IOV_MAX`，否则 `EINVAL`；
- 检查 `iov*iovec_size` 的总长度是否可计算；
- 通过 `user_bytes` 拉整段 `iovec` 内存；
- `ptr::read_unaligned` 一项项转为 `general::iovec`；
- 返回 Rust `Vec<general::iovec>`。

如果用户内存不可达直接 `EFAULT`。

### 2.4 `sys_fsync / sys_fdatasync`

- `sys_fsync(process, fd)` 映射到 `FdTable::sync(fd)`；
- `sys_fdatasync(process, fd)` 复用 `sys_fsync`。
- `FdTable::sync` 只处理以下情况：
  - `File`：调用 `OpenFileDescription::sync_file()`
  - `DevNull`：直接返回 `Ok(())`
  - `Directory` 或 `Pipe` 等：返回 `EINVAL`
- `OpenFileDescription::sync_file()` 中会调用 `compat_sync_unsupported_flush`，把部分后端不支持错误
  （`EINVAL` / `EOPNOTSUPP`）当作成功处理。

## 3. `pread64 / pwrite64` 的实现路径

### 3.1 `sys_pread64(process, fd, buf, count, offset)`

工作流程：

1. `explicit_file_offset(offset)` 校验；
2. 将用户 buffer 映射为可写切片；
3. 取 `FdTable` 中的 `FdEntry`，要求是 `File`；
4. 克隆一份 `Arc<OpenFileDescription>`（避免长临界区）；
5. 从传入偏移开始循环调用 `desc.pread_file`；
6. 每次成功后 `advance_explicit_offset(current_offset, n)`，防止越界；
7. 遇到 `n==0` 提前结束。

返回总读字节；不是共享 fd 的当前偏移。

### 3.2 `sys_pwrite64(process, fd, buf, count, offset)`

流程和 `pread64` 对称：

1. 校验偏移；
2. 拿可读用户 buffer；
3. 确保 fd 是文件；
4. 按当前偏移 `pwrite_file`；
5. 累加偏移；
6. 遇到零返回立即结束。

这两个函数都“按用户给定偏移工作”，不会改变 `open` 时共享在表里的同一描述符偏移。

> 这和“共享偏移文件描述符”语义是相反方向，必须单独理解。

## 4. 向量 I/O：`writev/readv`

### 4.1 `sys_writev(process, fd, iov, iovcnt)`

- 先 `entry(fd)` 确保 fd 合法；
- `read_iovec_entries` 解析向量；
- 依次取每个 `iov` 段：
  - `iov_len=0` 跳过；
  - 读用户内存（`user_bytes`）；
  - 调 `process.fds.lock().write` 写入；
  - `checked_io_total` 累加；
  - 当前段写完不够时 `break`（短写语义）。

返回写入总字节。

### 4.2 `sys_readv(process, fd, iov, iovcnt)`

- 先 `entry(fd)` 校验；
- 解析 `iovec`；
- 每段分配用户可写内存；
- `process.fds.lock().read`；
- 累加 + 短读 break。

返回总读取字节。  
与 `writev` 相同，失败时立刻返回对应 `errno`。

### 4.3 与非向量 `read/write` 的区别

- `read/write` 每次只处理一个 buffer，偏移由 FD 内部共享；
- `readv/writev` 按向量拆片处理，但仍受 `checked_io_total` 和短读/短写规则控制；
- 兼容性上两者都不保证完整处理所有段。

## 5. 向量的“显式偏移”版本：`preadv / pwritev`

### 5.1 `sys_preadv(process, fd, iov, iovcnt, offset)`

实现和 `sys_pread64` 的结合体：

- 先确认偏移；
- 只允许 `FdEntry::File(desc)`；
- 克隆 `Arc<OpenFileDescription>`；
- 解析 `iovec`；
- 每段按当前偏移 `desc.pread_file`；
- 每段成功后累加偏移。

### 5.2 `sys_pwritev(process, fd, iov, iovcnt, offset)`

流程同上，调用 `desc.pwrite_file`。

两者都避免锁住 `FdTable` 太久（仅在解析 fd 前短时间加锁），后续对每段 `desc` 操作在对象级锁和底层文件锁内部完成。

## 6. `FdTable::read_file_at` 的辅助作用

在 `uspace.rs` `FdTable` 新版里新增了：

- `read_file_at(fd, offset, len) -> Vec<u8>`

这个是从 `OpenFileDescription::read_file_at` 兼容抽象出来的旧接口延续，给某些调用（如 mmap 相关路径）提供稳定返回方式。它明确要求 fd 是文件，否则 `EBADF`。

## 7. 结合 `linux_fs::OpenFileDescription` 的行为一致性

`read_file / write_file / pread_file / pwrite_file` 的偏移行为一致性是这章的关键：

- `read_file` / `write_file`：依赖 `OpenFileDescription.offset`；
- `pread_file` / `pwrite_file`：完全按显式偏移，不改对象共享偏移；
- `read_file_at`：按固定长度一次性读到 Vec，直到 EOF。

这也是为什么 `sys_pread*/pwrite*/preadv/pwritev` 会先拿 `Arc<OpenFileDescription>`：  
共享对象才有共享位移语义，而显式偏移 API 要绕开共享位移。

## 8. 错误行为速记（重点）

- `explicit_file_offset` 负向/越界偏移 -> `EINVAL`；
- `iovcnt > IOV_MAX` 或字节数溢出 -> `EINVAL`；
- 用户指针非法 -> `EFAULT`；
- fd 类型错误（不是文件）-> `EBADF`；
- 到达末尾读到 0 后通常提前结束，返回已读/已写字节数。

## 9. 这章函数一览

- `explicit_file_offset`
- `checked_io_total`
- `read_iovec_entries`
- `sys_pread64`
- `sys_pwrite64`
- `sys_writev`
- `sys_readv`
- `sys_preadv`
- `sys_pwritev`
- `FdTable::read_file_at`
- `sys_fsync`
- `sys_fdatasync`
- `FdTable::sync`
