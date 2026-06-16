# rCore 耦合方案

本文记录将独立的 `kernel/src/kernel.rs` 教学内核实现与仓库中缺失的 rCore 模块树进行耦合的证据、范围和实施方案。

## 目标

当前仓库里有两部分内核相关代码：

- `kernel/src/kernel.rs`：一个基于 `std` 的独立内核模拟实现，用于理解和验证内核语义。
- `kernel/` 里的其余部分：一个不完整的 rCore 内核树，仍然期望原版 no-std rCore 模块存在。

耦合后的目标不是让 `chaos-tests` 通过。根据最新需求，最终验收应看完整 rCore 项目是否可以正常构建并运行。`chaos-tests` 只作为理解 `kernel.rs` 行为的参考材料，不作为耦合项目的验收门槛。

最终状态应满足：

- rCore 缺失模块被补齐，接口形状与对应版本的原版 rCore 保持兼容。
- 在需要补充行为时，以 `kernel.rs` 的实现语义作为参考，而不是机械地把 `kernel.rs` 拆进 no-std rCore。
- 主要运行验收集中在 `kernel` crate、RISC-V 构建、QEMU 启动和用户程序链路。

## 当前证据

- `chaos-tests/src/lib.rs` 是指向 `../../kernel/src/kernel.rs` 的符号链接。
- 当前 `kernel.rs` 模拟实现可作为行为参考；最近一次可选参考检查结果为：
  - `cargo test --test basic -- --test-threads=1`：33 项通过。
  - `basic` 加 14 个 audit 目标：117 项通过。
- [kernel/src/lib.rs](/home/zcychar/chaos/kernel/src/lib.rs)、[kernel/Cargo.toml](/home/zcychar/chaos/kernel/Cargo.toml)、[kernel/Cargo.lock](/home/zcychar/chaos/kernel/Cargo.lock)、[kernel/Makefile](/home/zcychar/chaos/kernel/Makefile) 和 [kernel/build.rs](/home/zcychar/chaos/kernel/build.rs) 与匹配的上游 rCore 完全一致。
- 当前 rCore 树已经按匹配 commit 恢复缺失模块面；恢复文件与上游对象 hash 比对无 mismatch，该模块面视为可运行基线。

## 匹配的上游版本

使用上游仓库 `https://github.com/rcore-os/rCore.git`，固定到：

```text
66cb4181ec6d3336d507c7c1ff100127f56fcc0a
2023-08-24 21:46:33 +0800
Add maintenance notice
```

选择这个版本的原因是：当前仓库的 rCore manifest、lockfile、build script、Makefile 和 `kernel/src/lib.rs` 与该 commit 完全一致。不要使用 rCore Tutorial v3，也不要使用其他 rCore 派生项目或更新分支作为接口来源。

上游根目录还包含 `rust-toolchain`：

```text
nightly-2020-06-04
```

## 已恢复的 rCore 源码面

当前仓库已从匹配上游恢复这些路径：

```text
crate/memory/
kernel/src/memory.rs
kernel/src/trap.rs
kernel/src/fs/
kernel/src/ipc/
kernel/src/process/
kernel/src/sync/
rust-toolchain
```

匹配上游 commit 中这些源码的大致规模：

| 区域 | 文件数 | 作用 |
| --- | ---: | --- |
| `crate/memory` | 18 | 通用 VM、页表、MemorySet、COW、swap 辅助。 |
| `kernel/src/memory.rs` | 1 | 内核帧分配器、地址转换、用户拷贝、缺页处理门面。 |
| `kernel/src/fs` | 15 | VFS 文件、管道、epoll、devfs、ioctl、fcntl。 |
| `kernel/src/ipc` | 3 | SysV semaphore 和共享内存进程状态。 |
| `kernel/src/process` | 6 | 进程、线程、futex、ABI 栈初始化、ELF 加载辅助。 |
| `kernel/src/sync` | 5 | Mutex、Condvar、Semaphore、EventBus。 |
| `kernel/src/trap.rs` | 1 | tick、timer、串口输入分发。 |

## 恢复记录

这是 rCore 源码缺失导致的恢复记录，不是 `kernel.rs` 中已定位的 bug，也不是后续迁移阶段要反复寻找的接口阻塞。

- 文件/行：[kernel/Cargo.toml](/home/zcychar/chaos/kernel/Cargo.toml:66)
- 恢复前现象：当前树缺少 `crate/memory`，因此不能形成完整 rCore 模块面：

```text
failed to get `rcore-memory` as a dependency of package `rcore v0.2.0`
unable to update /home/zcychar/chaos/crate/memory
failed to read `/home/zcychar/chaos/crate/memory/Cargo.toml`
No such file or directory
```

- 根因：`rcore-memory = { path = "../crate/memory" }` 指向一个本地 crate，但当前仓库没有包含 `crate/memory`。
- 预期行为：本地 `crate/memory` 存在，并提供 `kernel/src/memory.rs` 需要 re-export 的 `rcore_memory` API。
- 最小恢复方案：从上游 rCore commit `66cb4181ec6d3336d507c7c1ff100127f56fcc0a` 恢复 `crate/memory/`，并同时恢复 `kernel/src/lib.rs` 已声明但当前缺失的 rCore 模块。恢复后的上游模块面视为正确基线。
- 当前状态：用户已确认方案，本轮已按固定上游恢复这些路径；恢复后逐文件 hash 校验无 mismatch。

## 为什么不能直接拆分 kernel.rs

`kernel.rs` 是一个基于 `std` 的模拟内核。它使用 `std::sync::Mutex`、`RwLock`、`Condvar`、`std::thread` 和普通集合类型。真实 rCore 内核是 no-std 架构，依赖 `alloc`、spin lock、trapframe、体系结构页表、async 调度和 `rcore-memory` crate。

所以不能机械地把 `kernel.rs` 拆成 rCore 模块。正确路径是：

1. 先从匹配上游恢复 rCore 模块接口。
2. 保持 rCore 既有架构和调用面。
3. 在需要补齐行为时，把 `kernel.rs` 中已经明确的语义、边界条件和不变量移植到恢复后的 rCore 模块里。

## kernel.rs 行为参考

使用 [docs/kernel-map.md](/home/zcychar/chaos/docs/kernel-map.md) 作为 `kernel.rs` 的行号范围地图。注意：这不是最终验收门槛，而是理解 `kernel.rs` 行为的参考。

`kernel.rs` 每个行段对应恢复后 rCore 哪些文件、接口差异如何处理，见 [docs/kernel-to-rcore-map.md](/home/zcychar/chaos/docs/kernel-to-rcore-map.md)。

更细的 rCore API、调用面和迁移对齐点见 [docs/rcore-interface-map.md](/home/zcychar/chaos/docs/rcore-interface-map.md)。

每个新增文件/文件夹的分层迁移记录见 [docs/migration-records/README.md](/home/zcychar/chaos/docs/migration-records/README.md)。

Phase 1 的精确恢复文件列表、上游 blob hash 和执行步骤见 [docs/rcore-restore-manifest.md](/home/zcychar/chaos/docs/rcore-restore-manifest.md)。

| `kernel.rs` 范围 | 行为区域 | rCore 目标模块 |
| --- | --- | --- |
| 1-428 | 常量、全局锁、事件声明 | `consts`、`sync`、syscall 常量。 |
| 429-904 | 同步队列、semaphore、futex table | `kernel/src/sync`、`kernel/src/process/futex.rs`。 |
| 916-1689 | 地址辅助、物理帧、VM map、用户拷贝 | `crate/memory`、`kernel/src/memory.rs`。 |
| 1692-2205 | heap、slab、ELF、checksum、辅助算法 | `memory`、`process/structs.rs`、util 辅助。 |
| 2208-2991 | 文件句柄、管道、FileLike、epoll、终端 | `kernel/src/fs`。 |
| 2992-3214 | channel | `fs/devfs/tty.rs`、串口输入路径、pipe-like blocking。 |
| 3216-4154 | page cache、注册表、block cache、mount、磁盘队列 | `fs`、`drivers`、block cache 集成。 |
| 4158-4386 | IPC 权限、semaphore、共享内存 | `kernel/src/ipc`。 |
| 4387-4776 | 进程初始化、capability、signal、timer | `process`、`signal`、`trap`。 |
| 4777-5191 | context、trap、clock、serial 辅助 | `arch/*/interrupt`、`trap.rs`。 |
| 5194-6054 | scheduler、run queue、task、task table | `kernel/src/process/thread.rs`、async executor。 |
| 6055-7528 | 内核门面和 syscall 行为 | `kernel/src/syscall/*`。 |
| 7529-7659 | 访问校验和工具函数 | `memory.rs`、syscall user access helper。 |
| 7662-8071 | 地址空间、进程组、wait queue、resource limit | `process`、`syscall/proc.rs`、resource 路径。 |
| 8072-8338 | bit 工具和 buddy allocator | `crate/memory`、allocator 辅助。 |

## 分阶段实施方案

### Phase 1：恢复 rCore 可运行基线

已从匹配上游恢复缺失文件：

- `crate/memory/`
- `kernel/src/memory.rs`
- `kernel/src/trap.rs`
- `kernel/src/fs/`
- `kernel/src/ipc/`
- `kernel/src/process/`
- `kernel/src/sync/`
- `rust-toolchain`

未覆盖当前已经与上游一致的文件。未修改 `kernel/src/kernel.rs`。

主要基线验收命令：

```bash
cd kernel && cargo check
cd kernel && make build ARCH=riscv64
cd kernel && make run ARCH=riscv64 GRAPHIC=off
```

结果：当前仓库已恢复为匹配上游的完整 rCore 模块面。后续不以“找接口阻塞”为目标，而是在这个可运行基线上做 `kernel.rs` 语义迁移。

### Phase 2：建立显式迁移地图

对每个恢复模块记录两类信息：

- 必须保持的 rCore 兼容 API。
- 需要从 `kernel.rs` 迁移或确认不迁移的行为不变量。

高风险映射如下：

| rCore 模块 | `kernel.rs` 参考 | 需要对齐的语义 |
| --- | --- | --- |
| `sync/condvar.rs`、`sync/event_bus.rs` | `SyncQueue`、`EvBus` | 无 lost wakeup、清理 stale waiter、timeout 后清理等待者。 |
| `process/futex.rs` | `FutexTable` | wake 返回值等于实际移除/唤醒的等待者数量。 |
| `crate/memory/src/*`、`kernel/src/memory.rs` | `VmMap`、`FramePool`、`SharedPage` | checked arithmetic、无 refcount underflow、正确 user/kernel 边界。 |
| `fs/file.rs`、`fs/pipe.rs`、`fs/epoll.rs` | `FHandle`、`PipeNode`、`EpInst` | 共享 offset、append 语义、端点生命周期、epoll dup/removal 状态。 |
| `ipc/*` | `SemCtx`、`ShmCtx` | ID 复用安全、sem undo replay、existing-key size 检查。 |
| `process/*`、`syscall/proc.rs` | `Task`、`TaskTable`、`Kernel::dispatch_syscall` | parent-child link、wait filtering、pgid/session 不变量。 |
| `trap.rs`、`signal/*` | `TrapCtl`、`SigSet`、`TimerWheel` | timer overflow safety、signal range、uncatchable signal 规则。 |

### Phase 3：按批准批次迁移对齐语义

每个批次遵守仓库工作流：

1. 从 [docs/kernel-to-rcore-map.md](/home/zcychar/chaos/docs/kernel-to-rcore-map.md) 选择一个 `kernel.rs` 语义责任。
2. 明确它在恢复后 rCore 中的落点、当前接口表达方式、需要迁移的差异和不迁移的理由。
3. 报告目标文件/位置、迁移语义、预期对齐结果和最小修改范围。
4. 等待批准。
5. 修改最小相关模块集合。
6. 运行对应的真实 rCore 构建/运行验收，确认迁移没有破坏上游基线。

建议顺序：

1. `sync` 和 futex。
2. memory、VM、COW、user access。
3. file、pipe、epoll、terminal/device fs。
4. IPC semaphore 和 shared memory。
5. signal、timer、trap。
6. process/thread/wait/scheduler。
7. syscall facade 集成。
8. allocator 和 utility 边界情况。

### Phase 4：QEMU 和用户态集成

迁移批次完成后，进入真实系统运行验收：

```bash
cd kernel && make build ARCH=riscv64
cd kernel && make run ARCH=riscv64 GRAPHIC=off
```

在依赖 QEMU 结果之前，需要核对 `user/` 和 `rboot/` 是否与匹配上游 submodule 指针一致：

```text
upstream user  = 1c5e883fcfb0dc18895dce7b1931d7cf3a4261b1
upstream rboot = ea29a73dcf579fcf4215542423a60f75f4244d37
```

当前本地 `user/` 和 `rboot/` 是展开后的仓库，HEAD 为：

```text
fdac080430c584d227b53d11c34b551ed14471cf
```

这个差异需要在 full-system 运行验收前处理，或者明确接受它带来的不确定性。

## 验证矩阵

主验收以 rCore 系统构建和运行为准：

```bash
cd kernel && cargo check
cd kernel && make build ARCH=riscv64
cd kernel && make run ARCH=riscv64 GRAPHIC=off
```

如需构建用户镜像或进一步验证用户态链路，再补充：

```bash
cd user && make sfsimg PREBUILT=1 ARCH=riscv64
cd kernel && make run ARCH=riscv64 GRAPHIC=off
```

`chaos-tests` 不是耦合项目验收门槛。只有在修改 `kernel/src/kernel.rs`，或者需要确认 `kernel.rs` 参考行为没有被破坏时，才可选运行：

```bash
cd chaos-tests && cargo test --test basic
```

## 主要风险

- `kernel.rs` 的模拟测试通过不等于 no-std rCore 可以启动。
- 匹配的 rCore 基线较旧，依赖 `nightly-2020-06-04`。
- 受限网络或代理环境可能在源码问题前先暴露依赖拉取问题。
- 恢复上游模块只解决接口完整性，不等于功能语义已经完全对齐。
- `user/` 和 `rboot/` 当前不在匹配上游的 submodule commit 上，full-system 运行证据需要谨慎解释。

## 下一步批准点

下一次源码变更不再是恢复上游文件，而应从 [docs/migration-records/README.md](/home/zcychar/chaos/docs/migration-records/README.md) 中选择一个具体迁移项。批准应明确覆盖：

```text
目标文件/位置
迁移语义
当前 rCore 表达方式
预期对齐结果
最小修改范围
```
