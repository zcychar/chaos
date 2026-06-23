# B06 deferred/no-direct 详细迁移记录

本文记录 B06：恢复路径中没有直接 `kernel.rs` 同名迁移落点，或只作为上游基线/验证参考保留的内容。它的目的不是忽略这些路径，而是明确“不迁移什么、为什么不迁移、后续何时重新评估”。

当前状态：只记录，不修改源码。

## 批次定位

| 项目 | 内容 |
| --- | --- |
| 批次 | B06 deferred/no-direct |
| 恢复路径 | `crate/memory/src/swap/*`、`crate/memory/src/paging/mock_page_table.rs`、`kernel/src/fs/devfs/random.rs`、`kernel/src/fs/devfs/fbdev.rs`、`rust-toolchain` |
| `kernel.rs` 来源关系 | 模拟 cache/allocator、VM test model、设备节点或工具链环境，没有需要复制的同名真实 rCore 模块 |
| 接口原则 | 保持上游恢复内容；只有真实 rCore 调用面出现需求时，才从 `kernel.rs` 抽取语义，不主动发明接口 |

## B06.1 `crate/memory/src/swap/*`

| 项目 | 内容 |
| --- | --- |
| 路径 | `crate/memory/src/swap/mod.rs`、`fifo.rs`、`enhanced_clock.rs`、`mock_swapper.rs` |
| rCore 职责 | `SwapManager`、`Swapper`、`SwapExt` 和测试/mock swapper。 |
| `kernel.rs` 关联 | `kernel.rs` 中有 cache/allocator/utility 概念，但没有真实 rCore 当前运行所需的同名 swap subsystem 迁移。 |
| 处理结论 | 保持上游基线，不从 `kernel.rs` 迁移模拟 cache/allocator 结构到 swap。 |
| 重新评估条件 | 后续真实 rCore 启用 swap 或 page replacement 运行路径时，再按 `SwapManager` trait 单独建立迁移项。 |
| 状态 | `NO_DIRECT_PORT` |

## B06.2 `crate/memory/src/paging/mock_page_table.rs`

| 项目 | 内容 |
| --- | --- |
| 路径 | `crate/memory/src/paging/mock_page_table.rs` |
| rCore 职责 | mock page table、mock entry、fault handler，用于测试或行为模型。 |
| `kernel.rs` 关联 | 可作为 `VmMap`/page fault 行为参考，但不进入真实内核运行路径。 |
| 处理结论 | 保持上游；不为真实 rCore 构建迁移 `kernel.rs` 模拟页表。 |
| 重新评估条件 | 需要为迁移行为新增 crate-level 测试时，可参考 mock page table 编写验证，但不作为运行代码迁移。 |
| 状态 | `NO_DIRECT_PORT` |

## B06.3 `kernel/src/fs/devfs/random.rs`

| 项目 | 内容 |
| --- | --- |
| 路径 | `kernel/src/fs/devfs/random.rs` |
| rCore 职责 | `/dev/random` 或等价 random devfs 节点。 |
| `kernel.rs` 关联 | `kernel.rs` 没有需要直接落到 random devfs 的核心迁移语义。 |
| 处理结论 | 保持上游设备节点；不从 `kernel.rs` 添加随机源模拟结构。 |
| 重新评估条件 | 如果真实运行验收发现 random 设备行为影响用户程序，再按设备语义单独记录。 |
| 状态 | `NO_DIRECT_PORT` |

## B06.4 `kernel/src/fs/devfs/fbdev.rs`

| 项目 | 内容 |
| --- | --- |
| 路径 | `kernel/src/fs/devfs/fbdev.rs` |
| rCore 职责 | framebuffer devfs 节点和 ioctl 结构。 |
| `kernel.rs` 关联 | `kernel.rs` 没有 framebuffer 对应模拟模块。 |
| 处理结论 | 保持上游 framebuffer 设备，不迁移同名结构。 |
| 重新评估条件 | 如果图形运行验收或用户程序依赖 fbdev，再按 rCore 设备层接口单独记录。 |
| 状态 | `NO_DIRECT_PORT` |

## B06.5 `rust-toolchain`

| 项目 | 内容 |
| --- | --- |
| 路径 | `rust-toolchain` |
| rCore 职责 | 固定上游工具链 `nightly-2020-06-04`。 |
| `kernel.rs` 关联 | 无运行语义迁移；只保证恢复后 rCore 基线环境与上游一致。 |
| 处理结论 | 保持上游工具链 pin。 |
| 重新评估条件 | 若用户明确要求升级工具链或当前环境无法构建，再单独报批。 |
| 状态 | `BASELINE_RESTORED` |

## no-direct 原则

| 情况 | 处理 |
| --- | --- |
| `kernel.rs` 有模拟结构，但 rCore 有真实外部 crate/driver/设备层实现 | 不复制模拟结构，只迁移可验证语义。 |
| 上游文件是测试/mock/辅助实现 | 保持上游，作为验证参考，不作为真实运行路径迁移。 |
| 工具链/manifest 文件 | 作为基线记录，不承载 `kernel.rs` 行为迁移。 |

## 结论

B06 的路径均已在 [restored-tree.md](../restored-tree.md)、[path-traceability.md](../path-traceability.md)、[function-index.md](../function-index.md) 和对应模块文档中登记。它们当前不需要源码修改批准；后续只有出现真实运行调用面需求时，才重新拆分为新的迁移批次。
