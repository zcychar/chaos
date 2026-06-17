# `kernel/src/process` 迁移记录

## 模块定位

`kernel/src/process` 是恢复后 rCore 的进程、线程、futex、ELF/ABI 辅助和调度执行层。它对应 `kernel.rs` 中 `ProcInit`、`Task`、`TaskTable`、`RunQueue`、`FutexTable`、process group/wait/resource/fd 生命周期相关语义。

当前状态：B01 已迁移 `futex.rs`；B02 为了解决用户栈初始化与 `MemorySet` 的耦合，已部分迁移 `abi.rs` 和 `thread.rs`；B05 已迁移 process brk / heap boundary、fd lifecycle/cloexec、user stack 错误传播、ELF bounds 和 fork/wait parent-child 核心生命周期。

| 对应 `kernel.rs` 行段 | 迁移主题 |
| --- | --- |
| `4387-4776` | process init、signal/timer 与进程状态。 |
| `5194-6054` | scheduler、run queue、task、task table。 |
| `6055-7528` | syscall facade 中 fork/exec/wait/fd/process side effect。 |
| `7662-8071` | process group、wait queue、resource limit。 |

## 文件-功能记录

| 文件 | 主要结构/函数 | `kernel.rs` 对齐语义 | 迁移记录 |
| --- | --- | --- | --- |
| `mod.rs` | module exports、`init`、`current_thread`、processor-local current thread。 | `TaskTable` facade、current task lookup。 | 保持上游入口；迁移语义落在 `proc.rs`、`thread.rs` 和 syscall。 |
| `abi.rs` | `ProcInitInfo`、`StackWriter`、`InitStackWriter`、`VmStackWriter`、stack push helpers。 | `ProcInit::push_at`、用户栈布局 underflow、init-time VM 写入。 | 已新增 fallible `try_push_at` / `try_push_at_in_vm`；真实 exec 初始化通过 `MemorySet` 写栈并把 overflow/page prepare 失败作为错误返回。 |
| `futex.rs` | `Futex`、`Waiter`、async wait future、wake。 | `FutexTable::ftx_wait/ftx_wake`。 | 重点迁移 wake count 精确、timeout 后 waiter 不滞留。 |
| `proc.rs` | `Pid`、`Pgid`、`Process`、global process table、fd table、children、signals、IPC/shm state、program break。 | `Task`、`TaskTable`、`VmMap::brk`、process group/session、fd close/dup/cloexec、signal queue。 | 已新增 `USER_BRK_START`、`brk_start`、`brk`、fd-local `fd_cloexec` helper 和 orphan reparent helper；B05 parent-child wait/reap 已落地，process group/session/signal queue 更广语义仍按新边界推进。 |
| `structs.rs` | `ElfExt`、`ToMemoryAttr`、`INodeForMap`。 | ELF validation、file-backed mmap、program header bounds。 | `ElfExt` loader 已改为 fallible checked range；file mmap read bounds 与 B03/B05 记录关联。 |
| `thread.rs` | `Tid`、`ThreadContext`、`ThreadInner`、`Thread`、spawn/yield、executor wrapper、context switching。 | scheduler/run queue、context/trap state、thread lifecycle、user VM 初始化。 | `Thread::new_user_vm` 已使用临时 VM、fallible ELF loader 和 `try_push_at_in_vm`；fork parent-child link 经 B05.2 复核后保持上游单次链接。 |

## 子模块-功能迁移记录

| 功能项 | rCore 落点 | `kernel.rs` 来源 | 当前迁移状态 |
| --- | --- | --- | --- |
| current task lookup | `mod.rs::current_thread`、arch CPU state | `Kernel::cur_task`、`TaskTable` | `BASELINE_RESTORED` |
| user stack layout | `abi.rs::ProcInitInfo`、`StackWriter`、`VmStackWriter`、`thread.rs::Thread::new_user_vm` | `ProcInit::push_at` | `MIGRATED` |
| futex wake exact count | `futex.rs::Futex::wake` | `FutexTable::ftx_wake` | `MIGRATED` |
| futex timeout cleanup | `futex.rs::Futex::wait` | `FutexTable` timeout behavior | `MIGRATED` |
| process global table | `proc.rs::PROCESSES`、`process/process_of/process_group` | `TaskTable` map | `PARTIAL_MIGRATED` |
| process brk / heap boundary | `proc.rs::Process::{brk_start,brk}`、`thread.rs::{new_user,fork}`、`syscall/mem.rs::sys_brk` | `VmMap::brk`、`SYS_BRK` | `MIGRATED` |
| parent-child link | `proc.rs::Process::{children,parent}`、`syscall/proc.rs` | `TaskTable::fork_task/reap` | `MIGRATED` |
| fd lifecycle | `proc.rs::Process.{files,fd_cloexec}`、`fs/file_like.rs`、`syscall/fs.rs`、`syscall/proc.rs::sys_exec` | `Task::close_fd/dup_fd/dup2_fd/set_cloexec`、exec close-on-exec | `MIGRATED` |
| process group | `proc.rs::Pgid`、`process_group`、`syscall/proc.rs` | `ProcessGroup`、`setsid/setpgid/wait filtering` | `PARTIAL_MIGRATED` |
| signal queue | `proc.rs::sig_queue/pending_sigset/dispositions`、`signal/*` | `SigSet`、`Task::send_sig` | `MIGRATION_PENDING` |
| ELF load bounds | `structs.rs::ElfExt`、`thread.rs::Thread::new_user_vm` | `validate_elf_header` | `MIGRATED` |
| file-backed mmap | `structs.rs::INodeForMap`、`crate/memory::File` | `FLike::mmap_fl` | `MIGRATION_PENDING` |
| executor scheduling | `thread.rs::spawn/yield_now` | `RunQueue`、`SchedulePolicy` | `NO_DIRECT_PORT` |
| thread exit/reap | `thread.rs`、`proc.rs::Process::exit`、`syscall/proc.rs` | `TaskTable::reap` | `MIGRATED` for B05.2 |

## 待批准迁移候选

| 优先级 | 位置 | 迁移语义 | 最小范围 |
| --- | --- | --- | --- |
| 已完成 | `kernel/src/process/abi.rs`、`kernel/src/process/thread.rs` | 用户栈写入与正在构造的 `MemorySet` 关联，避免 init-time current thread 依赖，并将 stack overflow/page prepare 失败作为错误返回。 | `try_push_at_in_vm`、`VmStackWriter`、`Thread::new_user_vm` 调用点。 |
| 已完成部分 | `kernel/src/process/proc.rs`、`kernel/src/process/thread.rs`、`kernel/src/syscall/mem.rs`、`kernel/src/syscall/mod.rs` | `brk` 不再返回 unimplemented/ENOMEM，process heap 通过恢复后的 `MemorySet` 建立 lazy mapping。 | `Process` brk 状态、new/fork 初始化继承、`sys_brk`。 |
| 已完成 | `kernel/src/process/proc.rs` 和 `syscall/proc.rs` | wait 只从当前 parent children 匹配目标，reap 同步清理 parent list/global table，exit 将 orphan children 转交 init。 | process fork/wait/exit 路径。 |
| 已完成 | `kernel/src/process/proc.rs`、`kernel/src/process/thread.rs`、`kernel/src/syscall/fs.rs`、`kernel/src/syscall/proc.rs` | fd close/dup/cloexec 实际修改 process fd table，并在 exec 中关闭所有 fd-local cloexec fd。 | `Process.fd_cloexec` helper、fork 继承、open/pipe/epoll create、close/dup/fcntl/exec close loop。 |
| 已完成 | `kernel/src/process/structs.rs`、`kernel/src/process/thread.rs` | ELF program header bounds 使用 checked arithmetic，malformed ELF 经 `new_user_vm` 返回错误。 | `ElfExt` fallible load path。 |
