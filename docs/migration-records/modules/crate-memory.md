# `crate/memory` 迁移记录

## 模块定位

`crate/memory` 是恢复后的 `rcore-memory` crate，承担真实 rCore 的通用 VM 抽象、页表 trait、MemorySet、COW、handler 和 swap 辅助。它对应 `kernel.rs` 中的地址、frame、VM region/map、COW/shared page、address space 和 allocator 辅助语义。

当前状态：B02 已对 `cow.rs`、`memory_set/mod.rs`、`handler/file.rs`、`handler/byframe.rs` 做部分源码迁移；其余文件保持上游基线或等待后续批次。当前运行验证不是 `chaos-tests`，而是 RISC-V QEMU 启动：已进入 busybox shell `/ #`。

| 对应 `kernel.rs` 行段 | 迁移主题 |
| --- | --- |
| `916-1689` | 地址辅助、frame、VM map、用户拷贝、COW。 |
| `7662-8071` | `AddrSpace`、split/unmap/fork、资源边界。 |
| `8072-8338` | bit/allocator 辅助语义。 |

## 子目录记录

| 子目录 | 职责 | 迁移状态 |
| --- | --- | --- |
| `crate/memory/src/` | crate 根 API、地址、COW、MemorySet、paging。 | `PARTIAL_MIGRATED` |
| `crate/memory/src/memory_set/` | VM area、attr、map/unmap、page fault。 | `PARTIAL_MIGRATED` |
| `crate/memory/src/memory_set/handler/` | 各类映射 backing handler。 | `PARTIAL_MIGRATED` |
| `crate/memory/src/paging/` | 页表抽象和 mock 页表。 | `MIGRATION_PENDING` |
| `crate/memory/src/swap/` | swap 扩展，当前不作为首批真实运行迁移目标。 | `NO_DIRECT_PORT` |

## 文件-功能记录

| 文件 | 主要结构/函数 | `kernel.rs` 对齐语义 | 迁移记录 |
| --- | --- | --- | --- |
| `Cargo.toml` | crate manifest、依赖声明。 | 无运行语义迁移。 | 保持上游依赖形状，作为 `kernel/Cargo.toml` 的 path dependency。 |
| `src/lib.rs` | `VMError`、`VMResult`、模块导出。 | 模拟 VM helper 的错误通道。 | 不新增错误类型；迁移时把边界失败映射到 `VMError` 或调用方 `SysError`。 |
| `src/addr.rs` | `PhysAddr`、`VirtAddr`、`Page`、`PageRange`。 | `VmRegion::end`、range 半开区间、alignment。 | 迁移重点是 checked page rounding；相邻 range 不应被视为重叠。 |
| `src/cow.rs` | `CowExt`、`FrameRcMap`、COW fault。 | `SharedPage::fault`、source refcount 不 underflow。 | 已将 refcount increase 改为 saturating，将 decrease 改为 bool helper，缺失/0 count 不 underflow；完整 fork/COW 生命周期仍需后续运行核对。 |
| `src/no_mmu.rs` | no-MMU `MemorySet`、`MemoryArea`。 | `AddrSpace` 的非页表等价物。 | 当前真实 rCore 路径优先 MMU；仅记录，不做首批迁移。 |
| `src/memory_set/mod.rs` | `MemoryArea`、`MemoryAttr`、`MemorySet::{push,pop,insert,remove,handle_page_fault}`。 | `VmMap::insert/remove/split`、`AddrSpace::fork_from/unmap_range`。 | 已迁移 checked align、checked pointer/range、find/push/pop/split 早返回和 `with<R>`；后续仍需按具体 VM 生命周期路径继续核对。 |
| `src/memory_set/handler/mod.rs` | `AccessType`、`MemoryHandler`、`FrameAllocator`。 | page fault access 类型、`FramePool` 接口。 | 保持 trait 形状；迁移 checked alloc/contiguous alignment 语义到实现方。 |
| `src/memory_set/handler/byframe.rs` | `ByFrame<T>`、per-page frame map。 | 匿名 VM page/frame backing。 | 已对 fault path 增加 present + access check，已映射页只有权限满足才返回 true。 |
| `src/memory_set/handler/delay.rs` | `Delay<T>` lazy allocation。 | demand paging、fault-time allocation。 | 迁移目标是 fault 边界明确，非法 access 不隐式创建页。 |
| `src/memory_set/handler/file.rs` | `File<F,T>`、`Read` trait、file-backed page fault。 | `FLike::mmap_fl`、file offset/length overflow。 | 已迁移 file offset/read size checked arithmetic、非法 offset zero-fill、allocator failure false；已补非页对齐 ELF LOAD 的页内交集读取，避免 `.init_array` 被清零。 |
| `src/memory_set/handler/linear.rs` | `Linear` direct mapping。 | `p2v/v2p/k_off`、direct map。 | 保持上游 direct mapping；只对齐边界计算语义。 |
| `src/memory_set/handler/shared.rs` | `SharedGuard<T>`、`Shared<T>`。 | `ShmCtx`、`SharedPage` 生命周期。 | 重点是 shared frame 生命周期、drop 释放、attach/detach 不 underflow。 |
| `src/paging/mod.rs` | `PageTable`、`Entry`、`PageTableExt`。 | page table permission、map/unmap contract。 | 保持 trait；迁移语义落到 arch paging 和 MemorySet 调用。 |
| `src/paging/mock_page_table.rs` | `MockPageTable`、`MockEntry`、fault handler。 | `VmMap` 行为测试模型。 | 仅作为行为验证参考；不进入真实内核运行路径。 |
| `src/swap/mod.rs` | `SwapManager`、`Swapper`、`SwapExt`、`SwapError`。 | 模拟 cache/allocator 辅助，无直接当前目标。 | 记录为上游恢复内容，非首批迁移目标。 |
| `src/swap/fifo.rs` | `FifoSwapManager`。 | 无直接迁移。 | 保持上游。 |
| `src/swap/enhanced_clock.rs` | `EnhancedClockSwapManager`。 | 无直接迁移。 | 保持上游。 |
| `src/swap/mock_swapper.rs` | `MockSwapper`。 | 无直接迁移。 | 保持上游。 |

## 待批准迁移候选

| 优先级 | 位置 | 迁移语义 | 最小范围 |
| --- | --- | --- | --- |
| 已完成部分 | `src/memory_set/mod.rs` | 所有 region/page range 计算使用 checked arithmetic；半开区间保持一致。 | 基础 range helper 已迁移；运行边界转向硬件页表可见性。 |
| 已完成部分 | `src/cow.rs` | COW refcount 不 underflow，fork/source frame 计数只增减一次。 | refcount 边界已迁移；完整生命周期后续核对。 |
| 已完成部分 | `src/memory_set/handler/file.rs` | file-backed fault 的 offset/length 计算不 overflow，非页对齐 LOAD 正确页内填充。 | offset/read-size 边界和 page/file 交集读取已迁移；QEMU 已进入 busybox shell。 |
| P2 | `src/memory_set/handler/shared.rs` | shared memory attach/drop 生命周期与 `kernel.rs` 的 `ShmCtx` 对齐。 | `SharedGuard`、`Shared`。 |
