# 开发日志：filesystem/fd 兼容层重构与防投机规范落地

**时间**: 2026-04-25（更新）
**仓库**: `/home/majiaqi/Github/OS_Projects/arceos`
**关注范围**: 文件系统/文件描述符方向在 basic 套件关键项与团队长期接口治理

## 0. 这份日志为什么要写

本日志用于把“临时修补”行为彻底替换为长期可维护开发过程。目标有三个：

1. 让你能快速判断现在代码到底做了什么，不靠记忆。 
2. 把测例需求与代码决策绑定，避免后续人换手时丢掉语义边界。 
3. 建立一个“不会反复走回头路”的工程约束：禁止测试驱动的假成功，实现有状态可验证语义。

---

## 1. 业务背景与最初约束

### 1.1 业务目标

用户初始目标是进入 arceOS 开发，重点先通过以下关键测例：

- `basic`：`chdir`、`close`、`dup`、`dup2`、`fstat`、`getcwd`、`getdents`、`mkdir_`、`mount`、`open`、`openat`、`pipe`、`read`、`umount`、`unlink`、`write`
- 后续由队友负责更多领域（busybox/iozone/...）

### 1.2 用户反复强调的约束（高优先级）

- 不要走“硬编码/伪实现”思路过测例。
- 不要修改 `axfs`，先在其外面做 ABI/语义层封装。
- 分阶段推进：先做当前里程碑必要功能，不要一次性做过大范围重构。
- 若有兼容路径，必须可审计、可回退、有明确删除条件。
- 容器运行约束：Rust 编译/测试在长期存在的 `arceos-eval-fix` 容器里执行，不可删除容器。

### 1.3 为什么会出现“伪实现问题”

最初实现中出现了多个高风险点：

- `mount` / `umount2` 仅做参数检查返回，缺乏状态。
- `statx` mask 行为不严谨。
- 挂载、测试路径有时出现过于依赖测试环境路径特征。

这些问题一旦通过 basic 即可运行，但后续会在 LTP/iozone/busybox 出现不确定行为。最终方向是“先不赌测例”，改为可复用语义层。

---

## 2. 关键问题清单（来自你给出的批注）与处理

以下问题贯穿了整个开发决策：

### 2.1 getcwd 返回值语义

问题：`SYS_getcwd` 的返回值在计划/实现里不够清晰。

- **Linux 风格期望**：成功返回 `buf` 用户地址。
- **风险**：如果返回长度，只在个别 libc 封装下“偶然兼容”。
- **处理**：在兼容层语义内明确定义并接入调用方要求，避免返回长度误用。

### 2.2 dup3 flags

问题：`dup3` 的 `flags != 0` 未统一处理。

- **用户要求**：当前里程碑只允许 `flags == 0`。
- **处理策略**：在兼容语义边界先明确 `EINVAL` 兜底；后续再根据能力扩展。

### 2.3 getdents 对目录打开方式

问题：测例使用 `open(".", O_RDONLY)` 来读取目录。

- **处理**：目录路径要被解析为可进行 `getdents` 的 fd 语义（即使不带 `O_DIRECTORY`）。这点在路径/文件语义协作中记录为前置约束。

### 2.4 mount/umount 状态一致性

问题：仅参数形状通过导致“全都 return 0”。

- **解决方案**：引入最小状态机：`mount` 成功后记录 target；`umount2` 仅对已挂载目标成功，否则明确返回错误。

### 2.5 close stdio 与兼容差异

问题：以前代码有“保留 stdio 不关闭”倾向。

- **处理**：明确该行为属于兼容层决策，不允许无故偏离 Linux 可见语义；涉及 close 的行为变化保持显式记录与文档约束。

---

## 3. 架构决策：引入 linux_fs 语义层（非新 VFS）

### 3.1 约定

我们明确约定：`linux_fs` 是 Linux ABI 语义层，不是新的 VFS。它只做：

- 路径规则与参数规范化
- errno 策略
- mount/umount 的兼容状态
- stat/statx 的投影策略
- 未来承接 OFD/fdModel 的接口预备（此阶段不贸然迁移）

后端真实能力仍由 `axfs`（或现有内核文件后端）提供。

### 3.2 为什么这样做

原因是把测例兼容逻辑“包在 uspace”里，避免污染底层文件系统接口，保持可替换性：

- 后续若替换后端能力时，只改底层真实实现，不影响上层语义约定；
- 若兼容范围收紧/扩展，修改集中在 `linux_fs`。

---

## 4. 文件级实现过程（按模块）

### 4.1 新增模块与入口

- `examples/shell/src/linux_fs/mod.rs`
- `examples/shell/src/linux_fs/types.rs`
- `examples/shell/src/linux_fs/path.rs`
- `examples/shell/src/linux_fs/mount.rs`
- `examples/shell/src/linux_fs/stat.rs`
- `examples/shell/src/linux_fs/fd.rs`
- `examples/shell/src/main.rs` 新增 `mod linux_fs;`

`mod.rs` 作为最小门面（facade），为 `uspace.rs` 暴露稳定的调用入口，减少以后重构时上层联动。

### 4.2 path.rs 纯函数迁移

#### 目标

第一阶段不允许 `path.rs` 依赖 `UserProcess` 或 `FdTable`，避免耦合膨胀。

#### 实施

- 抽出 `resolve_cwd_path`、`normalize_path` 等路径转换逻辑。
- 行为尽量仅处理字符串/路径规范，避免文件系统状态依赖。

#### 影响

- `open`、`chdir` 等 syscall 侧路径构建更一致；
- 为后续 `AT_FDCWD` 做接口留口。

### 4.3 mount.rs 状态机化

#### 目标

解决 mount/umount 的“看起来成功但不可逆”问题。

#### 实施

- 加入 `MountTable` 管理挂载目标。
- 引入 `MountRequest/UmountRequest` 描述兼容请求。
- `compat_basic_mount` 保留作为当前测试兼容入口，但不扩张规则。
- `umount2` 通过状态判断返回：未挂载返回错误，而不是盲目成功。

#### 结果

- mount 与 umount 行为可被逆向验证；
- 避免“valid-looking 参数直接放行”。

### 4.4 stat.rs 投影和 mask 约束

#### 目标

保证 `statx` 的字段可见性与 mask 语义可信。

#### 实施

- `stat_to_statx` 按 `requested_mask & STATX_SUPPORTED_MASK` 填充 `stx_mask`。
- 明确 `AT_EMPTY_PATH` 与 `validate_statx_flags` 入口。

#### 结果

- 避免把用户请求 mask 原样回显导致的不透明行为。
- 为后续扩展提供能力位边界。

### 4.5 uspace 接入

#### 具体改造

- `examples/shell/src/uspace.rs` 将 syscalls 的逻辑入口切换到 `linux_fs`：
  - `mount_table` 从 `UserProcess` 迁移语义到 `linux_fs::MountTable`；
  - getcwd/openat/getdents/statx/umount 等路径、mask、状态判断交给语义层。

#### 结果

- 行为点集中：兼容策略在一处可审计；
- 仍保持底层文件操作接口路径。

---

## 5. 文档治理过程（不是一次性文档）

### 5.1 为什么要重构文档位置

你明确指出：

- 不能把长期约定写在一次性 `spec` 里；
- 需要按接口域分文档；
- 通过目录索引快速检索到相关规范，减少上下文开销。

### 5.2 已建立文档入口

以下文档形成持续使用链路（后续均按域继续扩展）：

- `doc/README.md`（开发文档入口）
- `doc/development/README.md`（开发进度、范围、gate）
- `doc/development/interfaces/filesystem.md`（linux_fs 边界和接口职责）
- `doc/development/policies/compatibility.md`（兼容策略与退出机制）

### 5.3 AGENTS 更新

为了把规则变成“执行约束”，更新了：

- 根工作区 `AGENTS.md`
- `arceos/AGENTS.md`
- `doc/development/policies/compatibility.md`

新增原则包括：

- 禁止硬编码工作负载名称/命令字符串/路径特征返回成功；
- 禁止测试驱动的 `if path==xxx => pass`；
- 不允许伪实现返回“假成功”；
- 兼容路径命名统一使用 `compat_*`，并要求带可删除条件。

---

## 6. 构建与验证（本阶段）

> 本阶段以 basic fs/fd 关键项的闭环为主，并未展开所有 workload。

已执行：

1. `docker exec arceos-eval-fix make -C /workspace/arceos fmt`
2. `docker exec arceos-eval-fix make -C /workspace/arceos kernel-rv KERNEL_LOG=info`
3. `docker exec arceos-eval-fix make -C /workspace/arceos kernel-la KERNEL_LOG=info`
4. 解析 `basic` fs/fd 关键项日志，RV/LA 均显示对应项通过。

容器使用要求已满足：在长期容器 `arceos-eval-fix` 中执行，不删除容器。

---

## 7. 对“短期通过 vs 长期正确”的取舍与当前状态

这次修复不是追求面面俱到，而是“先过关键闭环，保留未来扩展口”。当前状态定义为：

- 可维护性：兼容语义集中在 shell 侧 `linux_fs`。
- 测试连续性：basic fs/fd 路径具备可复现稳定性。
- 可扩展性：fd/OFD/dirfd/path 与更高并发语义有明确留口。

未覆盖点有意保留：

- 完整 OFD 与 fd 表迁移（当前 fd.rs 为占位）
- `AT_EMPTY_PATH` 以外的高级路径 flag 覆盖
- 真正 VFS 级挂载与设备文件系统整合

---

## 8. 风险记录（需在下一阶段处理）

1. `dup/fork/clone/read/write` 共享文件描述语义仍需统一。
2. `open` 的目录识别、`getdents`/`lseek` 在共享 offset 场景下还需收口。
3. mount/umount 若进入更高版本测试，需明确与真实 VFS 的接口转换。
4. 大量接口还未建立完整 errno 矩阵（fs 以外领域更明显）。

---

## 9. 下一步交接清单（给你直接用）

**下一阶段第一优先级**：

- 完成 `doc/interfaces` 按域扩展（内存/进程/调度/IPC/网络）
- 增加接口契约模板（强约束与示例 API 分离）
- 对 `mount/stat/path/getdents` 增加最小行为测试清单，避免回退到“看起来过得了基础测例”。

**第二优先级**：

- 将 `fd.rs` 从占位到最小可复用 OFD 表语义（仅在内核 shell 侧，先不碰 axfs）
- 建立 errno 统一策略文档（unsupported flag/syscall/能力缺失场景）

---

## 10. 本文件用途说明

你可以把这份日志当作：

- 团队交接说明（为何这么做）
- 评审依据（为啥不改 axfs、为什么要兼容状态机）
- 回归依据（验证过什么）
- 下一步开工前置（避免重复讨论，直接看风险条目）

