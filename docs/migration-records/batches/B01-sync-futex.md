# B01 sync + futex 详细迁移记录

本文是 B01 的批次级执行记录。它把 [migration-batches.md](../migration-batches.md) 中的 B01 拆成函数级迁移单元，用于在源码修改前明确：目标文件/行、`kernel.rs` 语义来源、当前 rCore 表达、接口处理方式、最小修改范围和验收点。

当前状态：已按用户批准执行 B01 源码迁移。本文同时保留源码修改前报告材料和修改后的执行证据。

```text
批准执行 B01 sync + futex 源码迁移。
```

## 批次定位

| 项目 | 内容 |
| --- | --- |
| 批次 | B01 sync + futex |
| 恢复模块 | `kernel/src/sync/condvar.rs`、`kernel/src/process/futex.rs` |
| `kernel.rs` 来源行段 | `429-635` 的 `SyncQueue`，`813-853` 的 `FutexTable` |
| 上游基线 | 恢复源码与 rCore commit `66cb4181ec6d3336d507c7c1ff100127f56fcc0a` hash 一致 |
| 接口原则 | 保留 rCore 的 `Condvar`、`Future`、`Waker`、`Arc<Thread>` 和 `Futex` API，不复制 `std::thread::park/unpark` 结构 |

## B01.1 `Condvar::wait_events`

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/sync/condvar.rs:55` |
| rCore 函数 | `Condvar::wait_events<T>(condvars: &[&Condvar], condition: impl FnMut() -> Option<T>) -> T` |
| `kernel.rs` 来源 | `SyncQueue::wait_events`，`kernel/src/kernel.rs:566-595` |
| 迁移语义 | 当前线程注册到多个等待队列；条件满足返回前，必须从所有队列清理当前 waiter；每次重新等待不能重复入队；不能 lost wakeup。 |
| 当前 rCore 表达 | `tid` 固定为 `0`；线程 token 入队逻辑是注释；循环中只 lock/unlock 队列；返回前没有真实 waiter 清理。 |
| 接口处理 | 不引入 `std::thread::Thread`；应使用 rCore 当前线程对象或等价 waiter token，并保持 `wait_events` public 签名。 |
| 最小修改范围 | `Condvar::wait_events` 内部 waiter token 获取、去重入队、返回/重试前清理；必要时增加私有 helper。 |
| 不应修改 | `Condvar` public API、`MutexGuard` 类型、调用方接口。 |
| 验收点 | 多队列等待返回后，所有参与队列中没有当前 waiter；重复 wait 不产生重复节点；`condition()` 已满足时不阻塞。 |
| 状态 | `MIGRATED` |

执行记录：

- `kernel/src/sync/condvar.rs` 新增 `current_thread()` token 获取，使用 `Thread.tid` 对等待队列去重。
- `Condvar::wait_events` 在返回前遍历所有参与队列并移除当前 waiter。
- 若条件尚未满足，当前 waiter 会按 tid 去重注册到每个 condvar 队列，避免重复节点。

## B01.2 `Condvar::wait_timeout`

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/sync/condvar.rs:109` |
| rCore 函数 | `Condvar::wait_timeout<'a, T, S>(&self, guard: MutexGuard<'a, T, S>, timeout: TimeSpec) -> Option<MutexGuard<'a, T, S>>` |
| `kernel.rs` 来源 | `SyncQueue::wait_timeout`，`kernel/src/kernel.rs:605-618` |
| 迁移语义 | timeout 后从 wait queue 移除当前 waiter；返回值区分真实唤醒和超时；等待时释放原 guard，返回时重新获取 mutex guard。 |
| 当前 rCore 表达 | token push/remove 逻辑是注释；`sleep(timeout)` 是注释；timeout 只由 `uptime_msec` 差值判断；没有清理等待队列。 |
| 接口处理 | 保留返回 `Option<MutexGuard<...>>`；`None` 表示 timeout，`Some` 表示被唤醒后重新持有锁。 |
| 最小修改范围 | `wait_timeout` 内部 waiter token 注册、timeout 后 retain/remove、真实 wake 后重新 lock；必要时抽取私有清理 helper。 |
| 不应修改 | `TimeSpec` ABI、`MutexSupport` trait、`MutexGuard` public 行为。 |
| 验收点 | timeout 返回 `None` 且队列无 stale waiter；notify 后返回 `Some(guard)`；后续 `notify_*` 不再处理已超时 waiter。 |
| 状态 | `MIGRATED` |

执行记录：

- `Condvar::wait_timeout` 在释放原 guard 前注册当前 waiter，释放 guard 后等待 notify 或 timeout。
- notify 移除 waiter 后返回 `Some(mutex.lock())`；timeout 后调用私有清理 helper 并返回 `None`。
- timeout 计算使用 `saturating_sub` 和 `usize::MAX` 上限截断，避免时间差 underflow 或 `as_millis()` 过大导致的 wrap。

## B01.3 `Futex::wake`

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/process/futex.rs:36` |
| rCore 函数 | `Futex::wake(&self, wake_count: usize) -> usize` |
| `kernel.rs` 来源 | `FutexTable::ftx_wake`，`kernel/src/kernel.rs:835-853` |
| 迁移语义 | `wake_count == 0` 必须返回 `0`；返回值等于实际移除并唤醒的有效 waiter 数；不能把已超时 waiter 计入真实唤醒。 |
| 当前 rCore 表达 | `for i in 0..wake_count` 从队头 pop waiter；队列不足时返回 `i`；没有主动跳过 timeout 后滞留的 waiter。 |
| 接口处理 | 保留 `wake_count: usize -> usize`；waiter 有效性应由 `Waiter` 状态或队列清理保证。 |
| 最小修改范围 | `Futex::wake` 的 pop/woken 计数逻辑，以及与 B01.4 共享的 waiter 有效状态判断。 |
| 不应修改 | Futex syscall 调用面、`Futex::wait` public 返回类型。 |
| 验收点 | `wake(0) == 0`；唤醒数量不超过请求数；返回值只统计实际仍等待的 waiter；队列为空时返回已唤醒数量。 |
| 状态 | `MIGRATED` |

执行记录：

- `Futex::wake(0)` 显式返回 `0`。
- `wake` 循环只统计未被标记为 `woken` 的有效 waiter。
- 从队列取出的 waker 在释放 futex 队列锁后再执行 `wake()`，避免持锁唤醒带来的重入风险。

## B01.4 `FutexFuture::poll`

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/process/futex.rs:62` |
| rCore 函数 | `Futex::wait` 内部 `FutexFuture::poll` |
| `kernel.rs` 来源 | `FutexTable::ftx_wait` 和 timeout cleanup audit，`kernel/src/kernel.rs:824-832` |
| 迁移语义 | futex wait 超时后，当前 waiter 必须从 `FutexInner.waiters` 中移除；后续 `wake` 不能再取出这个 waiter。 |
| 当前 rCore 表达 | 超时分支只设置 `inner.woken = true` 并返回 `ETIMEDOUT`；未从 `inner.futex.inner.waiters` 删除当前 waiter。 |
| 接口处理 | 保留内部 future 结构；可以通过 `Arc::ptr_eq` 或等价 token 判定当前 waiter；清理必须避免锁顺序死锁。 |
| 最小修改范围 | `FutexFuture::poll` 的 timeout 分支；必要时增加 `FutexInner` 私有清理 helper。 |
| 不应修改 | `SysError::ETIMEDOUT`、timer waker 注册方式、`Futex::wait` public 签名。 |
| 验收点 | timeout 返回 `Err(SysError::ETIMEDOUT)`；队列不保留该 waiter；timeout 后调用 `wake` 不重复唤醒该 waiter。 |
| 状态 | `MIGRATED` |

执行记录：

- `FutexInner` 新增私有 `remove_waiter` 和 `contains_waiter` helper。
- `FutexFuture` 持有 `Arc<Futex>`，poll 时按 futex 队列锁再 waiter 锁的顺序处理，避免和 `wake` 形成反向锁顺序。
- timeout 分支标记 waiter、清空 waker，并从 `FutexInner.waiters` 移除当前 waiter。

## 接口边界

| `kernel.rs` 结构 | rCore 落点 | 处理方式 |
| --- | --- | --- |
| `std::thread::Thread` | `Arc<Thread>`、`Waker` 或私有 waiter token | 只迁移 waiter 生命周期和 wake 计数语义，不复制 `std` 线程模型。 |
| `thread::park/unpark` | executor future、`Waker::wake`、rCore thread wake path | 保留 rCore async/sleep 机制。 |
| `Mutex<VecDeque<thread::Thread>>` | `SpinNoIrqLock<VecDeque<Arc<Thread>>>` 或 futex waiter queue | 保留 rCore 锁类型，补齐入队/清理不变量。 |
| `SyncQueue::sig` 预记录信号 | `Condvar` wait/notify path | B01 先记录为 no lost wakeup 目标；是否新增信号计数需要源码批准时再按最小范围决定。 |

## 批次内顺序

1. 先处理 `FutexFuture::poll` timeout cleanup，使后续 `Futex::wake` 能依赖队列中只保留有效 waiter。
2. 再处理 `Futex::wake` 的真实唤醒计数。
3. 再处理 `Condvar::wait_timeout` 的注册和 timeout cleanup。
4. 最后处理 `Condvar::wait_events` 的多队列去重注册和返回前清理。

## 风险和验收

| 风险 | 验收方式 |
| --- | --- |
| 清理 waiter 时锁顺序错误导致死锁 | 保持单队列短临界区，不在持有 futex/condvar 队列锁时调用外部可重入逻辑。 |
| waiter token 身份不稳定 | 使用稳定的 `Arc` 身份或线程 id；多队列清理使用同一 token。 |
| timeout 后 stale waiter 被 wake 计数 | timeout 分支立即清理；wake 只统计实际仍有效 waiter。 |
| 改动影响 epoll callback | `Condvar` notify path 仍通过现有 `epoll_callback`，B01 不扩展 epoll 语义。 |

## 本批次源码差异

| 文件 | 迁移内容 | 与上游 hash 关系 |
| --- | --- | --- |
| `kernel/src/process/futex.rs` | futex waiter helper、timeout cleanup、wake count 对齐。 | 相对上游产生有意差异。 |
| `kernel/src/sync/condvar.rs` | wait queue token 注册、去重、返回/timeout 清理。 | 相对上游产生有意差异。 |

## 本批次验证记录

| 验证项 | 命令/证据 | 结果 |
| --- | --- | --- |
| 格式化 | `rustfmt kernel/src/process/futex.rs kernel/src/sync/condvar.rs` | 已通过。第一次运行因 rustup 临时目录受沙箱只读限制失败，提升权限后安装指定 nightly 并完成格式化。 |
| 差异核对 | `diff -u /tmp/rcore-upstream/kernel/src/process/futex.rs kernel/src/process/futex.rs` | 仅 B01 futex 预期差异。 |
| 差异核对 | `diff -u /tmp/rcore-upstream/kernel/src/sync/condvar.rs kernel/src/sync/condvar.rs` | 仅 B01 condvar 预期差异。 |
| 内核构建 | `make build ARCH=riscv64 objcopy=/usr/bin/llvm-objcopy` | 已通过。Rust 编译仅有既有 deprecated/unreachable pattern warning；`kernel.img` 已生成。 |
| 运行冒烟 | 手动 QEMU 20 秒启动，去掉本机 QEMU 不支持的 `-cpu rv64,x-h=true`，保留 kernel/user image/virtio-blk。 | 已进入 OpenSBI 并跳转 rCore；随后在 B02 memory 初始化路径触发 stack trace，不归入 B01。 |

运行冒烟的符号化结果：

| PC | 符号 |
| --- | --- |
| `0xFFFFFFFFC0242B66` | `rcore::memory::handle_page_fault_ext` |
| `0xFFFFFFFFC024EA98` | `rcore::process::thread::Thread::new_user_vm` |
| `0xFFFFFFFFC024F00A` | `rcore::process::thread::Thread::new_user` |
| `0xFFFFFFFFC0259288` | `rust_main` |

结论：B01 的 Rust 编译和镜像生成已通过；完整运行的下一处失败属于 B02/B05 初始化内存路径，详见 [B02 memory + trap](B02-memory-trap.md)。

## 批准前报告摘要

本批次源码修改前已报告并获得用户要求“根据当前文档指导开始分步骤完成”的批准。报告摘要：

- 文件/行：`kernel/src/sync/condvar.rs:55`、`:109`，`kernel/src/process/futex.rs:36`、`:62`。
- 现象/语义差距：waiter 注册和清理逻辑仍是注释/不完整；futex timeout waiter 会留在队列。
- 根因/当前表达：恢复的 rCore 基线保留了 async/future 接口，但部分 wait queue token 操作未落地。
- 预期行为：无 lost wakeup；timeout 清理；wake 计数只包含实际唤醒；public API 不变。
- 最小修改：只改 `Condvar` wait path 和 `Futex` wait/wake path，必要时增加私有 helper。
