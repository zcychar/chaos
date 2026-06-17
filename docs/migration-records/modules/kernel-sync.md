# `kernel/src/sync` 迁移记录

## 模块定位

`kernel/src/sync` 是恢复后 rCore 的锁、条件变量、事件总线和 semaphore 层。它对应 `kernel.rs` 中 `KernLock`、`SyncQueue`、`EvBus`、`Sema`，并与 `process/futex.rs` 共同承担阻塞/唤醒语义。

| 对应 `kernel.rs` 行段 | 迁移主题 |
| --- | --- |
| `429-904` | 同步队列、event bus、semaphore、futex table。 |
| `3216-4154` | 部分 cache/disk 路径的锁状态恢复语义。 |

## 文件-功能记录

| 文件 | 主要结构/函数 | `kernel.rs` 对齐语义 | 迁移记录 |
| --- | --- | --- | --- |
| `mod.rs` | `SpinLock`、`SpinNoIrqLock`、`SleepLock`、`Condvar`、`EventBus`、`Semaphore` re-export。 | sync module facade。 | 保持上游 re-export。 |
| `mutex.rs` | `Mutex`、`MutexGuard`、`MutexSupport`、`Spin`、`SpinNoIrq`、`FlagsGuard`。 | `KernLock`、sleeping while synchronized、IRQ state。 | 重点迁移 guard drop 恢复状态、no-irq 区域状态一致。 |
| `condvar.rs` | `Condvar::{wait_events,wait,wait_timeout,notify_one,notify_all,notify_n}`、epoll registration。 | `SyncQueue` wait/wake/timeout/stale waiter cleanup。 | 重点迁移 wait 前后注册清理、timeout 后清理、`notify_n(0)` 返回 0。 |
| `event_bus.rs` | `EventBus::{subscribe,set,clear,subscribe_waker}`、`wait_for_event` future。 | `EvBus` event delivery/stale callback。 | 已为 B03.5 增加按 waker/mask 去重的订阅入口；旧 `subscribe` 调用面保持兼容。 |
| `semaphore.rs` | `Semaphore::{new,acquire,release,try_acquire}`、`SemaphoreGuard`。 | `Sema` acquire/release/wakeup/guard。 | 重点迁移 release 唤醒数量、guard drop 不重复 release。 |

## 子模块-功能迁移记录

| 功能项 | rCore 落点 | `kernel.rs` 来源 | 当前迁移状态 |
| --- | --- | --- | --- |
| stale waiter cleanup | `condvar.rs::wait_events` | `SyncQueue::wait_events` | `MIGRATED` |
| timeout waiter cleanup | `condvar.rs::wait_timeout` | `SyncQueue::wait_timeout` | `MIGRATED` |
| notify exact count | `condvar.rs::notify_n` | `SyncQueue::notify_n`、`FutexTable::wake` | `MIGRATION_PENDING` |
| epoll wake registration | `condvar.rs::{register_epoll_list,unregister_epoll_list}` | `EpInst` ready/control state | `MIGRATION_PENDING` |
| event stale handler | `event_bus.rs::subscribe_waker` and future polling | `EvBus` | `PARTIAL_MIGRATED` for B03.5 |
| semaphore guard release | `semaphore.rs::SemaphoreGuard::drop` | `Sema` guard | `MIGRATION_PENDING` |
| no-irq lock restore | `mutex.rs::FlagsGuard` | `TrapCtl` IRQ restoration, `KernLock` state | `MIGRATION_PENDING` |

## 待批准迁移候选

| 优先级 | 位置 | 迁移语义 | 最小范围 |
| --- | --- | --- | --- |
| 已完成子项 | `kernel/src/sync/event_bus.rs` | TTY/wait future 使用 waker/mask 去重，避免 pending poll 重复注册。 | `EventBus::subscribe_waker`、`wait_for_event`。 |
