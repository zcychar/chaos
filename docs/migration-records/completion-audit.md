# 目标完成度审计

本文按用户目标逐条审计当前工作区证据。结论：恢复基线和 L0-L4 迁移记录已经完成；B01 sync + futex 源码语义迁移已执行；B02 memory + trap 已部分迁移并已通过 QEMU 启动到 busybox shell；B03 append/seek、mmap range、epoll state、pipe endpoint、TTY/channel wakeup 子项、B04 IPC 三个子项和 B05 process brk、fd lifecycle/cloexec、user stack 错误传播、ELF bounds、fork/wait parent-child 核心子项已执行。

## 审计范围

用户目标：

```text
根据当前的文档指导开始进行分步骤完成。
对于每一个子模块-功能单独进行迁移记录。
最终呈现应当为对于每一个新加入的文件/文件夹都有一个不同层级的详细迁移记录。
```

当前约束：

- `kernel/src/kernel.rs` 不进入真实 rCore 构建，只作为语义迁移参考。
- 恢复后的上游 rCore 模块面作为可运行基线。
- 修改源码前必须先报告目标文件/位置、迁移语义、当前表达、预期结果和最小修改范围，并等待批准；B01 已按该流程完成，B02 后续如出现新的运行边界仍需重新报告。

## 要求覆盖矩阵

| 要求 | 当前证据 | 审计结论 |
| --- | --- | --- |
| 按当前文档指导开始分步骤完成 | [execution-log.md](execution-log.md) 记录 Phase 1 恢复、L4 功能索引、批次执行单、B01 源码迁移、B02 部分迁移、B03.1-B03.5、B04.1-B04.3、B05 brk/fd/stack/ELF/fork-wait 迁移和 QEMU shell 运行证据；[migration-batches.md](migration-batches.md) 记录 B01-B06。 | 已开始并完成恢复/记录阶段、B01 迁移、B02 部分迁移、B03.1-B03.5、B04.1-B04.3 和 B05 核心 process 子项；B02 或更广 process/signal/resource 后续如有新故障再行级报告。 |
| 恢复 rCore 可运行基线 | [restored-tree.md](restored-tree.md) L0/L1 记录恢复范围；本地 hash 校验无 mismatch。 | 已完成。 |
| 每个新加入文件夹有分层记录 | [restored-tree.md](restored-tree.md) L1/L2 记录顶层路径和子目录；[path-traceability.md](path-traceability.md) 逐目录记录层级、模块文档、批次和状态。 | 已完成。 |
| 每个新加入文件有文件级记录 | [restored-tree.md](restored-tree.md) L3 记录 50 个恢复文件；[path-traceability.md](path-traceability.md) 逐文件记录层级、模块文档、批次和状态。 | 已完成。 |
| 每个子模块-功能有单独迁移记录 | [function-index.md](function-index.md) 按新增文件列出 public API、关键 impl、trait、功能块；`modules/*.md` 按模块记录功能语义；[symbol-coverage.md](symbol-coverage.md) 记录 48 个恢复 Rust 文件的符号入口覆盖。 | 已完成功能级记录。 |
| 最终中文交付入口 | [final-delivery.md](final-delivery.md) 汇总恢复范围、记录层级、批次入口、验证结果和下一步批准点。 | 已完成。 |
| 需要源码修改的迁移项有报批材料 | [migration-batches.md](migration-batches.md) B01-B05 均记录目标文件/行、来源、语义、当前表达、预期结果、最小范围。 | 已完成报批材料。 |
| B01 批次有函数级执行记录 | [batches/B01-sync-futex.md](batches/B01-sync-futex.md) 分别记录 `Condvar::wait_events`、`Condvar::wait_timeout`、`Futex::wake`、`FutexFuture::poll`，并记录源码差异与验证。 | 已完成并已迁移。 |
| B02 批次有函数级/子模块级执行记录 | [batches/B02-memory-trap.md](batches/B02-memory-trap.md) 分别记录 COW、MemorySet、file-backed mapping、frame/user access、tick conversion。 | 已完成。 |
| B03 批次有函数级/子模块级执行记录 | [batches/B03-fs.md](batches/B03-fs.md) 分别记录 file、mmap、epoll、pipe、TTY/channel。 | 已完成。 |
| B04 批次有函数级/子模块级执行记录 | [batches/B04-ipc.md](batches/B04-ipc.md) 分别记录 semaphore create/get、undo/remove、shared memory key/size。 | 已完成。 |
| B05 批次有函数级/子模块级执行记录 | [batches/B05-process.md](batches/B05-process.md) 分别记录 stack、fork/wait、fd lifecycle、ELF bounds、process brk。 | 已完成。 |
| B06 no-direct 批次有保留/重新评估记录 | [batches/B06-no-direct.md](batches/B06-no-direct.md) 记录 swap、mock page table、random、fbdev、rust-toolchain 的处理结论。 | 已完成。 |
| 已执行源码语义迁移 | B01 修改 `kernel/src/process/futex.rs`、`kernel/src/sync/condvar.rs`；B02 修改 `kernel/src/memory.rs`、`kernel/src/trap.rs`、`crate/memory/src/cow.rs`、`crate/memory/src/memory_set/mod.rs`、`crate/memory/src/memory_set/handler/{file,byframe}.rs`、`kernel/src/process/{abi,thread}.rs`，并有已批准的 `kernel/src/arch/riscv/paging.rs` 修正；B03 修改 `kernel/src/fs/file.rs`、`kernel/src/syscall/fs.rs` 的 append/seek 子项，修改 `kernel/src/fs/file.rs`、`kernel/src/syscall/mem.rs` 的 mmap range 子项，修改 `kernel/src/fs/epoll.rs`、`kernel/src/syscall/fs.rs` 的 epoll state 子项，修改 `kernel/src/fs/{pipe,file,file_like}.rs` 的 pipe endpoint 子项，并修改 `kernel/src/fs/devfs/tty.rs`、`kernel/src/sync/event_bus.rs` 的 TTY/channel wakeup 子项；B04 修改 `kernel/src/ipc/semary.rs`、`kernel/src/syscall/ipc.rs` 的 semaphore create/get 子项，修改 `kernel/src/ipc/mod.rs` 的 semaphore undo/remove 子项，并修改 `kernel/src/ipc/shared_mem.rs`、`kernel/src/syscall/ipc.rs` 的 shared memory key/size 子项；B05 修改 `kernel/src/process/{abi,proc,structs,thread}.rs`、`kernel/src/syscall/{fs,mod,mem,proc}.rs`，覆盖 brk、fd lifecycle/cloexec、stack 错误传播、ELF bounds 和 fork/wait parent-child。 | B01 已完成；B02 部分完成；B03.1-B03.5 已完成；B04.1-B04.3 已完成；B05 核心 process 批次已完成。 |
| 迁移后运行验收 | B02 后 `make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；QEMU 已越过旧的 init stack `0x3fffffff` fault、busybox entry `0x100e8` instruction page fault 和 `sepc=0x0` 跳转 fault，进入 busybox shell `/ #`。B05 brk 后同一构建命令通过，`/tmp/chaos-qemu-brk.log` 进入 `/ #` 且无 `brk is unimplemented`。B03.1 后构建通过，`/tmp/chaos-qemu-b03-file.log` 进入 `/ #`，无 panic 或 `page fault from user @ 0x0`。B03.2 后构建通过，`/tmp/chaos-qemu-b03-mmap.log` 进入 `/ #`，无 panic 或 `page fault from user @ 0x0`。B03.3-B03.5、B04.1-B04.3、B05.3、B05.1/B05.4 和 B05.2 后均完成 `rustfmt`、同一构建命令和手动 QEMU 冒烟；最新 B05.2 后 QEMU 进入 `/ #`，`timeout` 结束码 124 为预期外部终止。 | 构建通过；QEMU shell 已到达；brk warning 已解除；B03.1-B03.5、B04.1-B04.3 和 B05 核心 process 子项未破坏启动。下一步转向新的实际运行边界。 |

## 文件/目录覆盖证据

恢复文件总数：

```text
50
```

覆盖记录：

- L0 恢复批次：[restored-tree.md](restored-tree.md)
- L1 顶层新增路径：[restored-tree.md](restored-tree.md)
- L2 子目录：[restored-tree.md](restored-tree.md)
- L3 单文件：[restored-tree.md](restored-tree.md)
- L4 子模块-功能：[function-index.md](function-index.md) 和 `modules/*.md`
- 最终交付入口：[final-delivery.md](final-delivery.md)
- 逐路径追踪矩阵：[path-traceability.md](path-traceability.md)
- 符号级覆盖证据：[symbol-coverage.md](symbol-coverage.md)
- 批次执行单：[migration-batches.md](migration-batches.md)
- B01 函数级批次记录：[batches/B01-sync-futex.md](batches/B01-sync-futex.md)
- B02 函数级/子模块级批次记录：[batches/B02-memory-trap.md](batches/B02-memory-trap.md)
- B03 函数级/子模块级批次记录：[batches/B03-fs.md](batches/B03-fs.md)
- B04 函数级/子模块级批次记录：[batches/B04-ipc.md](batches/B04-ipc.md)
- B05 函数级/子模块级批次记录：[batches/B05-process.md](batches/B05-process.md)
- B06 no-direct 批次记录：[batches/B06-no-direct.md](batches/B06-no-direct.md)

## 验证命令

这些命令用于证明“记录覆盖”和“除已迁移 B01/B02 文件外，其余恢复基线未被意外修改”：

```bash
git -C /tmp/rcore-upstream ls-tree -r --name-only \
  66cb4181ec6d3336d507c7c1ff100127f56fcc0a \
  crate/memory kernel/src/memory.rs kernel/src/trap.rs \
  kernel/src/fs kernel/src/ipc kernel/src/process kernel/src/sync rust-toolchain |
while read path; do
  rg -q --fixed-strings "$path" docs/migration-records ||
    printf 'MISSING_RECORD %s\n' "$path"
done
```

```bash
for p in \
  crate crate/memory crate/memory/src crate/memory/src/memory_set \
  crate/memory/src/memory_set/handler crate/memory/src/paging \
  crate/memory/src/swap kernel/src/fs kernel/src/fs/devfs \
  kernel/src/ipc kernel/src/process kernel/src/sync
do
  rg -q --fixed-strings "$p" docs/migration-records ||
    printf 'MISSING_DIR_RECORD %s\n' "$p"
done
```

```bash
git -C /tmp/rcore-upstream ls-tree -r \
  66cb4181ec6d3336d507c7c1ff100127f56fcc0a \
  crate/memory kernel/src/memory.rs kernel/src/trap.rs \
  kernel/src/fs kernel/src/ipc kernel/src/process kernel/src/sync rust-toolchain |
while read mode type hash path; do
  case "$path" in
    kernel/src/process/futex.rs|kernel/src/sync/condvar.rs|\
    kernel/src/memory.rs|kernel/src/trap.rs|\
    crate/memory/src/cow.rs|crate/memory/src/memory_set/mod.rs|\
    crate/memory/src/memory_set/handler/file.rs|\
    crate/memory/src/memory_set/handler/byframe.rs|\
    kernel/src/process/abi.rs|kernel/src/process/thread.rs) continue ;;
  esac
  local_hash=$(git hash-object "$path")
  if test "$hash" != "$local_hash"; then
    printf 'MISMATCH %s upstream=%s local=%s\n' "$path" "$hash" "$local_hash"
  fi
done
```

```bash
for path in $(
  find crate/memory/src kernel/src/fs kernel/src/ipc \
    kernel/src/process kernel/src/sync -name '*.rs' -type f | sort
  printf '%s\n' kernel/src/memory.rs kernel/src/trap.rs
); do
  rg -q --fixed-strings "$path" \
    docs/migration-records/function-index.md \
    docs/migration-records/symbol-coverage.md \
    docs/migration-records/modules ||
    printf 'MISSING_SYMBOL_RECORD %s\n' "$path"
done
```

```bash
for b in B01-sync-futex B02-memory-trap B03-fs B04-ipc B05-process B06-no-direct
do
  test -f "docs/migration-records/batches/$b.md" ||
    printf 'MISSING_BATCH_DOC %s\n' "$b"
  rg -q --fixed-strings "batches/$b.md" \
    docs/migration-records/README.md \
    docs/migration-records/migration-batches.md \
    docs/migration-records/execution-log.md \
    docs/migration-records/completion-audit.md \
    docs/migration-records/approval-queue.md ||
    printf 'MISSING_BATCH_LINK %s\n' "$b"
done
```

```bash
test -f docs/migration-records/final-delivery.md ||
  printf 'MISSING_FINAL_DELIVERY\n'
rg -q --fixed-strings 'final-delivery.md' \
  docs/migration-records/README.md \
  docs/migration-records/execution-log.md \
  docs/migration-records/completion-audit.md ||
  printf 'MISSING_FINAL_DELIVERY_LINK\n'
```

当前最后一次运行结果：

- `/tmp/rcore-upstream` 当前不存在，因此本次没有重新执行 upstream tree hash 比对；旧 hash 比对结果只作为历史记录保留。
- 当前工作区新增文件覆盖检查无 `MISSING_FILE_RECORD` 输出。
- 无 `MISSING_DIR_RECORD` 输出。
- 无 `MISSING_SYMBOL_RECORD` 输出。
- 无 `MISSING_BATCH_DOC` 输出。
- 无 `MISSING_BATCH_LINK` 输出。
- 无 `MISSING_FINAL_DELIVERY` 输出。
- 无 `MISSING_FINAL_DELIVERY_LINK` 输出。

## 未完成项

未完成项不是记录缺失；B05 核心 process 批次已完成。B02 当前已通过 QEMU shell 冒烟，但仍不是“全部语义完成”，更广 process group/session/signal/resource/scheduler 也只在新的运行边界出现时继续：

| 批次 | 内容 | 当前状态 |
| --- | --- | --- |
| B02 | 已修复 `0x100e8` instruction page fault 和 file-backed 非页对齐 LOAD 导致的 `sepc=0x0` fault；后续如出现新 memory/trap 故障再单独报告。 | `PARTIAL_MIGRATED` / `QEMU_SHELL_REACHED` |
| B03 | append/seek、mmap range、epoll state、pipe endpoint、TTY/channel wakeup 已完成；fd/open-file lifecycle 联动项已由 B05.3 收口。 | `PARTIAL_MIGRATED`；B03.1-B03.5 已完成 |
| B04 | semaphore create/get、undo/remove、shared memory key/size 已完成。 | `MIGRATED` |
| B05 | process brk、fd lifecycle/cloexec、stack 错误传播、ELF bounds、fork/wait parent-child 核心子项已完成。 | `MIGRATED` for core process batch |

## 下一步判定

如果目标仅要求“恢复基线 + 为每个新增文件/文件夹/子模块-功能建立详细迁移记录 + 每个路径能反查到迁移批次 + 开始分步骤执行源码迁移”，当前证据已满足到 B02 部分迁移、B03 append/seek、mmap range、epoll state、pipe endpoint 与 TTY/channel wakeup 子项、B04 IPC、B05 brk、fd lifecycle/cloexec、stack 错误传播、ELF bounds 与 fork/wait parent-child 子项并通过 QEMU shell 冒烟。

如果目标要求“实际完成全部源码语义迁移并让恢复模块与当前 rCore 体系稳定运行”，下一步应从新的真实运行故障进入；任何源码修改仍按文件/行、根因、预期行为、最小修复范围先报告。
