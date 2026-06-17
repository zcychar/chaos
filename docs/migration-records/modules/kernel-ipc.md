# `kernel/src/ipc` 迁移记录

## 模块定位

`kernel/src/ipc` 是恢复后 rCore 的 SysV semaphore 和 shared memory 进程状态模块。它对应 `kernel.rs` 中 `IpcPerm`、`SemArr`、`SemCtx`、`ShmTag`、`ShmCtx` 的语义。

| 对应 `kernel.rs` 行段 | 迁移主题 |
| --- | --- |
| `4158-4386` | IPC 权限、semaphore array/context、shared memory context。 |

## 文件-功能记录

| 文件 | 主要结构/函数 | `kernel.rs` 对齐语义 | 迁移记录 |
| --- | --- | --- | --- |
| `mod.rs` | `SemProc`、`ShmProc`、`SemId`、`ShmId`、undo add/drop、shm attach table。 | `SemCtx`、`ShmCtx`、sem undo、detach。 | 已迁移 remove 后 stale undo cleanup、drop replay 完整正向 undo magnitude。 |
| `semary.rs` | `IpcPerm`、`SemidDs`、`SemArray`、global semaphore arrays。 | `SemArr::get_or_create`、permission、existing key size/count。 | 已迁移拒绝 `nsems == 0`，existing key 请求更大 count 时失败。 |
| `shared_mem.rs` | `ShmIdentifier`、`SharedGuard<GlobalFrameAlloc>`。 | `ShmTag` 和 shared page attach metadata。 | 已迁移 private key 唯一性和 existing key size 扩展语义，实际 syscall 落点在 `syscall/ipc.rs`。 |

## 子模块-功能迁移记录

| 功能项 | rCore 落点 | `kernel.rs` 来源 | 当前迁移状态 |
| --- | --- | --- | --- |
| semaphore create zero-size | `semary.rs::SemArray` 创建路径、`syscall/ipc.rs` | `SemArr::get_or_create` | `MIGRATED` |
| existing-key nsems check | `semary.rs` global lookup、`syscall/ipc.rs` | `SemArr::get_or_create` | `MIGRATED` |
| sem undo add/drop | `mod.rs::SemProc::{add_undo,drop}` | `SemCtx::drop` | `MIGRATED` |
| remove stale undo | `mod.rs::SemProc::remove` | `SemCtx::remove` | `MIGRATED` |
| shared memory private key | `shared_mem.rs`、`syscall/ipc.rs` | `shm_get_or_create` | `MIGRATED` |
| existing shm size | `shared_mem.rs`、`syscall/ipc.rs` | `shm_get_or_create` | `MIGRATED` |

## 待批准迁移候选

| 优先级 | 位置 | 迁移语义 | 最小范围 |
| --- | --- | --- | --- |
| 已完成 | `kernel/src/ipc/semary.rs` 和 `syscall/ipc.rs` | 创建 semaphore array 时拒绝 `nsems == 0`；existing key 不满足请求 count 时返回错误。 | semaphore create/get path。 |
| 已完成 | `kernel/src/ipc/mod.rs::SemProc` | remove 清理该 id 的 undo，drop replay 完整正向 undo 值。 | `SemProc::remove`、`Drop for SemProc`。 |
| 已完成 | `kernel/src/ipc/shared_mem.rs` 和 `syscall/ipc.rs` | `IPC_PRIVATE` 唯一 segment，existing key size 扩展。 | shm get/create path。 |
