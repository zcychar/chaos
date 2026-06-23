# B04 IPC 详细迁移记录

本文是 B04 的批次级执行记录。它把 [migration-batches.md](../migration-batches.md) 中的 B04 拆成函数级/子模块级迁移单元，用于在源码修改前明确：目标文件/行、`kernel.rs` 语义来源、当前 rCore 表达、接口处理方式、最小修改范围和验收点。

当前状态：B04.1 semaphore create/get、B04.2 semaphore undo/remove、B04.3 shared memory key/size 已按行级报告完成源码迁移。

```text
批准执行 B04 ipc 源码迁移。
```

## 批次定位

| 项目 | 内容 |
| --- | --- |
| 批次 | B04 IPC |
| 恢复模块 | `kernel/src/ipc/*`，并联动现有 `kernel/src/syscall/ipc.rs` |
| `kernel.rs` 来源行段 | `4158-4386` |
| 上游基线 | 恢复源码与 rCore commit `66cb4181ec6d3336d507c7c1ff100127f56fcc0a` hash 一致 |
| 接口原则 | 保留 rCore 的 `SemArray`、`SemProc`、`ShmIdentifier`、`ShmProc` 和 syscall IPC 分发结构，不复制 `kernel.rs` 的模拟 `SemArr`/`SemCtx`/`ShmCtx` 类型 |

## B04.1 semaphore create/get

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/ipc/semary.rs:95`、`:117`、`kernel/src/syscall/ipc.rs:15` |
| rCore 函数 | `SemArray::get_or_create`、`Syscall::sys_semget` |
| `kernel.rs` 来源 | `SemArr::get_or_create`，`kernel/src/kernel.rs:4209-4260` |
| 迁移语义 | 创建 semaphore array 时拒绝 `nsems == 0`；existing key 请求更大的 `nsems` 时返回错误；`IPC_CREAT | IPC_EXCL` 对 existing key 返回 exists。 |
| 修改前 rCore 表达 | `sys_semget` 只检查 `nsems > SEMMSL`；`SemArray::get_or_create` existing key 直接返回 array；new array 用 `0..nsems`，允许 zero-length。 |
| 接口处理 | 保留 `SemArray::get_or_create(...) -> Result<Arc<Self>, SysError>`；错误映射使用现有 `SysError::{EINVAL,EEXIST}`。 |
| 最小修改范围 | `sys_semget` 的 `nsems == 0` 前置校验；`SemArray::get_or_create` existing-key count 检查。 |
| 不应修改 | `SemidDs` ABI、`IpcPerm` ABI、global `KEY2SEM` 存储形状。 |
| 验收点 | `semget(key,0,...)` 返回 `EINVAL`；existing key 且请求 count 更大返回 `EINVAL`；existing key 且 `CREAT|EXCLUSIVE` 返回 `EEXIST`；合法 existing key 仍返回原 array。 |
| 已落地结果 | `sys_semget` 拒绝 `nsems == 0`；`SemArray::get_or_create` 内部也拒绝 zero-size；existing key 且已有 `nsems` 小于请求 `nsems` 时返回 `EINVAL`；`IPC_CREAT | IPC_EXCL` 的 `EEXIST` 行为保持。 |
| 验收结果 | `rustfmt --edition 2018 kernel/src/ipc/semary.rs kernel/src/syscall/ipc.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止。 |
| 状态 | `MIGRATED` |

## B04.2 semaphore undo/remove

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/ipc/mod.rs:47`、`:62`、`:80`、`kernel/src/syscall/ipc.rs:56` |
| rCore 函数 | `SemProc::{remove,add_undo}`、`Drop for SemProc`、`Syscall::sys_semctl` IPC_RMID 分支 |
| `kernel.rs` 来源 | `SemCtx::{remove,add_undo,drop}`，`kernel/src/kernel.rs:4268-4313` |
| 迁移语义 | remove 某 semaphore id 时清理该 id 的 undo；进程 drop 时 replay 完整正向 undo magnitude，不只处理 `1`；已删除 array 的 stale undo 不应 panic。 |
| 修改前 rCore 表达 | `SemProc::remove` 只移除 array；`Drop for SemProc` 对 undo 值非 `0/1` 走 `unimplemented!`，并直接索引 `self.arrays[&id]`。 |
| 接口处理 | 保留 `SemProc` public 方法；undo replay 继续使用 `Semaphore::release` 或等价操作；缺失 id 跳过而不是 panic。 |
| 最小修改范围 | `SemProc::remove` 清理该 id 的 undo 项；`Drop for SemProc` 对 positive undo 值循环或等价 release；必要的 bounds/id 检查。 |
| 不应修改 | `Semaphore` public API、`sys_semop` 用户参数 ABI。 |
| 验收点 | IPC_RMID 后不保留该 id 的 undo；drop 遇到 op > 1 不 panic；drop 遇到 stale id 不 panic；SEM_UNDO 累积值按 magnitude replay。 |
| 已落地结果 | `SemProc::remove` 移除 array 后同步清理该 id 的 undo 项；`Drop for SemProc` 通过 `get` 跳过 stale id，检查 semaphore num 上界，并对正向 undo magnitude 循环 `release()`；为兼容当前旧 nightly/alloc，undo 清理使用“先收集 key 再 remove”的写法，不使用 `BTreeMap::retain`。 |
| 验收结果 | `rustfmt --edition 2018 kernel/src/ipc/mod.rs` 通过；首次构建发现旧工具链无 `BTreeMap::retain` 后已改为兼容写法；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止。 |
| 状态 | `MIGRATED` |

## B04.3 shared memory key/size

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/ipc/shared_mem.rs:27`、`kernel/src/syscall/ipc.rs:113` |
| rCore 函数 | `ShmIdentifier::new_shared_guard`、`Syscall::sys_shmget` |
| `kernel.rs` 来源 | `shm_get_or_create`，`kernel/src/kernel.rs:4328-4351`；`ShmCtx`，`:4353-4385` |
| 迁移语义 | `IPC_PRIVATE` 创建唯一 segment，不复用 ordinary key；existing key 的 size/页数语义必须明确，不应无条件返回过小 guard。 |
| 修改前 rCore 表达 | `new_shared_guard` 对任何 key 都查 `KEY2SHM`；existing key 直接返回 guard，不检查 size；`sys_shmget` 不做 size 前置校验。 |
| 接口处理 | 保留 `ShmIdentifier` 和 `ShmProc` 表达；`key == 0` 绕开 ordinary `KEY2SHM` lookup；existing key 请求更大 size 时扩展 `SharedGuard.size`，与 `kernel.rs` 的扩展语义对齐。 |
| 最小修改范围 | `ShmIdentifier::new_shared_guard` 和 `sys_shmget` 的 key/size 前置处理；必要时只增加私有 helper。 |
| 不应修改 | `SharedGuard<GlobalFrameAlloc>` public shape、`sys_shmat`/`sys_shmdt` ABI。 |
| 验收点 | key 0 每次创建唯一 segment；existing key 且 size 不满足时扩展 shared guard size；合法 existing key 返回原 guard；size 0 不产生无意义 segment。 |
| 已落地结果 | `sys_shmget` 拒绝 `size == 0`；`ShmIdentifier::new_shared_guard` 对 `key == 0` 直接创建独立 `SharedGuard`，不进入 `KEY2SHM`；existing key 命中时如果旧 `SharedGuard.size < memsize`，更新 size 后返回原 guard。 |
| 验收结果 | `rustfmt --edition 2018 kernel/src/ipc/shared_mem.rs kernel/src/syscall/ipc.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止。 |
| 状态 | `MIGRATED` |

## 接口边界

| `kernel.rs` 结构 | rCore 落点 | 处理方式 |
| --- | --- | --- |
| `SemArr` | `SemArray` | 保留 rCore metadata/permission ABI；迁移 zero-size 和 existing-key count 检查。 |
| `SemCtx` | per-process `SemProc` | 保留 `Process.semaphores`；迁移 undo cleanup 和 drop replay。 |
| `ShmTag`、`ShmCtx` | `ShmIdentifier`、`ShmProc` | 保留 rCore `SharedGuard` backing；迁移 private key 和 size 语义。 |
| `shm_get_or_create` | `ShmIdentifier::new_shared_guard` | 不复制模拟 `Vec<usize>` backing；只迁移 key/size 生命周期规则。 |

## 批次内顺序

1. 先处理 semaphore create/get，因为它只影响全局 semaphore array 创建路径。
2. 已处理 semaphore undo/remove，因为它依赖 per-process state 和 IPC_RMID。
3. 已处理 shared memory key/size，因为它联动 `SharedGuard`、`sys_shmget`、`sys_shmat` 和 B02 shared mapping。

## 风险和验收

| 风险 | 验收方式 |
| --- | --- |
| existing-key count 检查影响合法重用 | 只拒绝请求数量超过已有 `nsems` 的情况；相等或更小保持兼容。 |
| undo replay 循环可能长时间执行 | undo magnitude 来自 syscall 输入；源码修改时可考虑 bounds，但必须保持完整 replay 语义。 |
| shared memory size 处理策略不明确 | 源码修改前选择“size 不足返回错误”或“明确扩展”，并记录在批次执行日志。 |

## 批准前报告摘要

若进入源码修改，需要先向用户报告：

- 文件/行：`kernel/src/ipc/semary.rs:95`、`:117`；`kernel/src/syscall/ipc.rs:15`、`:56`、`:113`；`kernel/src/ipc/mod.rs:47`、`:62`、`:80`；`kernel/src/ipc/shared_mem.rs:27`。
- 现象/语义差距：semaphore zero-size/existing-key count、undo cleanup/replay、shm private key/size 语义与 `kernel.rs` 对齐不完整。
- 根因/当前表达：恢复上游基线保持原接口，但 existing key 和 undo/drop 边界处理较弱。
- 预期行为：非法 create/get 返回错误；remove 清理 stale undo；drop 完整 replay undo；shm key/size 生命周期明确。
- 最小修改：只改 IPC create/get/remove/drop 和 shm get/create 路径，保持 rCore public API。
