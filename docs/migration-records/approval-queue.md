# 源码迁移批准队列

本文是待批准源码迁移项的简表。详细执行单见 [migration-batches.md](migration-batches.md)。B02 已不再是“整批待批准”状态；它已部分迁移，并已通过 QEMU 启动到 busybox shell。B05 process 核心子项已完成。后续必须围绕新的具体运行边界或具体批次重新做行级报告。

## 建议批准顺序

| 顺序 | 批次 | 为什么先做 | 主要目标文件 |
| --- | --- | --- | --- |
| 1 | 当前无固定待批准批次 | B04 和 B05 核心子项已完成；不再用旧队列驱动源码修改。 | 无 |
| 2 | 后续真实运行边界 | B05 process 核心子项已完成；若 QEMU 或源码审计暴露新的 process group、signal、resource、scheduler 边界，再按行级报告推进。 | 由新边界确定 |
| 3 | B02 后续边界 | B02 已部分迁移且能启动到 shell；后续只针对新的 memory/trap 故障或明确子模块语义差距报告。 | 由新边界确定 |

## 批准格式

已执行批次：

```text
B01 sync + futex：已迁移。
```

后续若继续 B02，建议先要求报告而不是直接批准：

```text
请先基于新的运行现象或明确批次报告文件/行、根因、预期行为和最小修复。
```

后续批次仍可指定子项：

```text
批准执行 B03.1 append write and negative seek 源码迁移。
```

## 当前队列

| 批次 | 子项 | 状态 |
| --- | --- | --- |
| B01 | B01.1 `Condvar::wait_events` | `MIGRATED` |
| B01 | B01.2 `Condvar::wait_timeout` | `MIGRATED` |
| B01 | B01.3 `Futex::wake` | `MIGRATED` |
| B01 | B01.4 `FutexFuture::poll` | `MIGRATED` |
| B02 | B02.1 COW refcount | `PARTIAL_MIGRATED` |
| B02 | B02.2 MemorySet range arithmetic | `PARTIAL_MIGRATED` |
| B02 | B02.3 file-backed mapping offset | `PARTIAL_MIGRATED` |
| B02 | B02.4 kernel frame/user access | `PARTIAL_MIGRATED` |
| B02 | B02.5 tick conversion | `MIGRATED` |
| B02 | B02.6 init-time user stack VM write | `PARTIAL_MIGRATED` |
| B02 | B02.7 RISC-V PTE update / `sfence.vma` | `MIGRATED` |
| B02 | `0x100e8` instruction page fault 和 `sepc=0x0` file-backed LOAD fault | `MIGRATED` |
| B03 | B03.1 append write and negative seek | `MIGRATED` |
| B03 | B03.2 mmap file range | `MIGRATED` |
| B03 | B03.3 epoll state | `MIGRATED` |
| B03 | B03.4 pipe endpoint lifecycle | `MIGRATED` |
| B03 | B03.5 TTY/channel wakeup | `MIGRATED` |
| B04 | B04.1 semaphore create/get | `MIGRATED` |
| B04 | B04.2 semaphore undo/remove | `MIGRATED` |
| B04 | B04.3 shared memory key/size | `MIGRATED` |
| B05 | B05.1 user stack init | `MIGRATED` |
| B05 | B05.2 fork/wait parent-child | `MIGRATED` |
| B05 | B05.3 fd lifecycle and cloexec | `MIGRATED` |
| B05 | B05.4 ELF bounds | `MIGRATED` |
| B05 | B05.5 process brk / heap boundary | `MIGRATED` |
