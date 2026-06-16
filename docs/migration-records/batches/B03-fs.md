# B03 fs 详细迁移记录

本文是 B03 的批次级执行记录。它把 [migration-batches.md](../migration-batches.md) 中的 B03 拆成函数级/子模块级迁移单元，用于在源码修改前明确：目标文件/行、`kernel.rs` 语义来源、当前 rCore 表达、接口处理方式、最小修改范围和验收点。

当前状态：B03.1 append write and negative seek、B03.2 mmap file range、B03.3 epoll state、B03.4 pipe endpoint lifecycle、B03.5 TTY/channel wakeup 已按行级报告完成源码迁移。

## 批次定位

| 项目 | 内容 |
| --- | --- |
| 批次 | B03 fs |
| 恢复模块 | `kernel/src/fs/*`、`kernel/src/fs/devfs/*`，并联动现有 `kernel/src/syscall/mem.rs` |
| `kernel.rs` 来源行段 | `2241-2370`、`2488-2785`、`2907-3214`、`6430-6505` |
| 上游基线 | 恢复源码与 rCore commit `66cb4181ec6d3336d507c7c1ff100127f56fcc0a` hash 一致 |
| 接口原则 | 保留 rCore 的 `FileHandle`、`FileLike`、`EpollInstance`、`Pipe`、`TtyINode` 和 syscall 分发结构，不复制 `kernel.rs` 的模拟 `FHandle`/`PipeNode`/`EpInst`/`Channel` 类型 |

## B03.1 append write and negative seek

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/fs/file.rs:139`、`:151`、`:160` |
| rCore 函数 | `FileHandle::write`、`FileHandle::write_at`、`FileHandle::seek` |
| `kernel.rs` 来源 | `FHandle::{write,write_at,seek}`，`kernel/src/kernel.rs:2319-2357` |
| 迁移语义 | append 写入以 EOF 为起点；写后 offset 更新为 append_start + written_len；write offset + len checked；seek 结果为负或超过 `u64::MAX` 时返回错误。 |
| 当前 rCore 表达 | append 时用 EOF 作为写入 offset，但随后对旧 descriptor offset 加 len；`seek` 把 signed 结果直接 cast 为 `u64`。 |
| 接口处理 | 保留 `Result<usize>` / `Result<u64>`；错误映射使用现有 `FsError` 或上层 `SysError` 转换。 |
| 最小修改范围 | `FileHandle::write` offset 更新逻辑、`write_at` 的 checked end 前置、`seek` 的 signed range 校验。 |
| 不应修改 | `OpenFileDescription` 共享 offset 结构、`FileHandle::dup` 共享 description 语义。 |
| 验收点 | append 写后 offset 等于新 EOF；negative seek 不 wrap；超大 write range 不 wrap；非 append 写仍按旧 offset 更新。 |
| 已落地结果 | `FileHandle::write` 使用实际写入起点 `offset` 写回 `offset + len`；`write_at` 对 `offset + buf.len()` 做 checked 前置校验；`seek` 用 `i128` 计算并拒绝负数/超出 `u64::MAX`；`sys_lseek(SEEK_SET)` 拒绝负 offset。 |
| 验收结果 | `make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；`/tmp/chaos-qemu-b03-file.log` 到达 busybox shell `/ #`，无 panic 或 `page fault from user @ 0x0`。 |
| 状态 | `MIGRATED` |

## B03.2 mmap file range

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/fs/file.rs:228`、`kernel/src/syscall/mem.rs:10`、`:96` |
| rCore 函数 | `Syscall::sys_mmap`、`Syscall::sys_munmap`、`FileHandle::mmap` |
| `kernel.rs` 来源 | `FLike::mmap_fl`，`kernel/src/kernel.rs:2754-2775`；`SYS_MMAP`/`SYS_MUNMAP`，`:6430-6505` |
| 迁移语义 | `len == 0` 拒绝；`addr + len` checked；munmap addr/len page alignment 明确；file mapping 的 `file_end = offset + len` checked。 |
| 当前 rCore 表达 | `sys_mmap`、`sys_munmap`、`FileHandle::mmap` 多处直接使用 `addr + len`、`area.offset + area.end_vaddr - area.start_vaddr`。 |
| 接口处理 | 保留 syscall 返回 `SysResult`；合法半开 range 落到 `MemorySet::push/pop_with_split`；非法 range 返回现有 errno。 |
| 最小修改范围 | `sys_mmap`、`sys_munmap` range 前置校验；`FileHandle::mmap` file_end 计算；必要时联动 B02 的 `MemorySet` checked range。 |
| 不应修改 | `MmapProt`/`MmapFlags` bit layout、`MMapArea` ABI。 |
| 验收点 | zero-length mmap/munmap 返回错误；`addr + len` overflow 返回错误；file offset + mapping len overflow 返回错误；合法匿名/file mapping 仍进入原 handler。 |
| 已落地结果 | `sys_mmap` 拒绝 `len == 0` 并 checked 计算 mapping end；`sys_munmap` 拒绝 zero-length 和非页对齐 addr，并按页对齐 end 执行 `pop_with_split`；`FileHandle::mmap` checked 计算 `mapping_len` 和 file-backed `file_end`。 |
| 验收结果 | `make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；`/tmp/chaos-qemu-b03-mmap.log` 到达 busybox shell `/ #`，无 panic 或 `page fault from user @ 0x0`。 |
| 状态 | `MIGRATED` |

## B03.3 epoll state

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/fs/file_like.rs:20`、`kernel/src/fs/epoll.rs:10`、`:16`、`:35`、`kernel/src/syscall/fs.rs:318`、`:364` |
| rCore 函数 | `FileLike::dup`、`EpollInstance::clone`、`EpollInstance::control`、`Syscall::{sys_epoll_ctl,sys_epoll_pwait}` |
| `kernel.rs` 来源 | `FLike::dup` epoll 分支，`kernel/src/kernel.rs:2603-2627`；`EpInst::control`，`:2907-2950` |
| 迁移语义 | dup 后 epoll registration/ready/new_ctl state 共享或等价保持；ADD 重复 fd 返回错误；DEL 清理 `events`、`ready_list`、`new_ctl_list`。 |
| 修改前 rCore 表达 | `EpollInstance::clone` 返回空实例；ADD 直接 insert 覆盖旧 fd；DEL 只 remove `events`；`sys_epoll_ctl` 对 DEL 也强制读取 event 指针；`epoll_pwait` 写回 events 时未按 `maxevents` 截断。 |
| 接口处理 | 保留 `FileLike::EpollInstance` 变体；如需要共享状态，应在 `EpollInstance` 内部状态表达调整，不改变 syscall 外部 ABI。 |
| 最小修改范围 | `EpollInstance` state representation、`Clone`、`control` ADD/DEL 分支、通过 `FileLike::dup` 复用 `Clone`、`sys_epoll_ctl` event 参数分支、`sys_epoll_pwait` events 快照和写回边界。 |
| 不应修改 | `EpollEvent` ABI 常量、`Process::get_epoll_instance*` 调用面。 |
| 验收点 | dup 后旧/new fd 看到同一 registration；重复 ADD 返回错误；DEL 后 ready/new_ctl 不留 stale fd；MOD missing 返回错误。 |
| 已落地结果 | `EpollInstance::{events,ready_list,new_ctl_list}` 改为 `Arc<SpinNoIrqLock<_>>` 共享状态；`Clone` 保持 registration/ready/new_ctl；ADD 已存在返回 `EEXIST`；DEL 同时清理 events、ready_list、new_ctl_list；非法 op 返回 `EINVAL`；`sys_epoll_ctl` 仅 ADD/MOD 读取 event，DEL 忽略 event；`sys_epoll_pwait` 通过事件快照遍历并限制写回不超过 `maxevents`。 |
| 验收结果 | `rustfmt --edition 2018 kernel/src/fs/epoll.rs kernel/src/syscall/fs.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止。 |
| 状态 | `MIGRATED` |

## B03.4 pipe endpoint lifecycle

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/fs/pipe.rs:22`、`:36`、`:52`、`:90`、`:100`、`:141`、`kernel/src/fs/file.rs:94`、`kernel/src/fs/file_like.rs:39` |
| rCore 函数 | `PipeData`、`Pipe` clone/drop、`Pipe::{can_read,can_write,is_write_closed}`、`INode for Pipe::{read_at,write_at,poll,async_poll}`、`FileHandle::is_broken_pipe_write`、`FileLike::write` |
| `kernel.rs` 来源 | `PipeNode::{clone,drop,can_write,write_at}`，`kernel/src/kernel.rs:2488-2594` |
| 迁移语义 | clone 增加端点计数；drop 按方向减少 reader/writer 计数且 saturating；无 reader 时 write 返回错误；poll readiness 反映 reader/writer 生命周期。 |
| 修改前 rCore 表达 | `Pipe` derive clone 不增加 `end_cnt`；`Drop` 总是 `end_cnt -= 1`；`can_write` 只看 `end_cnt == 2`；`write_at` 不显式检查 live reader；`poll.error` 固定为 false。 |
| 接口处理 | 保留 `INode for Pipe` 和 `Pipe::create_pair`；在 `PipeData` 内部增加 reader/writer 统计。由于 `rcore-fs::FsError` 无 `BrokenPipe` 变体，pipe broken write 在 `FileLike::write` 层映射为现有 `SysError::EPIPE`。 |
| 最小修改范围 | `PipeData` 端点状态、`Pipe` clone/drop、`can_read/can_write/write_at/poll/async_poll`，以及 `FileHandle`/`FileLike` 的 pipe broken write 错误桥接。 |
| 不应修改 | `Pipe::create_pair` 返回类型、`INode` trait 方法签名。 |
| 验收点 | clone/drop 后计数不 underflow；read end 全关后 write 返回错误；write 端全关后 read 返回 EOF/ready；poll read/write/error 和 endpoint 状态一致。 |
| 已落地结果 | `PipeData` 新增 `readers/writers`；`Pipe::clone/drop` 按方向维护计数并 saturating drop；`can_read` 由 buffer 或 writers==0 决定；`can_write` 由 readers>0 决定；无 reader 写入返回错误并在 `FileLike::write` 映射到 `EPIPE`；`poll.error` 反映写端 broken pipe；`async_poll` 对 error 也立即 ready。 |
| 验收结果 | `rustfmt --edition 2018 kernel/src/fs/pipe.rs kernel/src/fs/file.rs kernel/src/fs/file_like.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止。 |
| 状态 | `MIGRATED` |

## B03.5 TTY/channel wakeup

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/fs/devfs/tty.rs:39`、`:68`、`:86`、`:142`，`kernel/src/sync/event_bus.rs:110` |
| rCore 函数 | `TtyINode::push`、`TtyINode::pop`、`INode for TtyINode::{read_at,async_poll}`、`EventBus::subscribe_waker` |
| `kernel.rs` 来源 | `Channel::{recv,send,close,send_batch}`，`kernel/src/kernel.rs:2992-3214` |
| 迁移语义 | push 后 readable/wakeup 不遗漏 waiter；read 按用户 buffer 尽量取走已有字节；read 清空 buffer 后清理 readable；pending async poll 不重复累计同一 waker/mask。 |
| 修改前 rCore 表达 | `read_at` 有数据时只写 `buf[0]`，零长度读会越界；`async_poll` 每次 pending 都 subscribe callback；callback 生命周期依赖 `EventBus` 后续事件变化清理。 |
| 接口处理 | 不新增 `Channel` 模块；语义落到 TTY buffer、EventBus 和 serial input path。 |
| 最小修改范围 | `TtyINode::{pop,read_at,async_poll}` 和 `EventBus` 的 waker/mask 去重订阅 helper；保留原 `subscribe` 接口给 semaphore/process 等既有路径。 |
| 不应修改 | TTY ioctl ABI、foreground pgid public helper。 |
| 验收点 | 多字节输入不会漏唤醒；read 清空后 `poll.read == false`；pending async poll 不累计 stale callback；serial CR/LF 路径仍正常。 |
| 已落地结果 | `read_at` 对空 buffer 返回 `Ok(0)`，并循环 drain `VecDeque` 到用户 buffer；`pop/read_at` 在 buffer 清空后清理 `READABLE`；`async_poll` 使用 `EventBus::subscribe_waker(Event::READABLE, waker)`，同一 waker/mask 不重复注册，事件已就绪时立即返回 ready。 |
| 验收结果 | `rustfmt --edition 2018 kernel/src/sync/event_bus.rs kernel/src/fs/devfs/tty.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止。 |
| 状态 | `MIGRATED` |

## 接口边界

| `kernel.rs` 结构 | rCore 落点 | 处理方式 |
| --- | --- | --- |
| `FHandle` | `FileHandle`、`OpenFileDescription` | 保留 rCore fd/open-file description；迁移 append/seek/write range 语义。 |
| `FLike` | `FileLike` enum | 保留 rCore file/socket/epoll 分发；迁移 dup 共享状态和错误 side effect。 |
| `EpInst` | `EpollInstance` | 保留 rCore epoll ABI；修正 registration/ready/control lifecycle。 |
| `PipeNode` | `Pipe`、`PipeData` | 保留 `INode` pipe；迁移 endpoint reader/writer 生命周期。 |
| `Channel` | `TtyINode`、serial input、EventBus | 不新增同名模块；迁移 readable/wakeup/close 语义。 |

## 批次内顺序

1. 已处理 `FileHandle::write/write_at/seek` 和 `sys_lseek` 负 offset。
2. 已处理 `sys_mmap/sys_munmap/FileHandle::mmap`，并与 B02 range helper 保持 syscall 层前置校验。
3. 已处理 `EpollInstance` clone/control、`FileLike::dup` epoll state sharing、`epoll_ctl` DEL event 参数和 `epoll_pwait` 写回边界。
4. 已处理 `Pipe` endpoint lifecycle。
5. 已处理 TTY/channel wakeup，并将必要的去重订阅能力落到 `EventBus`。

## 风险和验收

| 风险 | 验收方式 |
| --- | --- |
| 改 epoll state 表达影响 `Process.files` clone/drop | 只调整 `EpollInstance` 内部共享状态，保持 `FileLike` 变体和 fd 表接口。 |
| pipe clone/drop 改动导致端点计数重复 | 明确 reader/writer 计数，不只用总 `end_cnt` 推断。 |
| mmap range 与 B02 MemorySet range 重复处理 | syscall 层先拒绝非法 range；MemorySet 层保留自身不变量。 |
| TTY callback 清理和 EventBus 行为耦合 | 已通过 `EventBus::subscribe_waker` 只服务 TTY/wait future 去重，保留旧 `subscribe` 兼容原调用面。 |

## 已执行报告摘要

B03.1-B03.5 源码修改前均已按仓库规则报告并获得后续继续授权：

- B03.1-B03.4 已完成 file、mmap、epoll、pipe 子项。
- B03.5 报告的文件/行是 `kernel/src/fs/devfs/tty.rs:84`、`:122` 和 `kernel/src/sync/event_bus.rs:85`；问题是零长度读越界、单次只读 1 字节、pending poll 重复注册；已按最小范围修复并通过构建/QEMU smoke。
