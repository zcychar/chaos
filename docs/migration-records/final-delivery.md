# rCore 迁移记录最终交付说明

本文是本轮中文交付入口，用来汇总“恢复了什么、每个新增文件/文件夹如何追踪、每个子模块-功能如何单独记录、后续源码迁移从哪里进入”。

## 结论

本轮已完成：

- 从匹配上游 rCore commit `66cb4181ec6d3336d507c7c1ff100127f56fcc0a` 恢复缺失模块面。
- 为每个新增文件夹建立 L1/L2 分层记录。
- 为 50 个新增文件建立 L3 文件级记录。
- 为 48 个恢复 Rust 文件建立 L4 子模块-功能记录和符号覆盖证据。
- 为 B01-B06 建立批次级详细迁移记录。
- 已执行 B01 sync + futex 源码迁移。
- 已执行 B02 memory + trap 的部分源码迁移，并把 init-time 用户栈写入耦合到恢复后的 rCore `MemorySet`。
- 已执行已批准的 RISC-V `sfence.vma` 参数修正、Sv39 非 leaf PTE flags 归一化和 file-backed 非页对齐 ELF LOAD 页内填充修正。
- 已执行 B05 process brk / heap boundary 子项，使 `SYS_BRK` 接入恢复后的 rCore `Process` 和 `MemorySet`。
- 已执行 B05 fd lifecycle/cloexec 子项，使 fd-local `FD_CLOEXEC` 接入 `Process` fd table、`fcntl/dup/close` 和 exec close loop。
- 已执行 B05 user stack init 错误传播和 ELF bounds 子项，使 exec 装载路径使用 fallible stack writer、checked ELF LOAD range 和临时 `MemorySet`。
- 已执行 B05 fork/wait parent-child 子项，使 `wait4` 只匹配当前进程 children，支持 pid/pgid 目标过滤，并在 parent exit 时将 children 转交 init。
- 明确 `kernel.rs` 不直接替代 rCore 模块；它作为语义来源，对齐到恢复后的 rCore 既有模块/API。

B01 源码语义迁移已执行；B02 已部分迁移并已通过 QEMU 启动到 busybox shell `/ #`；B03 append/seek、mmap range、epoll state、pipe endpoint、TTY/channel wakeup 子项，B04 IPC 三个子项和 B05 stack、brk、fd lifecycle/cloexec、ELF bounds、fork/wait parent-child 核心子项已完成。后续如继续修改源码，仍按新的实际运行边界或明确子模块语义差距报告文件/行、根因、预期行为和最小修复。

## 交付文档地图

| 问题 | 查看文档 |
| --- | --- |
| 总索引和状态定义 | [README.md](README.md) |
| 每个新增目录/文件的树状记录 | [restored-tree.md](restored-tree.md) |
| 每个新增路径对应哪一级记录和哪个批次 | [path-traceability.md](path-traceability.md) |
| 每个 Rust 文件的功能/API 迁移记录 | [function-index.md](function-index.md) |
| 符号级覆盖证据 | [symbol-coverage.md](symbol-coverage.md) |
| B01-B06 总批次清单 | [migration-batches.md](migration-batches.md) |
| 源码迁移批准队列 | [approval-queue.md](approval-queue.md) |
| 执行过程和证据 | [execution-log.md](execution-log.md) |
| 目标完成度审计 | [completion-audit.md](completion-audit.md) |

## 新增路径覆盖

| 层级 | 覆盖对象 | 证明文档 |
| --- | --- | --- |
| L0 | 上游恢复批次、commit、hash 校验 | [restored-tree.md](restored-tree.md)、[completion-audit.md](completion-audit.md) |
| L1 | 顶层新增路径和容器目录 | [restored-tree.md](restored-tree.md)、[path-traceability.md](path-traceability.md) |
| L2 | 新增子目录 | [restored-tree.md](restored-tree.md)、[path-traceability.md](path-traceability.md)、`modules/*.md` |
| L3 | 50 个新增文件 | [restored-tree.md](restored-tree.md)、[path-traceability.md](path-traceability.md) |
| L4 | 子模块-功能、public API、关键 impl、trait、函数入口 | [function-index.md](function-index.md)、[symbol-coverage.md](symbol-coverage.md)、`modules/*.md` |

目录覆盖口径：

- `crate/` 是本轮工作区新增容器目录。
- 真正从上游恢复的源码范围从 `crate/memory/` 开始。
- `kernel/src/{fs,ipc,process,sync}/`、`kernel/src/memory.rs`、`kernel/src/trap.rs` 和 `rust-toolchain` 均按固定上游恢复。

## 批次级记录

| 批次 | 详细文档 | 状态 |
| --- | --- | --- |
| B01 sync + futex | [batches/B01-sync-futex.md](batches/B01-sync-futex.md) | `MIGRATED` |
| B02 memory + trap | [batches/B02-memory-trap.md](batches/B02-memory-trap.md) | `PARTIAL_MIGRATED` / `QEMU_SHELL_REACHED` |
| B03 fs | [batches/B03-fs.md](batches/B03-fs.md) | `PARTIAL_MIGRATED`；append/seek、mmap range、epoll state、pipe endpoint、TTY/channel wakeup 已完成 |
| B04 ipc | [batches/B04-ipc.md](batches/B04-ipc.md) | `MIGRATED` |
| B05 process | [batches/B05-process.md](batches/B05-process.md) | `MIGRATED` for core process batch |
| B06 deferred/no-direct | [batches/B06-no-direct.md](batches/B06-no-direct.md) | `NO_DIRECT_PORT` / `BASELINE_RESTORED` |

建议后续源码迁移顺序：

1. 如 QEMU 继续暴露新的 memory/trap、process group、signal、resource 或 scheduler 运行边界，再按具体文件/行做报告和修复。
2. 对已经能正常启动的路径，优先用代码证据和运行现象驱动，不再单独做“找接口阻塞”阶段。

## kernel.rs 到 rCore 的关系

不是替代关系：

```text
kernel.rs  --作为语义参考-->  恢复后的 rCore 模块
```

真实 rCore 调用面仍由原项目模块承担：

```text
kernel/src/lib.rs
  -> kernel/src/memory.rs + crate/memory
  -> kernel/src/trap.rs
  -> kernel/src/fs
  -> kernel/src/ipc
  -> kernel/src/process
  -> kernel/src/sync
```

接口处理原则：

- 不把 `kernel.rs` 的 `std` 模拟结构直接搬进 no-std rCore。
- 优先保留恢复后 rCore 的 public API、trait、syscall facade 和模块边界。
- 把 `kernel.rs` 中明确的边界条件、不变量和生命周期语义迁移到 rCore 对应模块。
- 无真实调用面的模拟辅助结构记录为 `NO_DIRECT_PORT`，不强行迁移。

当前 B02 对齐关系：

| `kernel.rs` 语义 | rCore 落点 | 当前状态 |
| --- | --- | --- |
| `PgFrame` / `SharedPage::fault` refcount | `crate/memory/src/cow.rs` | `PARTIAL_MIGRATED` |
| `VmRegion` / `VmMap` / `AddrSpace` range 语义 | `crate/memory/src/memory_set/mod.rs` | `PARTIAL_MIGRATED` |
| `FLike::mmap_fl` file offset 语义 | `crate/memory/src/memory_set/handler/file.rs` | `PARTIAL_MIGRATED` |
| 匿名页已映射后的 fault 权限核对 | `crate/memory/src/memory_set/handler/byframe.rs` | `MIGRATED` |
| `FramePool`、`check_access`、`cfu`、`ctu` | `kernel/src/memory.rs` | `PARTIAL_MIGRATED` |
| `up_ms` | `kernel/src/trap.rs::uptime_msec` | `MIGRATED` |
| `ProcInit::push_at` 的 init-time stack 写入 | `kernel/src/process/abi.rs`、`kernel/src/process/thread.rs` | `MIGRATED` for B05.1 |
| `TaskTable::{fork_task,reap}` / `SYS_WAIT4` | `kernel/src/process/proc.rs`、`kernel/src/syscall/proc.rs` | `MIGRATED` for B05.2 |
| fault 后 PTE 对硬件可见 | `kernel/src/arch/riscv/paging.rs::PageEntry::update`、`PageTableImpl::normalize_intermediate_entries` | `MIGRATED` |

## 验证结果

当前最后一次验证结果：

- 当前工作区新增文件记录覆盖：无 `MISSING_FILE_RECORD`。
- 12 个新增目录记录覆盖：无 `MISSING_DIR_RECORD`。
- 48 个恢复 Rust 文件符号记录：无 `MISSING_SYMBOL_RECORD`。
- B01-B06 详细文档和索引链接：无 `MISSING_BATCH_DOC` / `MISSING_BATCH_LINK`。
- `/tmp/rcore-upstream` 当前不存在，本次未重新执行 upstream hash 比对；恢复源码差异以后续批次记录和当前工作区路径覆盖为准。
- 最新 debug 构建：`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 已通过并生成 `kernel.img`。
- QEMU 冒烟：已越过旧的 init stack `0x3fffffff` fault、busybox entry point `0x100e8` 反复 instruction page fault 和 `sepc=0x0` 跳转 fault。
- 最新运行证据：B05.2 后手动 QEMU 启动已进入 busybox shell `/ #`；`timeout` 结束码 124 为预期外部终止，未见 fork/wait 修改引入的 panic。

这些验证命令的完整形式记录在 [completion-audit.md](completion-audit.md)。

## 下一步批准点

下一步不是批准整个 B02，也不是继续旧的 B05 队列。B05 核心 process 批次已经完成；后续只按新的实际运行边界或明确子模块语义差距做行级修复报告。

B01 已完成的源码范围：

- `kernel/src/sync/condvar.rs:55`：`Condvar::wait_events`
- `kernel/src/sync/condvar.rs:109`：`Condvar::wait_timeout`
- `kernel/src/process/futex.rs:36`：`Futex::wake`
- `kernel/src/process/futex.rs:62`：`FutexFuture::poll`

对应详细材料见 [batches/B01-sync-futex.md](batches/B01-sync-futex.md)。

B02 已完成的主要源码范围：

- `kernel/src/memory.rs`：active `MemorySet` fault fallback、checked frame/user access。
- `kernel/src/trap.rs`：`uptime_msec` saturating conversion。
- `crate/memory/src/cow.rs`：COW refcount 边界。
- `crate/memory/src/memory_set/mod.rs`：range/align/split 基础边界和 `with<R>`。
- `crate/memory/src/memory_set/handler/file.rs`：file offset/read size checked。
- `crate/memory/src/memory_set/handler/byframe.rs`：present/access check。
- `kernel/src/process/abi.rs`、`kernel/src/process/thread.rs`：VM-backed init stack writer。
- `kernel/src/arch/riscv/paging.rs`：已批准的 `sfence.vma` vaddr 修正和 Sv39 非 leaf PTE flags 归一化。

B03 已完成的源码范围：

- `kernel/src/fs/file.rs`：append 写后 offset 写回实际写入终点，`write_at` 检查 `offset + len` overflow，`seek` 拒绝负结果和超出 `u64::MAX` 的结果，file-backed mmap 检查 `start/end` 和 `offset + len`。
- `kernel/src/syscall/fs.rs`：`sys_lseek(SEEK_SET)` 拒绝负 offset，并拒绝无法返回为 `usize` 的 offset。
- `kernel/src/fs/epoll.rs`：epoll registration/ready/new_ctl 改为共享状态，ADD 重复返回 `EEXIST`，DEL 清理 stale ready/new_ctl。
- `kernel/src/fs/pipe.rs`：pipe endpoint 增加 reader/writer 计数，关闭读端后写端返回错误，关闭写端后读端 EOF/ready，poll error 反映 broken pipe。
- `kernel/src/syscall/fs.rs`：`sys_epoll_ctl` 对 DEL 不强制读取 event；`sys_epoll_pwait` 使用事件快照并限制写回不超过 `maxevents`。
- `kernel/src/fs/file.rs`、`kernel/src/fs/file_like.rs`：pipe broken write 在 syscall 可见层映射为 `EPIPE`。
- `kernel/src/syscall/mem.rs`：`sys_mmap` 拒绝 zero-length 和 overflow range；`sys_munmap` 拒绝 zero-length、非页对齐地址和 overflow range。

B05 已完成的源码范围：

- `kernel/src/process/abi.rs`：`try_push_at`、`try_push_at_in_vm`、fallible `InitStackWriter`、checked `StackWriter` / `VmStackWriter`。
- `kernel/src/process/structs.rs`：`ElfExt::{make_memory_set,append_as_interpreter,get_phdr_vaddr}` 改为 fallible，LOAD virtual/file range、interpreter bias、PHDR inferred address 和 farthest memory checked。
- `kernel/src/process/thread.rs`：`new_user_vm` 使用临时 `MemorySet` 装载 ELF/stack，成功后替换传入 VM，错误向 `sys_exec` 返回。
- `kernel/src/process/proc.rs`：`USER_BRK_START`、`Process::{brk_start,brk}`。
- `kernel/src/process/thread.rs`：新进程 brk 初始化和 fork brk 继承。
- `kernel/src/syscall/mod.rs`、`kernel/src/syscall/mem.rs`：`SYS_BRK` 分发到 `sys_brk`，通过 `MemorySet` lazy heap mapping 增缩 brk 区间。
- `kernel/src/process/proc.rs`：`Process.fd_cloexec`、`add_file_with_cloexec`、`close_file`、`is_fd_cloexec`、`set_fd_cloexec`。
- `kernel/src/process/thread.rs`：新进程初始化空 fd-local cloexec 集合，fork 继承 fd-local flags。
- `kernel/src/syscall/fs.rs`：open/pipe/epoll create 接入 close-on-exec；`sys_close` 清理 fd metadata；`dup2` 同 fd no-op；`dup3` 校验同 fd和非法 flags；`F_DUPFD`、`F_DUPFD_CLOEXEC`、`F_GETFD`、`F_SETFD` 使用 fd-local metadata。
- `kernel/src/syscall/proc.rs`：exec close loop 关闭所有 fd-local cloexec fd，不再只处理 `FileLike::File`。
- `kernel/src/process/proc.rs`：新增私有 orphan reparent helper，parent exit 时将 children 转交 init。
- `kernel/src/syscall/proc.rs`：`sys_wait4` 只从当前 parent children 匹配 wait 目标，支持 any child、当前 pgid、指定 pid、指定 pgid，并同步清理 global table 与 parent children。
