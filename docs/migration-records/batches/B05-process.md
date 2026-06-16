# B05 process 详细迁移记录

本文是 B05 的批次级执行记录。它把 [migration-batches.md](../migration-batches.md) 中的 B05 拆成函数级/子模块级迁移单元，用于在源码修改前明确：目标文件/行、`kernel.rs` 语义来源、当前 rCore 表达、接口处理方式、最小修改范围和验收点。

当前状态：B05 五组核心子项均已落地：`kernel/src/process/abi.rs` 和 `kernel/src/process/thread.rs` 因 B02 init-time stack fault 做了 VM-backed 写入耦合，且 B05.1 已完成 stack checked arithmetic/error propagation；`Process` program break 与 `sys_brk` 已按运行证据完成最小迁移；fd lifecycle/cloexec 已接入 `Process` fd-local metadata、syscall fs 和 exec close loop；B05.4 ELF bounds 已接入 fallible loader；B05.2 fork/wait parent-child 已完成 wait 目标过滤、reap cleanup 和孤儿进程转交 init。

```text
批准执行 B05 process 源码迁移。
```

## 批次定位

| 项目 | 内容 |
| --- | --- |
| 批次 | B05 process |
| 恢复模块 | `kernel/src/process/*`，并联动现有 `kernel/src/syscall/{proc,fs}.rs` |
| `kernel.rs` 来源行段 | `1950-2022`、`4387-4470`、`5736-5801`、`5814-5968`、`6509-6540`、`6700-6785`、`7436-7475` |
| 上游基线 | 恢复源码与 rCore commit `66cb4181ec6d3336d507c7c1ff100127f56fcc0a` hash 一致 |
| 接口原则 | 保留 rCore 的 `Process`、`Thread`、`ProcInitInfo`、`ElfExt`、syscall process/fs 分发结构，不复制 `kernel.rs` 的模拟 `Task`/`TaskTable` 类型 |

## B05.1 user stack init

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/process/abi.rs:13`、`:24`、`:63`、`:81`、`:102`、`:119`；`kernel/src/process/thread.rs:116`、`:195`、`:221` |
| rCore 函数 | `ProcInitInfo::{push_at,try_push_at,push_at_in_vm,try_push_at_in_vm,push_to}`、`StackWriter::{push_slice,push_str}`、`VmStackWriter::{push_slice,write_bytes}`、`Thread::new_user_vm` |
| `kernel.rs` 来源 | `ProcInit::push_at`，`kernel/src/kernel.rs:4387-4470` |
| 迁移语义 | 所有 stack pointer 下移、字符串长度、指针数组长度和 alignment 计算使用 checked arithmetic；空间不足干净失败。 |
| 修改前 rCore 表达 | 旧 `StackWriter::push_slice` 仍直接 `self.sp -= vs.len() * size_of::<T>()`；B02 已新增 `VmStackWriter` 和 `push_at_in_vm`，真实 `Thread::new_user_vm` 路径通过 `MemorySet` 写栈，但失败仍使用 `expect/panic`。 |
| 接口处理 | 保留 `push_at(stack_top) -> usize` 和 `push_at_in_vm(...) -> usize` 兼容 wrapper；新增 fallible `try_push_at` / `try_push_at_in_vm` 服务真实 exec path，避免把错误通道扩散到 syscall ABI。 |
| 最小修改范围 | `StackWriter`、`VmStackWriter` 的 push helper 和 `ProcInitInfo::{push_at,push_at_in_vm}` 的失败处理。 |
| 不应修改 | auxv key/value ABI、argv/envp 布局顺序。 |
| 已落地结果 | `InitStackWriter` 改为 fallible；`StackWriter` 和 `VmStackWriter` 的 size 乘法、sp 下移、alignment、VA 加法、flush range 都使用 checked path；VM page prepare 失败返回错误而非 panic；`Thread::new_user_vm` 在临时 `MemorySet` 中完成 ELF/stack 装载，成功后替换传入 VM。 |
| 验收结果 | `rustfmt --edition 2018 kernel/src/process/abi.rs kernel/src/process/structs.rs kernel/src/process/thread.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止；未运行 `chaos-tests`。 |
| 状态 | `MIGRATED` |

## B05.2 fork/wait parent-child

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/process/thread.rs:376`、`:439`；`kernel/src/process/proc.rs:245`、`:297`；`kernel/src/syscall/proc.rs:84`、`:97`、`:111`、`:147` |
| rCore 函数 | `Thread::fork`、`Syscall::sys_fork`、`Syscall::sys_wait4` |
| `kernel.rs` 来源 | `TaskTable::{fork_task,reap}`，`kernel/src/kernel.rs:5814-5935`；wait/exit 参考 `:6700-6785` |
| 迁移语义 | fork 只把 child 链接到 parent 一次；wait/reap 从 global process table 和 parent children list 一致清理；指定 pid/group wait filtering 正确。 |
| 修改前 rCore 表达 | `Thread::fork` 构造 new process 并 push 到 parent children；`sys_wait4(pid > 0)` 先查全局 `process(pid)`，在证明该进程是当前进程 child 前即可观察/回收；`pid == 0` 被标为 group wait 但行为等同 any child；`pid < -1` 走 `unimplemented!()`；`Process::exit` 不把仍存活或 zombie children 转交 init。 |
| 接口处理 | 保留 `Thread::fork(&UserContext) -> Arc<Thread>` 和 `sys_wait4(pid, wstatus)` ABI；wait 目标在 syscall 内部用 `WaitFor::{AnyChild,ProcessGroup,Pid}` 表达；孤儿进程转交放入 `Process::reparent_children_to_init` 私有 helper。 |
| 最小修改范围 | `sys_wait4` target filtering、children snapshot、reap cleanup；`Process::exit` 调用私有 helper 处理 children reparent。`Thread::fork` 现有单次 parent-child link 与目标一致，未额外修改。 |
| 不应修改 | `Pid`/`Pgid` public shape、executor spawn 调用面。 |
| 已落地结果 | `sys_wait4` 只从当前进程 `children` 快照判断目标；`pid > 0` 仅匹配当前 child pid，不再先查全局表；`pid == 0` 匹配调用者当前 pgid；`pid < -1` 匹配 `-pid` pgid，非法溢出返回 `EINVAL`；无匹配 child 返回 `ECHILD`；reap 时先写 `wstatus`，再移除 `PROCESSES` 和 parent children 记录；父进程退出时将 children 的 `parent` 改为 pid 1，并把 child 加入 init children list，已退出 child 会唤醒 init wait。 |
| 验收结果 | `rustfmt --edition 2018 kernel/src/syscall/proc.rs kernel/src/process/proc.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止；未运行 `chaos-tests`。 |
| 状态 | `MIGRATED` |

## B05.3 fd lifecycle and cloexec

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/process/proc.rs:80`、`:190`、`:206`、`:212`、`:221`；`kernel/src/process/thread.rs:332`、`:378`；`kernel/src/syscall/fs.rs:310`、`:669`、`:670`、`:900`、`:905`、`:922`、`:1162`、`:1365`；`kernel/src/syscall/proc.rs:201` |
| rCore 函数 | `Process::{add_file,add_file_with_cloexec,close_file,is_fd_cloexec,set_fd_cloexec}`、`Thread::new_user`、`Thread::fork`、`sys_close`、`dup_impl`、`sys_dup3`、`sys_fcntl`、`sys_exec` close-on-exec path |
| `kernel.rs` 来源 | `Task::{close_fd,dup_fd,dup2_fd,set_cloexec}`，`kernel/src/kernel.rs:5736-5801`；exec close-on-exec，`:7436-7475` |
| 迁移语义 | close/dup/cloexec 实际修改 fd table；dup 保持 open-file description sharing；`FD_CLOEXEC` 是 fd-local；exec 关闭所有 cloexec fd。 |
| 修改前 rCore 表达 | `sys_close` remove fd；`dup_impl` 先删目标再取源，导致 `dup2(old, old)` 误关 fd；`sys_fcntl` 只在 `FileLike::File` 上修改 `fd_cloexec`；socket/epoll fd 无 fd-local close-on-exec；`F_DUPFD` 落到默认 `Ok(0)`；exec close loop 只看 file fd。 |
| 接口处理 | 保留 `Process.files: BTreeMap<usize, FileLike>` 和 `FileLike` enum，不把 fd flag 塞进 socket/epoll 对象；新增 `Process.fd_cloexec: BTreeSet<usize>` 作为 fd-local metadata，使 B03 epoll registration 共享和 file open-file description sharing 不受影响。 |
| 最小修改范围 | `Process.files` helper、`sys_close`、`dup_impl`、`sys_fcntl`、`sys_exec` close-on-exec loop；与 B03 `FileLike::dup` 状态共享一致。 |
| 不应修改 | syscall fd numbers/flags ABI、`FileHandle` open-file description sharing。 |
| 已落地结果 | `Process` 增加 fd-local `fd_cloexec` 集合和 helper；fork 继承 fd-local flags；open/pipe/epoll create 写入 close-on-exec；close 清理 fd-local metadata；dup2 同 fd no-op 并校验源 fd；dup3 同 fd或非法 flags 返回 `EINVAL`；`F_DUPFD`/`F_DUPFD_CLOEXEC` 按 arg 查找空 fd；`F_GETFD/F_SETFD` 对 file/socket/epoll 均有效；exec 关闭所有 fd-local cloexec fd。 |
| 验收结果 | `rustfmt --edition 2018 kernel/src/process/proc.rs kernel/src/process/thread.rs kernel/src/syscall/fs.rs kernel/src/syscall/proc.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止；未运行 `chaos-tests`。 |
| 状态 | `MIGRATED` |

## B05.4 ELF bounds

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/process/structs.rs:56`、`:82`、`:132`、`:202`、`:220`；`kernel/src/process/thread.rs:155`、`:168`、`:184` |
| rCore 函数 | `ElfExt::{make_memory_set,append_as_interpreter,get_phdr_vaddr}`、`Thread::new_user_vm` |
| `kernel.rs` 来源 | `validate_elf_header`，`kernel/src/kernel.rs:1950-2022` |
| 迁移语义 | program header virtual/file range 使用 checked arithmetic；farthest memory 计算不 overflow；malformed ELF 返回错误或拒绝加载。 |
| 修改前 rCore 表达 | `ph.virtual_addr() as usize + ph.mem_size() as usize`、`ph.offset() as usize + ph.file_size() as usize` 直接计算；interpreter `+ bias`、PHDR inferred address 和 farthest memory 也直接加；`make_memory_set` 返回 `usize`，错误通道不足。 |
| 接口处理 | 不复制 `kernel.rs` parser；保留 `ElfExt` 和 `MemorySet` loader 形状，但把 `make_memory_set`、`append_as_interpreter` 和 `get_phdr_vaddr` 改为 `Result`，错误在 `Thread::new_user_vm` 收口，再由 `sys_exec` 映射为 `EINVAL`。 |
| 最小修改范围 | `make_memory_set` 和 `append_as_interpreter` 内的 checked range helper；如需签名变化，联动 `Thread::new_user_vm` 调用点。 |
| 不应修改 | ELF crate API、`INodeForMap` read trait、MemorySet handler public shape。 |
| 已落地结果 | 新增 `u64_to_usize` helper；LOAD virtual/file range、`file_size <= mem_size`、interpreter bias、PHDR inferred address、farthest memory 和 bias address 均 checked；无 LOAD 或空 LOAD range 返回错误；`new_user_vm` 使用临时 VM，避免 exec 失败提前破坏旧地址空间。 |
| 验收结果 | `rustfmt --edition 2018 kernel/src/process/abi.rs kernel/src/process/structs.rs kernel/src/process/thread.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止；未运行 `chaos-tests`。 |
| 状态 | `MIGRATED` |

## B05.5 process brk / heap boundary

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/process/proc.rs:73`、`:94`；`kernel/src/process/thread.rs:339`、`:384`；`kernel/src/syscall/mod.rs:241`；`kernel/src/syscall/mem.rs:11` |
| rCore 函数 | `Process` program break 状态、`Thread::new_user` 初始化、`Thread::fork` 继承、`Syscall::sys_brk` |
| `kernel.rs` 来源 | `SYS_BRK` 分支，`kernel/src/kernel.rs:6509-6540`；`VmMap::brk`，`kernel/src/kernel.rs:1100-1108` |
| 迁移语义 | `brk(0)` 返回当前 break；增长时把新增区间接入用户 `MemorySet`；缩小时移除对应 VM 区间；overflow、低于初始 break 和逼近用户栈的请求不破坏当前 break。 |
| 当前 rCore 表达 | 原 `SYS_BRK` 直接走 `unimplemented("brk", Err(SysError::ENOMEM))`；`Process` 没有 program break 状态。 |
| 接口处理 | 不引入 `kernel.rs` 的模拟 `Task`/`VmMap`；在恢复后的 `Process` 上记录 `brk_start/brk`，用 `MemorySet`、`Delay` 和 `GlobalFrameAlloc` 表达 lazy heap mapping。 |
| 已落地结果 | `Process` 增加 `USER_BRK_START`、`brk_start`、`brk`；新进程初始化为 `0x0040_0000`，fork 继承；`SYS_BRK` 分发到 `sys_brk`；增长路径 `vm.push(..., Delay::new(GlobalFrameAlloc), "brk")`，收缩路径 `pop_with_split`。 |
| 验收结果 | `make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；`/tmp/chaos-qemu-brk.log` 到达 busybox shell `/ #`，不再出现 `brk is unimplemented`，并出现 `0x400010/0x401ff0/0x403ff0` 等 brk heap 页 demand fault。 |
| 状态 | `MIGRATED` |

## 接口边界

| `kernel.rs` 结构 | rCore 落点 | 处理方式 |
| --- | --- | --- |
| `ProcInit` | `ProcInitInfo`、`StackWriter` | 保留 rCore stack layout，迁移 checked subtraction 和 failure handling。 |
| `TaskTable::fork_task/reap` | `Thread::fork`、`sys_wait4`、global `PROCESSES` | 保留 rCore process/thread split，迁移 parent-child lifecycle。 |
| `Task` fd helpers | `Process.files`、`sys_close`、`dup_impl`、`sys_fcntl` | 保留 rCore fd table，迁移 fd-local side effects。 |
| `validate_elf_header` | `ElfExt`、`Thread::new_user_vm` | 不复制模拟 parser；迁移 checked PHDR/file range 语义。 |
| `VmMap::brk` / `SYS_BRK` | `Process::{brk_start,brk}`、`Syscall::sys_brk` | 保留 rCore `MemorySet`，迁移 program break 边界和 lazy heap mapping。 |

## 批次内顺序

1. 已按运行证据处理 `brk`，解除 busybox 启动期间的 `brk is unimplemented` warning。
2. 已处理 `ProcInitInfo` stack checked arithmetic 和错误传播。
3. 已处理 ELF bounds，因为它也是 exec 前置。
4. 已处理 fd lifecycle/cloexec，联动 exec close loop 和 B03 epoll dup。
5. 已处理 fork/wait parent-child，收口 process table、wait、eventbus 和 orphan reparent 的核心生命周期。

## 风险和验收

| 风险 | 验收方式 |
| --- | --- |
| 改 `ProcInitInfo::push_at` 签名影响 exec/load 调用链 | 已保留兼容 wrapper，真实 exec path 使用 `try_push_at_in_vm`。 |
| fd-local cloexec 当前只存在 `FileHandle` | 已通过 `Process.fd_cloexec` 落在 fd table 侧，不破坏 B03 epoll/filelike 共享状态。 |
| wait4 group filtering 语义扩大 | 已按 pid = -1/0/>0/<-1 分别记录目标，避免用 AnyChild 覆盖 group wait；`pid < -1` 溢出路径返回 `EINVAL`。 |
| ELF trait 错误通道不足 | 已将 `ElfExt` loader 函数改为 `Result`，错误在 `Thread::new_user_vm` 收口。 |

## 批准前报告摘要

若进入源码修改，需要先向用户报告：

- 文件/行：`kernel/src/process/abi.rs:13`、`:58`；`kernel/src/process/thread.rs:365`、`:425`；`kernel/src/syscall/proc.rs:23`、`:85`、`:151`、`:201`；`kernel/src/process/proc.rs:179`；`kernel/src/syscall/fs.rs:639`、`:862`、`:1317`；`kernel/src/process/structs.rs:78`、`:105`。
- 现象/语义差距：stack/ELF range 直接算术，fork/wait/fd lifecycle 需要与 `kernel.rs` 的 side effect 和 cleanup 语义对齐。
- 根因/当前表达：恢复上游基线保留原接口，但部分边界和生命周期语义没有显式校验。
- 预期行为：stack/ELF 不 wrap；fd close/dup/cloexec/exec side effect 可见；fork/wait parent-child cleanup 一致。
- 最小修改：只改上述函数和必要的私有 helper，保持 rCore public API，若 public signature 必须变化则单独报批。
