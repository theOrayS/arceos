# BusyBox 文件管理长期可维护性设计与计划

## 2026-04-26 09:26 CST 设计与实现计划

- 范围和目标：为 ArceOS shell syscall 路径中的 BusyBox 文件管理命令制定长期可维护方案，优先收敛路径解析、fd/OFD offset、metadata/statx 投影和 syscall errno 语义。
- 修改文件：`docs/superpowers/specs/2026-04-26-arceos-busybox-filesystem-design.md`，`docs/superpowers/plans/2026-04-26-arceos-busybox-filesystem-implementation-plan.md`，`doc/logs/2026-04-26-busybox-filesystem.md`。
- 关键决策：采用 `examples/shell/src/linux_fs/` ABI 语义层优先方案；`uspace.rs` 保留 syscall 分发和用户内存复制；真实文件内容仍走现有 `axfs::api` 与 `axfs::fops`；本阶段不修改 `modules/axfs/**`；不按 BusyBox 命令名、workload 名称或固定路径添加特判。
- 验证结果：未执行构建、测试或 QEMU 验证；当前步骤只写设计和实现计划，等待用户授权后再执行源码修改与验证命令。
- 剩余风险：fd/OFD 迁移会影响 `dup`、`fork`、`clone(CLONE_FILES)`、`execve` 的 fd 语义；`getdents64` 目录 offset 影响 BusyBox `find`；`statfs/fstatfs`、symlink、权限模型、runtime mount/devfs 仍需后续真实能力支撑。

## 2026-04-26 09:40 CST 第一批路径 resolver 与 OFD 骨架改动

- 范围和目标：执行 BusyBox 文件管理实现计划的 Task 1 到 Task 4，新增 dirfd-aware 路径 resolver，初步让若干 path-taking syscall 复用统一解析路径，并在 `linux_fs/fd.rs` 建立 open-file-description 类型骨架。
- 修改文件：`examples/shell/src/linux_fs/path.rs`，`examples/shell/src/linux_fs/mod.rs`，`examples/shell/src/linux_fs/fd.rs`，`examples/shell/src/uspace.rs`，`doc/logs/2026-04-26-busybox-filesystem.md`。
- 关键决策：`openat` 暂时保留现有 runtime candidate 查找逻辑，避免动态库路径解析回归；`mkdirat`、`unlinkat`、`faccessat`、`newfstatat`、`statx` 先收敛到 `resolve_dirfd_path`；`stat_path_abs` 复用既有 `File::open(...).get_attr()` 与 `open_dir_entry()` 获取 `FileAttr`，不引入 `axfs::api::Metadata` 到 `FileAttr` 的伪转换。
- 验证结果：已执行 `docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info`，RISC-V64 kernel build 成功；当前存在 11 个 warning，主要来自 OFD 骨架尚未接入以及旧 `FdTable` path helper 暂未删除。
- 剩余风险：尚未迁移 `FdEntry::File/Directory` 到共享 OFD，因此 `dup/fork` 共享 offset、目录 offset 和 `O_APPEND` 语义仍未完成；尚未执行 LoongArch64 构建或 BusyBox/QEMU 运行验证。

## 2026-04-26 09:44 CST 共享 OFD 初步接入

- 范围和目标：将 `FdEntry::File` 与 `FdEntry::Directory` 从独立可克隆 entry 迁移为 `Arc<OpenFileDescription>`，让普通文件和目录 fd 在 `dup`、`fork_copy` 等复制路径中共享同一个 open-file-description。
- 修改文件：`examples/shell/src/linux_fs/fd.rs`，`examples/shell/src/uspace.rs`，`doc/logs/2026-04-26-busybox-filesystem.md`。
- 关键决策：在 `linux_fs::fd::OpenFileDescription` 内部保存 status flags、offset 和后端对象；`read`、`write`、`lseek`、`read_file_at`、`truncate`、`getdents64` 通过共享 OFD 后端访问真实 `File`/`Directory`；目录 fd 的 base path 改为从 `desc.path()` 读取。
- 验证结果：未执行构建或 QEMU 测试；等待用户授权后运行 `kernel-rv` 构建检查。
- 剩余风险：需要构建确认 fd/OFD 迁移后的类型和 trait 方法解析；`FD_CLOEXEC`、完整 `fcntl(F_GETFL/F_SETFL)`、`execve` close-on-exec 和 LoongArch64 验证仍未完成。

## 2026-04-26 09:47 CST OFD 迁移构建检查

- 范围和目标：验证共享 OFD 初步接入后的 RISC-V64 kernel build，并修复构建暴露的旧 `FileEntry` 访问残留。
- 修改文件：`examples/shell/src/uspace.rs`，`doc/logs/2026-04-26-busybox-filesystem.md`。
- 关键决策：`sys_pread64` 改为通过 `OpenFileDescription::pread_file` 执行 explicit-offset 读取，保持不更新共享 OFD offset 的语义。
- 验证结果：第一次执行 `docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info` 失败，错误为 `sys_pread64` 仍访问旧 `file.file` 字段；修复后再次执行同一命令成功。当前剩余 6 个 warning，主要是 `FdFlags`/`ResolvedPath` 导出和若干后续阶段才会使用的 fd flag helper 未接入。
- 剩余风险：尚未执行 LoongArch64 构建；尚未运行 basic/BusyBox QEMU 测试；fd flags、close-on-exec、完整 `fcntl` 状态仍未实现。

## 2026-04-26 09:48 CST LoongArch64 构建检查

- 范围和目标：验证共享 OFD 与路径 resolver 改动在 LoongArch64 kernel build 下无架构相关编译问题。
- 修改文件：`doc/logs/2026-04-26-busybox-filesystem.md`。
- 关键决策：本次仅记录验证结果，不修改源码；RV 与 LA 使用同一套 shell syscall/OFD 代码路径。
- 验证结果：已执行 `docker exec arceos-eval-fix make -C /workspace/arceos kernel-la KERNEL_LOG=info`，LoongArch64 kernel build 成功；warning 数量与 RISC-V64 一致，为 6 个。
- 剩余风险：尚未运行 QEMU testsuite wrapper；`basic` 文件/fd 子集和 BusyBox 文件操作段仍需运行时验证。

## 2026-04-26 09:59 CST BusyBox 文件命令 RV/LA 运行验证

- 范围和目标：修复 BusyBox 文件操作段中 `mv test_dir test` 失败，并验证用户指定的 `touch`、`cat`、`cut`、`od`、`head`、`tail`、`hexdump`、`md5sum`、`sort`、`uniq`、`stat`、`wc`、`more`、`rm`、`mkdir`、`mv`、`rmdir`、`cp`、`find` 在 RISC-V64 与 LoongArch64 上通过。
- 修改文件：`examples/shell/src/uspace.rs`，`docs/development/policies/compatibility.md`，`docs/development/interfaces/syscall-inventory.md`，`doc/logs/2026-04-26-busybox-filesystem.md`。
- 关键决策：`sys_renameat2` 仍优先调用真实 `axfs::api::rename`；仅当真实 rename 返回 `EOPNOTSUPP` 或 `ENOSYS` 时进入 `compat_empty_dir_rename`。兼容路径只处理源为目录、目标不存在的目录 rename，执行 create-destination/remove-source，并在 remove-source 失败时回滚新目录；不对非目录源、已有目标或非空目录返回假成功。
- 验证结果：已执行 `QEMU_TIMEOUT=240s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-busybox-fs-rv.log`。第一次运行定位到 `mv test_dir test` 失败；修复后第二次运行中 basic-musl 文件/fd 子集通过，busybox-musl 文件操作段中用户指定命令全部 success。完成 busybox 后停止容器内 QEMU，保留 `arceos-eval-fix`。
- 验证结果：已执行 `QEMU_TIMEOUT=240s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info | tee /tmp/arceos-basic-fsfd-after-la.log`。basic-musl 文件/fd 子集通过，busybox-musl 文件操作段中用户指定命令全部 success。完成 busybox 后停止容器内 QEMU，保留 `arceos-eval-fix`。
- 验证结果：已执行 `python3 testsuits-for-oskernel/basic/user/src/oscomp/test_runner.py /tmp/arceos-busybox-fs-rv.log` 与 `python3 testsuits-for-oskernel/basic/user/src/oscomp/test_runner.py /tmp/arceos-basic-fsfd-after-la.log`；RV/LA 的 basic 文件/fd 相关项目均通过。RV parser 中 `test_yield` 有非文件相关失败项，未归入本次 filesystem/fd 目标。
- 剩余风险：`compat_empty_dir_rename` 非原子且仅覆盖空目录 rename；完整目录 rename、覆盖目标、跨文件系统 rename、权限、symlink、`FD_CLOEXEC`、完整 `fcntl` 和真实 runtime mount/devfs 仍待后续阶段。

## 2026-04-26 10:10 CST fd flags 与 close-on-exec 接入

- 范围和目标：补齐 fd slot 层的 `FD_CLOEXEC` 保存、`dup3(O_CLOEXEC)`、`fcntl` fd flags/status flags 查询设置，以及 `execve` 成功路径的 close-on-exec 行为。
- 修改文件：`examples/shell/src/linux_fs/fd.rs`，`examples/shell/src/uspace.rs`，`docs/development/interfaces/syscall-inventory.md`，`doc/logs/2026-04-26-busybox-filesystem.md`。
- 关键决策：`FdTable.entries` 从裸 `FdEntry` 升级为 `FdSlot { fd_flags, entry }`；`FD_CLOEXEC` 只存在于 fd slot，`O_APPEND`/`O_NONBLOCK` 等 status flags 保存在共享 `OpenFileDescription`；`dup`/`F_DUPFD` 创建的新 fd 清除 `FD_CLOEXEC`，`dup3(O_CLOEXEC)` 与 `F_DUPFD_CLOEXEC` 显式设置该标志；`execve` 仅在新镜像加载成功后关闭 CLOEXEC fd，失败时保留原 fd 表。
- 验证结果：未执行构建或 QEMU 测试；等待下一步授权/执行验证。
- 剩余风险：`F_SETFL` 当前只更新文件/目录 OFD status flags，未实现文件锁、异步 I/O 标志、权限模型或完整 Linux fcntl 错误矩阵；`execve` 原子替换仍未完整实现。

## 2026-04-26 10:17 CST - fd flags / close-on-exec 验证收尾

- 变更范围和目标：补齐 shell Linux ABI 层的 fd flags、open file description 状态标志共享、`dup`/`dup3`/`fcntl` 语义和 `execve` 成功后的 close-on-exec 行为，服务 BusyBox 文件管理命令和基础 fd 兼容测试。
- 涉及文件：
  - `examples/shell/src/linux_fs/fd.rs`
  - `examples/shell/src/linux_fs/mod.rs`
  - `examples/shell/src/uspace.rs`
  - `docs/development/interfaces/syscall-inventory.md`
  - `doc/logs/2026-04-26-busybox-filesystem.md`
- 关键决策：
  - 将 per-fd 的 `FD_CLOEXEC` 保存在 fd table slot 中，避免与 open file description 状态混淆。
  - 将 `O_APPEND`、`O_NONBLOCK` 等文件状态标志保存在共享 open file description 中，使 `dup`/`fork` 后共享偏移和状态符合 Linux 语义。
  - `execve` 仅在新程序装载成功后关闭带 `FD_CLOEXEC` 的 fd，失败路径保留原 fd table。
  - `fcntl(F_SETFL)` 只接受当前已建模的状态标志集合，未建模能力不伪造成功语义。
- 验证结果：
  - `docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info`：通过，构建输出无 Rust warning。
  - `docker exec arceos-eval-fix make -C /workspace/arceos kernel-la KERNEL_LOG=info`：通过，构建输出无 Rust warning。
  - `QEMU_TIMEOUT=240s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-busybox-fdflags-rv.log`：basic 文件/fd 子集通过，BusyBox 文件管理命令段通过；非本任务范围的 RV `test_yield` 仍存在解析失败记录。
  - `QEMU_TIMEOUT=240s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info | tee /tmp/arceos-busybox-fdflags-la.log`：basic 文件/fd 子集通过，BusyBox 文件管理命令段通过。
  - `python3 testsuits-for-oskernel/basic/user/src/oscomp/test_runner.py /tmp/arceos-busybox-fdflags-rv.log`：文件/fd 相关用例通过，RV `test_yield` 保持既有非文件系统问题。
  - `python3 testsuits-for-oskernel/basic/user/src/oscomp/test_runner.py /tmp/arceos-busybox-fdflags-la.log`：文件/fd 相关用例通过。
- 剩余风险和后续任务：
  - `fcntl` 文件锁语义仍未实现，当前保持显式兼容边界。
  - `F_SETFL` 仅覆盖 `O_APPEND`/`O_NONBLOCK` 等已建模状态，完整 Linux 标志矩阵仍需后续扩展。
  - `execve` 的进程镜像原子替换语义仍有更大范围改造空间，本次只保证 close-on-exec 的成功路径位置正确。
  - 后续可增加专门的 close-on-exec 运行时回归用例，避免仅依赖 BusyBox/basic 覆盖。

- 验证补充：BusyBox RV/LA 日志中 `mv test_dir test` 前仍会出现底层 `axfs_vfs` 的 `AxError::Unsupported` 打印，这是 `axfs::api::rename` 暂不支持目录 rename 后进入 `compat_empty_dir_rename` 的预期回退路径；命令最终返回 success。后续删除条件仍是底层 `axfs` 支持真实目录 rename。

- 收尾构建确认：本轮收尾再次执行 `docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info` 与 `docker exec arceos-eval-fix make -C /workspace/arceos kernel-la KERNEL_LOG=info`，两者均返回 exit 0。

## 2026-04-26 10:25 CST - 本轮 BusyBox 文件管理与 fd 语义任务详细总结

- 任务背景：用户目标是让 `scripts/busybox/busybox_testcode.sh` 中的 BusyBox 文件管理命令在 ArceOS shell syscall 路径下可维护地通过，同时补齐 fd flags / close-on-exec 这类基础 Linux fd 语义。覆盖命令包括 `touch`、`cat`、`cut`、`od`、`head`、`tail`、`hexdump`、`md5sum`、`sort`、`uniq`、`stat`、`wc`、`more`、`rm`、`mkdir`、`mv`、`rmdir`、`cp`、`find`。
- 总体方案：把兼容语义收敛在 `examples/shell/src/linux_fs/` 和 `examples/shell/src/uspace.rs` 这条 shell Linux ABI 路径中，不为了 BusyBox workload 修改 `modules/axfs/**`，不按命令名、测试脚本路径或固定文件名添加特判。
- 主要入口和调用路径：`sys_openat`、`sys_read`、`sys_write`、`sys_lseek`、`sys_pread64`、`sys_ftruncate`、`sys_fstat`、`sys_getdents64`、`sys_mkdirat`、`sys_unlinkat`、`sys_faccessat`、`sys_newfstatat`、`sys_statx`、`sys_renameat2`、`sys_dup`、`sys_dup3`、`sys_fcntl`、`sys_execve`。
- 逐文件职责：
  - `examples/shell/src/linux_fs/path.rs`：新增 `ResolveOptions`、`ResolvedPath`、`resolve_at_path`，统一处理 `AT_FDCWD`、dirfd base、绝对路径、相对路径、空路径和 trailing slash。
  - `examples/shell/src/linux_fs/fd.rs`：建立 `FdFlags`、`OpenStatusFlags`、`OpenFileDescription` 和 `OpenFileBackend`，把文件/目录后端、共享 offset、共享 status flags 放入 open file description。
  - `examples/shell/src/linux_fs/mod.rs`：导出 shell syscall 层需要的 fd/path 类型，避免 `uspace.rs` 直接依赖过多内部模块细节。
  - `examples/shell/src/uspace.rs`：将 fd table entry 升级为 `FdSlot { fd_flags, entry }`，接入共享 OFD、dirfd path resolver、`fcntl`、`dup`/`dup3`、`execve` close-on-exec 和目录 rename 兼容回退。
  - `docs/development/policies/compatibility.md`：记录 `compat_empty_dir_rename` 的兼容边界和删除条件。
  - `docs/development/interfaces/syscall-inventory.md`：同步本轮涉及 syscall 的支持状态和语义边界。
  - `docs/superpowers/specs/2026-04-26-arceos-busybox-filesystem-design.md` 与 `docs/superpowers/plans/2026-04-26-arceos-busybox-filesystem-implementation-plan.md`：保留本轮设计和实现计划记录。
- fd/OFD 关键语义：
  - `FD_CLOEXEC` 是 per-fd 属性，保存在 `FdSlot.fd_flags`，不放进 open file description。
  - `O_APPEND`、`O_NONBLOCK` 等 status flags 是 open file description 属性，保存在共享 `OpenFileDescription.status_flags`。
  - `dup` 和 `F_DUPFD` 复制 fd 引用但清除新 fd 的 `FD_CLOEXEC`。
  - `dup3(..., O_CLOEXEC)` 和 `F_DUPFD_CLOEXEC` 为新 fd 设置 `FD_CLOEXEC`。
  - `fork_copy` 复制 fd table slot，但文件/目录 fd 继续共享同一个 `Arc<OpenFileDescription>`，保持共享 offset 和 status flags。
  - `pread64` 通过 explicit offset 读取，不推进共享 OFD offset。
  - `execve` 仅在新程序装载成功后调用 `close_cloexec`，失败路径保留原 fd table。
- 路径与 metadata 关键语义：
  - path-taking syscall 统一通过 dirfd-aware resolver 转为绝对路径，减少 `openat`、`mkdirat`、`unlinkat`、`faccessat`、`newfstatat`、`statx` 各自处理路径的分叉。
  - `stat_path_abs` 优先复用真实 `File::open(...).get_attr()` 与目录打开路径获取 `FileAttr`，避免伪造一套 metadata 转换。
  - `openat` 保留现有 runtime candidate 查找逻辑，避免动态库/测试程序路径查找回归。
- `mv`/rename 兼容语义：
  - `sys_renameat2` 仍优先调用 `axfs::api::rename`。
  - 只有真实 rename 返回 `EOPNOTSUPP` 或 `ENOSYS` 时才进入 `compat_empty_dir_rename`。
  - `compat_empty_dir_rename` 只处理“源是目录、目标不存在、空目录 rename”这一窄场景。
  - 不支持非空目录、覆盖目标、跨文件系统、非目录源的伪成功。
  - 回退流程为 `create_dir(new)` 后 `remove_dir(old)`，若删除源目录失败则尝试回滚目标目录。
  - 删除条件：底层 `axfs` 支持真实目录 rename 后，应删除 `compat_empty_dir_rename` 并改由真实 VFS/filesystem 语义负责。
- 验证证据：
  - `docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info`：RISC-V64 kernel build exit 0。
  - `docker exec arceos-eval-fix make -C /workspace/arceos kernel-la KERNEL_LOG=info`：LoongArch64 kernel build exit 0。
  - `QEMU_TIMEOUT=240s ARCH=riscv64 ./run-testsuite-bench-rv-direct.sh KERNEL_LOG=info | tee /tmp/arceos-busybox-fdflags-rv.log`：运行到 busybox-musl 文件命令段，用户指定 BusyBox 文件命令均输出 success；完成目标段后仅停止容器内 QEMU，保留 `arceos-eval-fix`。
  - `QEMU_TIMEOUT=240s ARCH=loongarch64 ./run-testsuite-bench-la-direct.sh KERNEL_LOG=info | tee /tmp/arceos-busybox-fdflags-la.log`：运行到 busybox-musl 文件命令段，用户指定 BusyBox 文件命令均输出 success；完成目标段后仅停止容器内 QEMU，保留 `arceos-eval-fix`。
  - `python3 testsuits-for-oskernel/basic/user/src/oscomp/test_runner.py /tmp/arceos-busybox-fdflags-rv.log`：basic 文件/fd 相关项通过；RV `test_yield` 仍有既有非 filesystem/fd 失败。
  - `python3 testsuits-for-oskernel/basic/user/src/oscomp/test_runner.py /tmp/arceos-busybox-fdflags-la.log`：basic 解析项通过，包含文件/fd 相关项。
  - BusyBox 文件命令确认：RV/LA 日志中 `touch`、`cat`、`cut`、`od`、`head`、`tail`、`hexdump`、`md5sum`、`sort|uniq`、`stat`、`wc`、`more`、`rm`、`mkdir`、`mv`、`rmdir`、`cp`、`find` 均有 success 记录。
- 已知非目标现象：
  - RV basic parser 中 `test_yield` 存在失败，不属于本轮 filesystem/fd 目标。
  - busybox 前置段中的 `hwclock` 失败与 `/dev/misc/rtc` 缺失相关，不属于本轮文件管理命令目标。
  - `mv test_dir test` 前的 `AxError::Unsupported` 是底层目录 rename 不支持触发兼容回退的可见日志，不代表 BusyBox `mv` 命令失败。
- 剩余风险：
  - `fcntl` 文件锁、异步 I/O、完整 `F_SETFL` 标志矩阵尚未实现。
  - `execve` 的完整进程镜像原子替换语义仍需后续系统性整理，本轮只保证 close-on-exec 的成功路径位置。
  - symlink、权限模型、设备节点、`statfs/fstatfs` 和真实 runtime mount/devfs 仍需后续补齐。
  - `compat_empty_dir_rename` 非原子，只适合当前 BusyBox 空目录 rename 场景，不能替代真实 filesystem rename。
  - 目前 close-on-exec 没有独立回归测试，后续应增加一个专门用例验证 `O_CLOEXEC`、`F_SETFD(FD_CLOEXEC)`、`dup` 清除 CLOEXEC、`dup3(O_CLOEXEC)` 设置 CLOEXEC 和 failed exec 保留 fd table。
- 后续建议：
  - 增加 fd flags 专项 basic 测例，覆盖 `F_GETFD/F_SETFD/F_GETFL/F_SETFL/F_DUPFD/F_DUPFD_CLOEXEC/dup3`。
  - 在 `axfs` 或 VFS 层支持真实目录 rename 后删除 `compat_empty_dir_rename`。
  - 继续把 syscall 语义边界写入 `docs/development/interfaces/syscall-inventory.md`，避免日志和实际代码状态分离。
