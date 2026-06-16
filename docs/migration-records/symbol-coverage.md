# 子模块-功能符号覆盖记录

本文为 L4 子模块-功能记录提供可复核的符号级证据。它不替代 [function-index.md](function-index.md)，而是说明每个恢复 Rust 文件中可识别的类型、trait、impl、函数、常量入口已经被归入对应的迁移记录。

## 提取口径

使用下列命令从恢复源码中提取符号入口：

```bash
rg -n '^\s*(pub\s+)?(struct|enum|trait|type|static|const)\s+[A-Za-z_][A-Za-z0-9_]*|^\s*(pub\s+)?fn\s+[A-Za-z_][A-Za-z0-9_]*|^\s*impl(\s*<[^>]+>)?\s+[^\{]+\{' \
  crate/memory/src kernel/src/memory.rs kernel/src/trap.rs \
  kernel/src/fs kernel/src/ipc kernel/src/process kernel/src/sync
```

该命令用于验证迁移记录覆盖，不作为 Rust 语法解析器。宏生成项、`bitflags!` 内部常量、模块导出语句和纯 `pub mod` 文件由 [function-index.md](function-index.md) 与各模块文档单独记录。

## 覆盖摘要

| 区域 | Rust 文件数 | 有符号入口文件数 | 迁移记录位置 |
| --- | ---: | ---: | --- |
| `crate/memory/src/` | 17 | 17 | [function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) |
| `kernel/src/memory.rs`、`kernel/src/trap.rs` | 2 | 2 | [function-index.md](function-index.md)、[modules/kernel-memory-trap.md](modules/kernel-memory-trap.md) |
| `kernel/src/fs/` | 15 | 14 | [function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) |
| `kernel/src/ipc/` | 3 | 3 | [function-index.md](function-index.md)、[modules/kernel-ipc.md](modules/kernel-ipc.md) |
| `kernel/src/process/` | 6 | 6 | [function-index.md](function-index.md)、[modules/kernel-process.md](modules/kernel-process.md) |
| `kernel/src/sync/` | 5 | 4 | [function-index.md](function-index.md)、[modules/kernel-sync.md](modules/kernel-sync.md) |

`kernel/src/fs/devfs/mod.rs` 和 `kernel/src/sync/mod.rs` 是模块边界/导出文件，符号提取命令没有命中函数或类型；它们已经在 [restored-tree.md](restored-tree.md)、[path-traceability.md](path-traceability.md) 和 [function-index.md](function-index.md) 中作为模块边界记录。

## 每文件符号入口数量

| 文件 | 符号入口数 | 子模块-功能记录 |
| --- | ---: | --- |
| `crate/memory/src/addr.rs` | 17 | [function-index.md](function-index.md) `crate/memory/src/addr.rs` |
| `crate/memory/src/cow.rs` | 26 | [function-index.md](function-index.md) `crate/memory/src/cow.rs` |
| `crate/memory/src/lib.rs` | 2 | [function-index.md](function-index.md) `crate/memory/src/lib.rs` |
| `crate/memory/src/memory_set/handler/byframe.rs` | 9 | [function-index.md](function-index.md) `handler/byframe.rs` |
| `crate/memory/src/memory_set/handler/delay.rs` | 9 | [function-index.md](function-index.md) `handler/delay.rs` |
| `crate/memory/src/memory_set/handler/file.rs` | 13 | [function-index.md](function-index.md) `handler/file.rs` |
| `crate/memory/src/memory_set/handler/linear.rs` | 9 | [function-index.md](function-index.md) `handler/linear.rs` |
| `crate/memory/src/memory_set/handler/mod.rs` | 20 | [function-index.md](function-index.md) `handler/mod.rs` |
| `crate/memory/src/memory_set/handler/shared.rs` | 19 | [function-index.md](function-index.md) `handler/shared.rs` |
| `crate/memory/src/memory_set/mod.rs` | 37 | [function-index.md](function-index.md) `memory_set/mod.rs` |
| `crate/memory/src/no_mmu.rs` | 16 | [function-index.md](function-index.md) `no_mmu.rs` |
| `crate/memory/src/paging/mock_page_table.rs` | 46 | [function-index.md](function-index.md) `paging/mock_page_table.rs` |
| `crate/memory/src/paging/mod.rs` | 39 | [function-index.md](function-index.md) `paging/mod.rs` |
| `crate/memory/src/swap/enhanced_clock.rs` | 10 | [function-index.md](function-index.md) `swap/*` |
| `crate/memory/src/swap/fifo.rs` | 7 | [function-index.md](function-index.md) `swap/*` |
| `crate/memory/src/swap/mock_swapper.rs` | 12 | [function-index.md](function-index.md) `swap/*` |
| `crate/memory/src/swap/mod.rs` | 28 | [function-index.md](function-index.md) `swap/*` |
| `kernel/src/memory.rs` | 33 | [function-index.md](function-index.md) `kernel/src/memory.rs` |
| `kernel/src/trap.rs` | 9 | [function-index.md](function-index.md) `kernel/src/trap.rs` |
| `kernel/src/fs/devfs/fbdev.rs` | 21 | [function-index.md](function-index.md) `fs/devfs/fbdev.rs` |
| `kernel/src/fs/devfs/mod.rs` | 0 | [function-index.md](function-index.md) `fs/devfs/mod.rs` |
| `kernel/src/fs/devfs/random.rs` | 10 | [function-index.md](function-index.md) `fs/devfs/random.rs` |
| `kernel/src/fs/devfs/serial.rs` | 10 | [function-index.md](function-index.md) `fs/devfs/serial.rs` |
| `kernel/src/fs/devfs/shm.rs` | 7 | [function-index.md](function-index.md) `fs/devfs/shm.rs` |
| `kernel/src/fs/devfs/tty.rs` | 19 | [function-index.md](function-index.md) `fs/devfs/tty.rs` |
| `kernel/src/fs/device.rs` | 6 | [function-index.md](function-index.md) `fs/device.rs` |
| `kernel/src/fs/epoll.rs` | 33 | [function-index.md](function-index.md) `fs/epoll.rs` |
| `kernel/src/fs/fcntl.rs` | 15 | [function-index.md](function-index.md) `fs/fcntl.rs` |
| `kernel/src/fs/file.rs` | 27 | [function-index.md](function-index.md) `fs/file.rs` |
| `kernel/src/fs/file_like.rs` | 9 | [function-index.md](function-index.md) `fs/file_like.rs` |
| `kernel/src/fs/ioctl.rs` | 37 | [function-index.md](function-index.md) `fs/ioctl.rs` |
| `kernel/src/fs/mod.rs` | 8 | [function-index.md](function-index.md) `fs/mod.rs` |
| `kernel/src/fs/pipe.rs` | 19 | [function-index.md](function-index.md) `fs/pipe.rs` |
| `kernel/src/fs/pseudo.rs` | 9 | [function-index.md](function-index.md) `fs/pseudo.rs` |
| `kernel/src/ipc/mod.rs` | 25 | [function-index.md](function-index.md) `ipc/mod.rs` |
| `kernel/src/ipc/semary.rs` | 17 | [function-index.md](function-index.md) `ipc/semary.rs` |
| `kernel/src/ipc/shared_mem.rs` | 5 | [function-index.md](function-index.md) `ipc/shared_mem.rs` |
| `kernel/src/process/abi.rs` | 25 | [function-index.md](function-index.md) `process/abi.rs` |
| `kernel/src/process/futex.rs` | 11 | [function-index.md](function-index.md) `process/futex.rs` |
| `kernel/src/process/mod.rs` | 3 | [function-index.md](function-index.md) `process/mod.rs` |
| `kernel/src/process/proc.rs` | 23 | [function-index.md](function-index.md) `process/proc.rs` |
| `kernel/src/process/structs.rs` | 18 | [function-index.md](function-index.md) `process/structs.rs` |
| `kernel/src/process/thread.rs` | 25 | [function-index.md](function-index.md) `process/thread.rs` |
| `kernel/src/sync/condvar.rs` | 17 | [function-index.md](function-index.md) `sync/condvar.rs` |
| `kernel/src/sync/event_bus.rs` | 24 | [function-index.md](function-index.md) `sync/event_bus.rs` |
| `kernel/src/sync/mod.rs` | 0 | [function-index.md](function-index.md) `sync/mod.rs` |
| `kernel/src/sync/mutex.rs` | 56 | [function-index.md](function-index.md) `sync/mutex.rs` |
| `kernel/src/sync/semaphore.rs` | 21 | [function-index.md](function-index.md) `sync/semaphore.rs` |

## 结论

- 48 个恢复 Rust 文件中，46 个有可提取符号入口，2 个是纯模块边界/导出文件。
- 所有 48 个 Rust 文件都能反查到 [function-index.md](function-index.md) 或对应模块文档中的 L4 迁移记录。
- `crate/memory/Cargo.toml` 和 `rust-toolchain` 不是 Rust 源文件，已经在 [restored-tree.md](restored-tree.md)、[path-traceability.md](path-traceability.md) 和对应模块文档中作为基线文件记录。
