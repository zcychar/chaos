# rCore 迁移记录总索引

本文是 `kernel.rs` 到恢复后 rCore 体系的迁移记录入口。记录对象包括本轮从上游新增的每一个文件、文件夹，以及每个子模块下需要从 `kernel/src/kernel.rs` 对齐迁移的功能点。

## 基线状态

- -os/rCore`
- 固定 commit：`66cb4181ec6d3336d507c7c1ff100127f56fcc0a`
- 恢复范围：
  - `crate/memory/`
  - `kernel/src/memory.rs`
  - `kernel/src/trap.
## 记录层级

| 层级 | 记录对象 | 记录位置 | 说明 |
| --- | --- | --- | --- |
| L0 | 恢复批次 | `restored-tree.md` | 记录上游 commit、恢复范围、校验证据。 |
| L1 | 顶层新增路径 | `restored-tree.md` | 例如 `crate/memory/`、`kernel/src/fs/`。 |
| L2 | 子目录 | `restored-tree.md` 和对应模块文档 | 例如 `crate/memory/src/memory_set/handler/`、`kernel/src/fs/devfs/`。 |
| L3 | 单个文件 | `restored-tree.md` 和对应模块文档 | 每个新增文件都有文件级职责、迁移归属和状态。 |
| L4 | 子模块-功能 | `function-index.md` 和 `modules/*.md` | 记录主要结构、函数、trait、syscall/handler 落点及迁移语义。 |

## 状态定义

| 状态 | 含义 |
| --- | --- |
| `BASELINE_RESTORED` | 文件已经按上游固定 commit 恢复，尚未修改。 |
| `MIGRATION_PENDING` | 已识别 `kernel.rs` 对应语义，等待按批次迁移。 |
| `NO_DIRECT_PORT` | `kernel.rs` 中存在模拟结构，但真实 rCore 不需要同名结构，只迁移语义或不迁移。 |
| `NEEDS_APPROVAL` | 已定位到需要修改 rCore 源码的迁移项，必须先报告并等待批准。 |
| `PARTIAL_MIGRATED` | 对应语义已有部分源码落地，但仍有后续运行边界或批次外语义需要继续收口。 |
| `RUNTIME_BOUNDARY` | 构建已通过，但运行停在明确的下一处系统行为差距，需要新的行级修复报告。 |
| `MIGRATED` | 对应语义已经迁移并通过指定验收。 |

## 文档索引

| 文档 | 覆盖范围 |
| --- | --- |
| [final-delivery.md](final-delivery.md) | 本轮中文总交付说明，汇总恢复范围、记录层级、批次入口、验证结果和下一步批准点。 |
| [completion-audit.md](completion-audit.md) | 用户目标逐项完成度、当前证据和未完成项审计。 |
| [approval-queue.md](approval-queue.md) | 待批准源码迁移队列、已执行 B01 状态和建议批准顺序。 |
| [execution-log.md](execution-log.md) | 分步骤执行日志、校验证据和下一批待批准迁移项。 |
| [path-traceability.md](path-traceability.md) | 每个新增目录/文件到 L0-L4 记录、模块文档和迁移批次的逐路径追踪矩阵。 |
| [symbol-coverage.md](symbol-coverage.md) | 恢复 Rust 文件的类型、trait、impl、函数、常量入口提取结果，以及对应 L4 记录落点。 |
| [migration-batches.md](migration-batches.md) | 可执行迁移批次，含文件/行、迁移语义、当前表达、预期结果和最小修改范围。 |
| [batches/B01-sync-futex.md](batches/B01-sync-futex.md) | B01 sync + futex 的函数级详细迁移记录和源码修改前报告材料。 |
| [batches/B02-memory-trap.md](batches/B02-memory-trap.md) | B02 memory + trap 的函数级/子模块级详细迁移记录和源码修改前报告材料。 |
| [batches/B03-fs.md](batches/B03-fs.md) | B03 fs 的 file、mmap、epoll、pipe、TTY/channel 详细迁移记录和源码修改前报告材料；append/seek、mmap range 子项已完成。 |
| [batches/B04-ipc.md](batches/B04-ipc.md) | B04 IPC 的 semaphore、undo/remove、shared memory key/size 详细迁移记录和源码修改前报告材料。 |
| [batches/B05-process.md](batches/B05-process.md) | B05 process 的 stack、process brk、fork/wait、fd lifecycle、ELF bounds 详细迁移记录和源码修改前报告材料。 |
| [batches/B06-no-direct.md](batches/B06-no-direct.md) | B06 deferred/no-direct 路径的保留原因、重新评估条件和状态记录。 |
| [restored-tree.md](restored-tree.md) | 所有新增目录和文件的树状迁移记录。 |
| [function-index.md](function-index.md) | 每个新增 Rust 文件的 public API、关键 impl 和子模块-功能迁移项。 |
| [modules/crate-memory.md](modules/crate-memory.md) | `crate/memory/`，对应 VM、页表、COW、handler、swap 辅助。 |
| [modules/kernel-memory-trap.md](modules/kernel-memory-trap.md) | `kernel/src/memory.rs`、`kernel/src/trap.rs`。 |
| [modules/kernel-fs.md](modules/kernel-fs.md) | `kernel/src/fs/` 和 `kernel/src/fs/devfs/`。 |
| [modules/kernel-ipc.md](modules/kernel-ipc.md) | `kernel/src/ipc/`。 |
| [modules/kernel-process.md](modules/kernel-process.md) | `kernel/src/process/`。 |
| [modules/kernel-sync.md](modules/kernel-sync.md) | `kernel/src/sync/`。 |
| [modules/rust-toolchain.md](modules/rust-toolchain.md) | `rust-toolchain`。 |

## 迁移批次顺序

建议迁移顺序仍按高风险共享状态优先：

1. B01 已完成：`kernel/src/sync/condvar.rs` 和 `kernel/src/process/futex.rs`。
2. B02 已部分完成：`crate/memory/`、`kernel/src/memory.rs`、`kernel/src/trap.rs`、`kernel/src/process/{abi,thread}.rs`，以及已批准的 `kernel/src/arch/riscv/paging.rs` 修正。
3. B02 最新运行验证：已越过 `0x100e8` instruction page fault 和 `sepc=0x0` 跳转 fault，QEMU 日志进入 busybox shell `/ #`。
4. `kernel/src/fs/`，其中 append/seek、mmap range 子项已完成，后续重点是 pipe、epoll、TTY。
5. `kernel/src/ipc/`。
6. `kernel/src/process/` 和 syscall process/fd 生命周期；其中 process brk 子项已完成。
7. `kernel/src/signal/*` 和架构 tra