# 06 | 兼容版 SysV 共享内存（shm*）实现

这一章是“临时配方”。  
它不是真正的 SysV IPC 子系统，而是兼容层：让 `shmget/shmat/shmdt/shmctl` 在当前 workload 下能跑通。

类比一下：  
你可以把它看成“酒店前台临时拼出来的储物柜系统”：有编号、有容量、有计数，但不追求完整的 IPC 规范。

## 1. 为何引入 compat shm

一些基准或测试用例会依赖：

- `IPC_PRIVATE` 创建私有段；
- `shmat` 映射到用户地址；
- `shmdt` 释放；
- `shmctl(shmid, IPC_RMID)` 标记删除。

如果没有这个层，测试会直接报 `ENOTSUPP` 或 `EINVAL`。  
而完整实现通常要系统级 IPC 命名空间、权限、`shmget` 键管理，这些在当前阶段是额外复杂度。

## 2. 数据模型

### 2.1 `CompatShmSegment`

一个共享段记录：

- `size`: 对齐后的总字节数；
- `pages`: 分页数量；
- `kernel_vaddr`: 内核虚拟地址（由 axalloc 分配）；
- `phys_start`: 物理起始地址（给 `map_linear` 用）；
- `marked_removed`: 是否被 `IPC_RMID` 标记；
- `attachments`: 当前挂载到多少个进程。

### 2.2 `CompatShmRegistry`

全局静态的注册表，核心字段：

- `next_id`: 下一个 shmid；
- `segments: BTreeMap<i32, CompatShmSegment>`：`shmid -> segment`。

通过 `LazyInit<Mutex<CompatShmRegistry>>` 提供单例，避免初始化顺序问题。

### 2.3 `UserProcess::shm_attachments`

每个进程自己维护：

- `BTreeMap<usize, i32>`：`user_virtual_addr -> shmid`；
- 方便进程退出时统一清理。

## 3. 生命周期函数

### 3.1 `compat_shm_table()`

返回全局单例 `&'static Mutex<CompatShmRegistry>`，未初始化则创建空表。

### 3.2 `compat_shm_free_segment(segment)`

调用 `axalloc::global_allocator().dealloc_pages` 释放内核页。

### 3.3 `compat_shm_segment_size(shmid)`

返回段大小；如果 id 不存在则 `None`。

## 4. 分配与登记：`CompatShmRegistry::allocate_private(size)`

`shmget(IPC_PRIVATE, size, flags)` 会走到这里：

1. `size == 0` => `EINVAL`；
2. 按页向上对齐；
3. `axalloc` 分配物理页；
4. 写 0 初始化；
5. 通过 `virt_to_phys` 记录 `phys_start`；
6. 用 `next_id` 发号；
7. 入库返回 `shmid`。

`next_id` 用 `checked_add` 保护溢出后回绕到 1，避免产生 0。

## 5. attach / detach / remove 的状态机

### 5.1 `compat_shm_prepare_attach(shmid)`

前提：`shmid` 必须存在且 `marked_removed == false`。

- `attachments + 1`（有溢出保护）；
- 返回 `(phys_start, size)` 给映射器。

### 5.2 `compat_shm_detach(shmid)`

查表减一，不存在则直接 return。  
如果该段已标记删除且 attachment 变成 0，则立刻释放内存。

### 5.3 `compat_shm_mark_removed(shmid)`

将 `marked_removed = true`。  
若附件为 0，立即释放段并从表删掉；否则等最后一次 detach。

### 5.4 `compat_shm_clone_attachments(attachments)`

fork 时调用：把父进程的 attachments 数量“平移”到子进程，因子进程会和父进程共享已映射段，因而需要提前加一。

失败条件：

- 段不存在 => `EINVAL`；
- 附件计数加一溢出 => `EINVAL`。

## 6. 用户态系统调用链

### 6.1 `sys_shmget(key, size, flags)`

- `flags` 仅允许 `IPC_CREAT | IPC_EXCL | 0o777`。
- 非 `IPC_PRIVATE` => `EOPNOTSUPP`（当前仅支持私有段）；
- 通过 `CompatShmRegistry::allocate_private(size)` 建表；
- 成功返回 `shmid`。

### 6.2 `sys_shmat(process, shmid, shmaddr, shmflg)`

支持限制很严格（兼容最小集）：

- `shmflg` 仅允许 0 或组合 `SHM_RDONLY/SHM_RND/SHM_REMAP`，不匹配 => `EINVAL`；
- `shmaddr != 0` 或 `shmflg != 0` => `EOPNOTSUPP`（这个版本不允许指定地址）。

流程：

1. `compat_shm_prepare_attach`；
2. 在进程 brk 区域 `next_mmap` 处挑选 `start`，按页对齐；
3. `target = align_up(next_mmap, PAGE_SIZE_4K)`，并更新 `next_mmap`；
4. 空间不足（低于 `USER_MMAP_BASE` 或碰到用户栈保留）=> `compat_shm_detach` 反向回滚；
5. `aspace.map_linear(target, phys_start, size, MAP_ANON...)` 映射；
6. `shm_attachments` 记下 `target -> shmid`；
7. 返回 `target` 地址给用户。

### 6.3 `sys_shmdt(process, shmaddr)`

- `shmaddr` 必须在当前进程附件表里存在；
- 取到 `shmid` 后查大小；
- 从地址空间 `unmap`；
- 若 `unmap` 失败则把映射记录放回原地；
- 成功后 `compat_shm_detach(shmid)`。

### 6.4 `sys_shmctl(shmid, cmd, buf)`

只支持 `cmd == IPC_RMID`；  
其它都返回 `EINVAL`。  
执行 `compat_shm_mark_removed`。

## 7. 与进程生命周期联动（高价值点）

### 7.1 `UserProcess::teardown`

新增清理顺序：

- `compat_itimer_real_disarm`
- `detach_all_compat_shm`
- `aspace.clear`
- `fds.close_all`
- `mount_table.clear`

### 7.2 `UserProcess::note_thread_exit`

当线程计数归零时也会 `detach_all_compat_shm`，并清理文件描述符、唤醒 wait。  
避免多线程只退出一个后，另一个线程残留 shm 映射。

### 7.3 `UserProcess::fork`

`fork` 时从父进程克隆 `shm_attachments` 并调用 `compat_shm_clone_attachments`，新子进程继承共享段引用。

### 7.4 `UserProcess::detach_all_compat_shm`

退出清理遍历 `shm_attachments`：

- 先 `take` 全部映射记录；
- 尝试 `unmap` 每段；
- 对每个 `shmid` 调 `compat_shm_detach`。  

这是“多进程/多附件一致性的关键回收点”。

## 8. 已定义的兼容边界

- 不支持命名键（非 `IPC_PRIVATE`）；
- 不支持 `shmaddr` 指定映射地址；
- `shmctl` 命令集很窄；
- 没有权限位、uid/gid、`shm_nattch` 等完整字段。

注释里每个关键分支都写了“delete-when”计划：当出现真实 IPC 模块后可移除。

## 9. 本章函数速查

- `compat_shm_table`
- `compat_shm_free_segment`
- `compat_shm_prepare_attach`
- `compat_shm_detach`
- `compat_shm_mark_removed`
- `compat_shm_segment_size`
- `compat_shm_clone_attachments`
- `CompatShmRegistry::allocate_private`
- `sys_shmget`
- `sys_shmat`
- `sys_shmdt`
- `sys_shmctl`
- `UserProcess::detach_all_compat_shm`
