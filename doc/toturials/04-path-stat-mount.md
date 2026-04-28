# 04 | 路径解析与 stat/mount 兼容链路

本章处理三个经常一起打结的点：`path`、`stat`、`mount`。它们在这批改造里都属于“兼容翻译层 + 用户态入口”协作问题，而不是真正文件系统核心功能。

先用一句话看全景：  
**`linux_fs::path` 负责把人类看见的路径句法整理干净；`uspace.rs` 的 `resolve_dirfd_path / open_fd_entry` 负责把“哪个 fd 是基准目录”翻译成最终绝对路径；`mount/stat` 决定“这类路径语义允许到什么程度”。**

## 1. 关键目标（为什么要这么做）

BusyBox / iozone 一类测试会发出大量 `openat/faccessat/mount/statx` 变体，其中大量测试只关心“行为看起来对”，不关心内核有没有完整 VFS。  
改造目标不是重建 VFS，而是把系统调用层的语义差异补齐：

- `AT_FDCWD`、`dirfd`、空字符串路径（`""`）在 Linux 里有明确规则；
- `mount/umount2` 在这里不做真正挂载，只记录兼容状态；
- `statx` 的 flag/mask 要能接受并返回合理字段，不该返回 `ENOTSUP` 的就不要提前炸掉；
- 对目录重命名兼容行为不足时，用最小替代路径保证测试可过。

## 2. 路径解析：`linux_fs::path` 的职责

### 2.1 `resolve_path` / `normalize_path(base, path)`

**类比：** 把一条歪歪扭扭的“行车导航输入”先变成标准路口坐标。

- 输入：
  - `base="/tmp"`，`path="a/../b"`。
- 处理：
  - 切分 `/` 段；
  - `.` 忽略；
  - `..` 回退一层；
  - 拼出 `/tmp/b`。
- 输出：
  - 始终返回 `/xxx` 开头的标准绝对路径字符串。
- 异常：
  - 语法异常时返回 `None`（上层会转成 `EINVAL`）。

> 这个函数在仓库里是 `crate::linux_fs::normalize_path`，被 `uspace.rs` 多处调用用于“把宿主路径候选”统一成绝对路径。

### 2.2 `resolve_cwd_path(cwd, path)`

`normalize_path(cwd, path)` 的语义糖封装。  
主要服务入口：`sys_getcwd` 反馈和若干 open/rename 解析分支。

### 2.3 `resolve_at_path(cwd, dirfd_base, path, options)`

这是 “at 系列”系统调用最关键的决策函数。

- 输入：
  - `cwd`: 进程工作目录；
  - `dirfd_base`: `AT_FDCWD` 解析失败时可替代的 `dirfd` 目录基址；
  - `path`: 原始路径；
  - `options`: 是否允许空路径（`allow_empty`）。
- 决策规则：
  - `path=""`：
    - 默认不允许，返回 `ENOENT`；
    - 如果 `allow_empty`，返回空字符串，且 `had_trailing_slash=false`。
  - 绝对路径：忽略 `dirfd_base`，直接按 `/` 归一化。
  - 相对路径：优先 `dirfd_base`，否则回退 `cwd`。
  - 保留 `had_trailing_slash`（是否原路径有尾随 `/`），给上层作兼容判断。

### 2.4 `resolve_dirfd_path` / `resolve_dirfd_path_allow_empty`（`uspace.rs`）

这两个函数把 `linux_fs::resolve_at_path` 放在进程上下文里调用。

- `resolve_dirfd_path` = 默认不允许空路径；
- `resolve_dirfd_path_allow_empty` = 允许空路径（带 `[allow_empty]` 标记）。

共同流程：

1. 读取 `process.cwd()`；
2. 用 `FdTable::dirfd_base_path(dirfd)` 判断是否用 cwd 或 `dirfd` 的目录路径；
3. 调 `crate::linux_fs::resolve_at_path`；
4. 返回标准化后的绝对路径。

如果 `dirfd` 不是目录 fd，返回 `ENOTDIR`。

## 3. `open`/`faccessat`/`stat` 的路径落地逻辑

这里是用户态兼容层最容易踩坑的地方，主要函数如下：

### 3.1 `open_fd_entry(process, table, dirfd, path, flags)`

目标：从不同路径候选中拿到一个 `FdEntry`，要兼容 `O_DIRECTORY`、运行时库路径搜索、`/dev/shm` 映射。

关键行为：

- 根据 `flags` 构建 `OpenOptions`：
  - `O_RDONLY/O_WRONLY/O_RDWR` => 读写模式；
  - `O_APPEND / O_TRUNC / O_CREAT / O_EXCL` 等透传；
- `prefer_dir = (O_DIRECTORY != 0)`。
- `absolute` 分支：
  - `"/dev/shm/*"` 会转 `"/tmp/shm/*"`；
  - 生成 `runtime_absolute_path_candidates` + 运行时候选路径（用于测试阶段兼容）；
  - 逐个候选做打开。
- 非绝对且非 `AT_FDCWD`：
  - 必须先从 `dirfd` 得到目录对象；
  - 路径拼接到 `dir.path()` 后再候选。
- 最终交给 `open_fd_candidates`。

> 这里的“候选路径”设计是为了兼容脚本加载、so 查找、busybox 场景里出现的“同名不同位置”情况。

### 3.2 `open_fd_candidates(candidates, prefer_dir, flags, opts)`

候选路径迭代器。逻辑上是：

1. 若路径是 `/dev/null` 且未要求目录，直接返回 `FdEntry::DevNull`；
2. 如果 `prefer_dir` 或“只读未带创建/截断”，先 `metadata` 判断是否目录：
   - 目录且命中 => `open_dir_entry`；
   - `prefer_dir` 但目标是普通文件 => `ENOTDIR`。
3. 否则尝试 `File::open`：
   - 成功 => 返回 `FdEntry::File(Arc<OpenFileDescription>)`；
   - `EISDIR` => 回退 `open_dir_entry`；
   - `ENOENT` 继续下一个候选；
   - 其他错误立即返回。

### 3.3 `open_dir_entry(path)`

按目录路径返回 `FdEntry::Directory`，并携带：

- 目录 handle（`Directory`）；
- 文件属性（`FileAttr`）；
- `path`（供后续 `fstat` 使用）。

该对象包装进 `OpenFileDescription`（见 `linux_fs::fd`）后放入 `FdEntry::Directory`。

### 3.4 `dirfd_base_path(dirfd)`（`FdTable`）

- `AT_FDCWD` -> `Ok(None)`；
- 普通 fd 取 `entry(fd)`，要求是 `Directory`；
- 不是目录 => `ENOTDIR`。

是 `resolve_dirfd_path` 的基石。

### 3.5 `dev_shm_host_path` 与候选构造细节（最新补充）

- `dev_shm_host_path(path)`：
  - 把用户态 `/dev/shm/xxx` 映射到主机路径 `/tmp/shm/xxx`；
  - 目的是让 busybox/脚本路径在当前环境里更容易落地。
- `ensure_host_dir(path)`/`ensure_dev_shm_dir()`：
  - 打开 `/dev/shm` 前，确保 `/tmp` 和 `/tmp/shm` 存在；
  - 不存在就按 axfs 侧的目录创建能力补齐。
- `runtime_library_name_candidates(...)`（隐式）：
  - 对非绝对路径，会额外尝试 `lib`/运行时根相关候选；
  - 这就是 `execve / open` 在不同运行时根下“同名不同位置”仍能被找到的原因。

## 4. `stat` 系列兼容路径

### 4.1 `stat_path_abs(path)`

作用：不借助 `sys_fstat` 的 fd 直接路径版本，服务 `faccessat / newfstatat / statx`。

执行顺序：

1. `path == "/dev/null"` 直接返回伪造 `stdio` stat；
2. 先尝试 `File::open(path, O_RDONLY)`；
3. 成功则 `file.get_attr()`；
4. 如果 `EISDIR`，再走 `open_dir_entry` 拿目录属性；
5. 其他错误直接透传。

### 4.2 `sys_statx(process, dirfd, pathname, flags, mask, statxbuf)`

这个函数现在完整支持 `statx` 的基本流程：

- 先 `validate_statx_flags(flags)`；
- `path=""` 时要求 `AT_EMPTY_PATH`，没有则报 `ENOENT`；
- 读取路径时走 `resolve_dirfd_path`；
- 空路径且 `AT_EMPTY_PATH` 设置时直接 `process.fds.stat(dirfd)`；
- 最终 `stat_to_statx(&st, mask)` 写回用户空间。

和旧行为对比：现在不再只局限于 `fstat`/`fstatat` 形式，能处理 `statx` 标志与掩码。

## 5. `mount / umount` 的最小兼容状态机

这里没有改动真实 VFS 挂载，而是记录“挂载状态”让测试看到可预期语义。

### 5.1 `sys_mount(process, source, target, fstype, flags, data)`

流程：

1. 从用户态读三个字符串（`source / target / fstype`）；
2. `target` 先 `normalize_user_path` 成标准绝对路径；
3. 校验 `target` 目录可用：`open_dir_entry(target)`；
4. 构造 `crate::linux_fs::MountRequest`；
5. 调 `process.mount_table.lock().mount(request)`。

### 5.2 `sys_umount2(process, target, flags)`

流程简单：

1. 读 `target`；
2. 标准化 `target`；
3. 构造 `UmountRequest`；
4. 调用 `MountTable::umount`。

### 5.3 `MountTable`

在 `examples/shell/src/linux_fs/mount.rs`：

- `mount(request)`：
  - 先 `validate_mount_request`；
  - 检查 `target` 不重复；
  - 成功则 `targets.push(target)`。
- `umount(request)`：
  - `flags == 0`；
  - `target` 非空；
  - 在 `targets` 里找到并 `swap_remove`；
  - 找不到 => `EINVAL`。

### 5.4 `validate_mount_request` 与 `compat_basic_mount`

- 只允许：
  - `flags == 0`；
  - `data == 0`；
  - `source / target / fstype` 非空；
  - `fstype == "vfat"`；
  - `source` 以 `/dev/` 开头。

不满足条件一律走 `EINVAL / EOPNOTSUPP / ENOENT`，避免把不支持行为误报为内核 bug。

## 6. 目录重命名的兼容补丁

### 6.1 `rename_path_abs(old_path, new_path)`

先尝试 `axfs::api::rename`。如果后端返回 `EOPNOTSUPP / ENOSYS`，转到 `compat_empty_dir_rename`。

### 6.2 `compat_empty_dir_rename(old_path, new_path)`

这是“目录名空目录重命名”的临时兼容：

- 确认 `old_path` 是目录；
- `new_path` 不存在（否则 `EEXIST`）；
- `create_dir(new_path)`；
- 删除 `old_path`；
- 任一失败回滚 `new_path` 的清理。

注释里明确写了删除条件：真实后端实现 rename 目录语义后可移除。

## 7. 与上一章（03）衔接点

`FdTable::open / sys_openat / sys_newfstatat / sys_fstat` 的行为变化不是独立的：

- `sys_openat` 只是入口，核心都是 `FdTable::open -> open_fd_entry`；
- `stat/fstat` 在 `FdEntry::Directory` 分支里走 `OpenFileDescription::attr()`；
- `resolve_dirfd_path` 让 `...at` 一族 syscall 对 `AT_FDCWD` 和 `dirfd` 规则一致。

## 8. 这章里新增/修改的核心函数一览（速查）

- `crate::linux_fs::resolve_at_path`
- `crate::linux_fs::resolve_cwd_path`
- `crate::linux_fs::normalize_path`
- `resolve_dirfd_path`
- `resolve_dirfd_path_allow_empty`
- `open_fd_entry`
- `open_fd_candidates`
- `open_dir_entry`
- `stat_path_abs`
- `rename_path_abs`
- `compat_empty_dir_rename`
- `sys_mount`
- `sys_umount2`
- `sys_statx`
- `sys_newfstatat`（路径路径分支改造）
- `sys_faccessat`（路径分支改造）
- `sys_renameat2`（目录重命名走 compat）

## 9. 对新同学的记忆锚点

如果你只记三句：

1. `linux_fs::resolve_at_path` 负责路径句法收敛；`resolve_dirfd_path` 负责上下文注入；
2. `open_fd_entry` 负责“候选路径 + 打开策略”，不是 `sys_openat` 本身；
3. mount 在这里只是“兼容账本”，真实挂载语义留给真正的文件系统层。
