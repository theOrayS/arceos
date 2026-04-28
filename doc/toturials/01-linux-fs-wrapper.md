# 01 | linux_fs 兼容语义层（fd/path/mount/stat）

本章只讲新文件：

- `examples/shell/src/linux_fs/fd.rs`
- `examples/shell/src/linux_fs/path.rs`
- `examples/shell/src/linux_fs/mount.rs`
- `examples/shell/src/linux_fs/stat.rs`
- `examples/shell/src/linux_fs/mod.rs`

这个层次的目标是：把“Linux syscall 需要的语义小模型”先标准化，再由 `uspace.rs` 去消费。

## 1. `fd.rs`：文件描述相关数据结构

### 1.1 `FdFlags`

- 成员：`CLOEXEC`
- 职责：
  - 表示 file descriptor 的“执行时关闭”位（类似 `O_CLOEXEC`）
- 关键方法：
  - `empty()`：生成 `fd=0` 的起始值
  - `from_raw(raw)`：只保留兼容位
  - `raw()`：写回内核内部状态时用的原始数值
  - `cloexec()`：是否设置了 CL OEXEC
  - `set_cloexec(enabled)`：位开关
- 设计理由：
  - 把 `O_CLOEXEC` 和后续文件打开流程分开存储，避免和 `OpenStatusFlags` 混淆

### 1.2 `OpenStatusFlags`

- 成员：`APPEND`、`NONBLOCK`
- 职责：
  - 对应 open file description 里的“文件操作状态位”
- 关键方法：
  - `from_raw(raw)`、`raw()`、`set_raw(raw)`、`append()`
- 说明：
  - 只保留 `O_APPEND`、`O_NONBLOCK` 这类可回写给 `fcntl(F_SETFL)` 的位

### 1.3 `OpenFileBackend`

- 枚举：
  - `File(FileBackend)`
  - `Directory(DirectoryBackend)`
- 类比：一张“车厢票”：同样是票号（FD），但有可能是“文件乘客”或“目录乘客”。

### 1.4 `OpenFileDescription` 核心方法

#### `new_file(file, path, status_flags)`
- 建立普通文件描述
- 记录当前偏移 `offset=0`

#### `new_directory(dir, attr, path)`
- 建立目录描述
- 后续 `read/stat/getdents` 不走 file 后端，而走目录后端

#### `path() -> &str`
- 取当前打开对象路径，便于日志、`stat` 回传 ino 映射

#### `attr() -> Result<FileAttr, LinuxError>`
- 文件直接查 `File`；目录返回预存 `attr`
- 避免目录每次重复 `stat` 系统调用开销

#### `read_file(dst)`
- 类比：像按“当前游标”从文件里读，读完后游标自动前进
- 文件外访问返回 `EISDIR`
- 依赖 `self.offset` 与文件锁保护（`Mutex`）

#### `write_file(src)`
- 类比：在“当前游标”写，如果有 `APPEND` 就先 `seek_end`
- 每次成功后更新内核侧偏移
- 失败返回 `EBADF`/底层映射错误

#### `seek_file(offset, whence)`
- 支持 `SEEK_SET/CUR/END`
- 只对文件生效，目录返回 `ESPIPE`

#### `pread_file(dst, offset)`、`pwrite_file(src, offset)`
- 这两个是“显式偏移读写”，不改共享游标
- 常用于 `pread64/pwrite64/preadv/pwritev`

#### `advance_explicit_offset(offset, completed)`（新增）

- 作用：给 `pread/pwrite` 的显式偏移做“加法护栏”；
- 输入：当前偏移 + 本次已处理字节；
- 行为：用 `checked_add` 做溢出保护，超界返回 `EINVAL`；
- 类比：像拿着一把尺子往前走，走出边界就提醒你“不能再往前”。

#### `read_file_at(offset, len)`
- 先分配 buffer 后逐块 `read_at`，返回实际读到的切片
- 供 `mmap` 填充场景使用（`sys_mmap` 非匿名路径）

#### `truncate_file(size)`
- 目录/非法对象返回 `EINVAL`

#### `sync_file()`
- 当前实现里：调用文件 `flush()`，再包一层兼容逻辑

#### `compat_sync_unsupported_flush(result)`（补充）

- `compat_` 前缀函数，处理 `fsync` 不统一返回的问题；
- 规则：把后端返回的 `EINVAL`、`EOPNOTSUPP` 视为“可接受成功”（兼容层内）；
- 这样避免 `fsync/fdatasync` 在某些 `axfs` backend 上因为不支持而直接失败。

#### `compat_sync_unsupported_flush(result)`
- 名称上是 `compat_`，作用是吞掉某些后端不支持 `fsync` 的错误码
- 为什么要吞：用于测试路径中出现 `EINVAL`/`EOPNOTSUPP` 时不把兼容行为报错
- 注释里有删除条件：当 `axfs` 提供真实 fsync 能力时可移除

#### `advance_explicit_offset` 的防回归测试

- 在 `fd.rs` 的 `#[cfg(test)]` 里覆盖了两条核心边界：
  - 正常累计：`4096 + 128 = 4224`
  - 溢出边界：`u64::MAX - 3 + 4 -> EINVAL`
- 这类测试目的是固定 `pread/pwrite` 偏移前进的数学行为，避免后续改接口时静默改变语义。

## 2. `path.rs`：路径归一化与解析

### `resolve_path` 系列

- `normalize_path(base, path) -> Option<String>`
  - 做 `.`、`..`、多斜杠处理
  - `base` 为 `/` 时直接拼接，否则 `base/path`
  - 类比：把一条“有坑”的路径走一遍 GPS，消掉 `.` 和回退 `..`
- `resolve_cwd_path(cwd, path)`
  - 当前工作目录下的相对路径归一化入口
- `resolve_at_path(cwd, dirfd_base, path, options)`
  - 核心入口：支持 `AT_FDCWD`、dirfd 基址、空路径策略
  - `ResolveOptions::default()`：空路径报 `ENOENT`
  - `ResolveOptions::allow_empty()`：空路径允许并返回空字符串
  - 同时保留 `had_trailing_slash`，用于后续兼容判断

## 3. `mount.rs`：简化 mount 表实现

### `MountTable`

- 成员：`targets: Vec<String>`，是“已挂载挂载点集合”的最小兼容状态
- `new()`：初始化空表
- `clear()`：进程清理时一键清空

#### `mount(request)`
- 校验请求：`validate_mount_request`
- 去重目标检查：重复目标报 `EBUSY`
- 成功就把目标路径加入 `targets`

#### `umount(request)`
- 只接受 `flags == 0`
- 目标空字符串报 `EINVAL`
- 找不到 target 报 `EINVAL`，找到则 `swap_remove`

### `validate_mount_request / compat_basic_mount`

- 限制：
  - `flags != 0` -> `EINVAL`
  - `data != 0` -> `EOPNOTSUPP`
  - source/target/fstype 为空 -> `EINVAL`
- `compat_basic_mount` 只允许：
  - `fstype == "vfat"`
  - `source` 以 `/dev/` 开头
- 不满足返回 `EOPNOTSUPP`/`ENOENT`

## 4. `stat.rs`：statx 投影层

- 常量 `STATX_*` 与 `STATX_SUPPORTED_MASK`
- `validate_statx_flags(flags)`
  - 只允许部分位：`AT_SYMLINK_NOFOLLOW/NO_AUTOMOUNT/EMPTY_PATH/SYNC类型掩码`
  - 不支持位直接 `EINVAL`
- `statx_accepts_empty_path(flags)`
  - 是否带了 `AT_EMPTY_PATH`
- `stat_to_statx(st, mask)`
  - Linux 层面从 `stat` 映射到 `statx`
  - `mask=0` 表示默认返回支持的全部字段

## 5. `mod.rs`：导出地图

- `pub mod ...` 按职责分文件
- `pub use ...` 只导出外部要用的结构和函数
- 这样 `uspace.rs` 不用认识具体实现细节，只关心接口名
