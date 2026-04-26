# ArceOS BusyBox 文件管理长期可维护设计

日期：2026-04-26

## 背景

当前目标是完善 ArceOS shell syscall 路径中的文件管理能力，使 BusyBox
文件命令能够稳定运行，同时避免为了单个测试命令引入不可维护的特判。

BusyBox 脚本入口为 `testsuits-for-oskernel/scripts/busybox/busybox_testcode.sh`，
实际命令列表来自 `busybox_cmd.txt`。文件操作段覆盖 `touch`、`cat`、`cut`、
`od`、`head`、`tail`、`hexdump`、`md5sum`、`sort`、`uniq`、`stat`、
`strings`、`wc`、`more`、`rm`、`mkdir`、`mv`、`rmdir`、`grep`、`cp`、
`find`。

根据 `arceos/docs/development/interfaces/filesystem.md`，当前 shell syscall
路径应优先在 `examples/shell/src/linux_fs/` 承载 Linux ABI 与语义兼容逻辑。
该目录不是 VFS，也不应复制 `axfs` 后端能力。真实文件操作仍通过现有
`axfs::api` 与 `axfs::fops` 调用点完成。

## 目标

第一目标是建立长期可维护的文件 syscall 语义边界，而不是按 BusyBox 命令名补
测试特例。

本阶段目标：

1. 使 BusyBox 文件操作依赖的 Linux 可见行为有统一实现位置。
2. 降低 `examples/shell/src/uspace.rs` 中路径、fd、offset、flag 与 errno
   语义的耦合。
3. 保持 `basic` 文件/fd 子集在 RISC-V64 与 LoongArch64 上可回归验证。
4. 为后续 iozone、UnixBench fstime、lmbench 文件测试和 LTP fs 类测试保留
   可扩展模型。

非目标：

1. 本阶段不重构 `modules/axfs/**`。
2. 本阶段不实现完整权限模型、符号链接模型、真实 runtime mount/devfs。
3. 不通过 workload 名称、命令字符串、固定路径或 broad hardcode 返回成功。

## 推荐方案

采用 `linux_fs` ABI 语义层优先方案。

`uspace.rs` 保留 syscall 入口职责：

1. 读取和写回用户内存。
2. 做架构 syscall 分发。
3. 将参数传给 `linux_fs` 或现有 fd table 操作。
4. 将 `LinuxError` 转成负 errno。

`linux_fs` 逐步承接 Linux 文件语义：

1. `path.rs`：统一路径解析，处理 `AT_FDCWD`、绝对路径、相对路径、目录 fd、
   空路径、`.`、`..`、尾随 slash、基础 `AT_*` flag 校验。
2. `fd.rs`：迁移 fd slot 与 open-file-description 语义，区分 fd flags 和
   file status flags。
3. `stat.rs`：继续承载 `stat/statx` 投影，只上报真实支持字段。
4. `mount.rs`：保持 narrow `compat_*` mount 状态，直到真实 runtime mount
   接口存在。

## 模块边界

### `linux_fs/path.rs`

负责纯 Linux 路径语义和 resolver 输入输出。

需要支持：

1. null pathname 指针在 syscall 层返回 `EFAULT`。
2. 空路径默认返回 `ENOENT`，仅 `AT_EMPTY_PATH` 支持的查询类 syscall 例外。
3. 绝对路径从进程 namespace root 开始。
4. 相对路径在 `AT_FDCWD` 时从进程 cwd 开始，否则从目录 fd 对应路径开始。
5. bad relative `dirfd` 返回 `EBADF`，非目录 fd 返回 `ENOTDIR`。
6. `.` 被忽略，`..` 不能逃出 namespace root。
7. 尾随 slash 要求目标为目录，否则返回 `ENOTDIR`。
8. 未知 `AT_*` flags 返回 `EINVAL`。

### `linux_fs/fd.rs`

负责 fd table 与 open-file-description 模型的长期归宿。

目标模型：

```rust
pub struct FdTable {
    entries: Vec<Option<FdSlot>>,
}

pub struct FdSlot {
    pub fd_flags: FdFlags,
    pub desc: Arc<OpenFileDescription>,
}

pub struct OpenFileDescription {
    pub status_flags: Mutex<OpenStatusFlags>,
    pub offset: Mutex<u64>,
    pub backend: OpenFileBackend,
}
```

必须分离：

1. `FD_CLOEXEC` 属于 fd slot。
2. `O_APPEND`、`O_NONBLOCK` 和共享文件 offset 属于 open file description。
3. `dup`、`dup2`、`dup3` 指向同一个 open file description。
4. `read`、`write`、`lseek`、`getdents64` 使用并更新共享 offset。
5. `pread/pwrite` 类 syscall 使用显式 offset，不更新共享 offset。

### `linux_fs/stat.rs`

继续负责 Linux metadata 投影。

要求：

1. `statx` 返回 `requested_mask & supported_mask`。
2. 不伪造尚无后端支持的字段。
3. invalid statx flags 返回 `EINVAL`。
4. 用户输出缓冲由 syscall 层最后写回。

### `linux_fs/mount.rs`

保持现有 narrow compatibility 路径。

要求：

1. 兼容代码继续使用 `compat_*` 命名。
2. 成功 mount 必须记录足够状态，使 `umount2` 的逆操作有意义。
3. 未支持 flags、data、filesystem 或 source 明确返回 errno。
4. 不为了 BusyBox 文件命令扩展 mount fake state。

## 实施顺序

### 阶段 1：路径 resolver 收敛

把 `openat`、`mkdirat`、`unlinkat`、`renameat2`、`faccessat`、`newfstatat`、
`statx`、`utimensat` 的路径解析统一到一个 resolver。

该阶段优先减少行为分叉，不改变 `axfs` 后端。

### 阶段 2：OFD 与 fd offset 收敛

将普通文件、目录和 pipe 的 fd slot 与共享 open-file-description 语义迁移到
`linux_fs/fd.rs`。

优先行为：

1. `read/write/lseek` 对普通文件共享 offset。
2. `getdents64` 对目录 fd 使用目录 offset。
3. `dup/dup3` 与 fork 后 fd 语义不复制独立 offset。
4. `O_APPEND` 在 write 路径生效。

### 阶段 3：BusyBox 文件命令缺口

在 resolver 与 OFD 基础上补齐 BusyBox 文件命令可能触发的缺失 syscall 或
返回策略：

1. `truncate`、`ftruncate`。
2. `statfs`、`fstatfs`。
3. `readlinkat`，在符号链接未支持时返回一致 errno。
4. `sync`，没有全局 flush 能力时返回明确 unsupported errno 或实现可证明的
   no-op 策略。
5. `sendfile`，若 BusyBox `cp` 或相关路径触发，优先基于 fd read/write 实现，
   不做命令名特判。

### 阶段 4：回归与推广

先保持 `basic` 文件/fd 子集稳定，再验证 BusyBox 文件操作段。后续将同一模型
推广到 iozone、UnixBench fstime、lmbench 文件测试和 LTP fs 类测试。

## Errno 与兼容原则

1. 不支持的 syscall dispatcher arm 返回 `ENOSYS`。
2. 已知 syscall 的未知 flag 或非法 flag 组合返回 `EINVAL`。
3. 后端能力不存在时返回 `EOPNOTSUPP`、`ENODEV` 或更具体 errno。
4. fd 类 syscall 先校验 fd，再校验用户 buffer。
5. 路径输入先复制用户字符串，再进行可见状态改变。
6. 不为 workload、命令名或固定测试路径添加成功特判。

## 文件影响范围

预计主要修改：

1. `examples/shell/src/linux_fs/path.rs`
2. `examples/shell/src/linux_fs/fd.rs`
3. `examples/shell/src/linux_fs/stat.rs`
4. `examples/shell/src/linux_fs/mod.rs`
5. `examples/shell/src/uspace.rs`
6. `docs/development/interfaces/filesystem.md`
7. `docs/development/interfaces/syscall-inventory.md`
8. `docs/development/policies/compatibility.md`
9. `doc/logs/*`

除非后续任务明确要求真实底层接口，本设计不修改 `modules/axfs/**`。

## 验证策略

最小验证顺序：

1. RISC-V64 basic 文件/fd 子集。
2. LoongArch64 basic 文件/fd 子集。
3. RISC-V64 BusyBox 文件操作段。
4. LoongArch64 BusyBox 文件操作段。

推荐命令：

```sh
QEMU_TIMEOUT=240s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info
QEMU_TIMEOUT=240s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info
```

如 wrapper 继续进入非目标 suite，可在 basic 或 BusyBox 相关段完成后只停止容器
内 QEMU 进程，保留 `arceos-eval-fix` 容器。

## 风险

1. `FdTable` 当前在 `uspace.rs` 中，迁移到 `linux_fs/fd.rs` 时容易产生大 diff。
   计划应拆成小步，先建立类型和适配接口，再迁移行为。
2. `fork/clone/execve` 与 fd slot/OFD 语义耦合，不能只改 read/write。
3. `getdents64` 的目录 offset 如果处理不完整，会直接影响 BusyBox `find`。
4. `statfs/fstatfs` 需要决定是提供真实近似信息还是明确 unsupported，不能返回
   无依据的成功。
5. 权限、symlink、mount/devfs 仍是后续阶段风险。

## Spec 自检

1. 无 `TBD` 或未定实现项。
2. 设计边界与 `docs/development/interfaces/filesystem.md` 保持一致。
3. 不依赖 BusyBox 命令名或固定测试路径。
4. 先处理 syscall ABI 语义层，再考虑 `axfs` 底层能力扩展。
5. 验证策略包含 RISC-V64 与 LoongArch64。
