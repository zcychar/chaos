# `rust-toolchain` 迁移记录

## 文件定位

`rust-toolchain` 是恢复自上游 rCore 的工具链固定文件，内容为：

```text
nightly-2020-06-04
```

它没有 `kernel.rs` 中的功能语义对应项，但它是恢复后 rCore 可运行基线的一部分。真实 rCore 代码使用旧 nightly feature，例如 `llvm_asm`、`naked_functions`、`const_fn` 等，不能随意升级工具链后再把编译结果解释为迁移问题。

## 迁移记录

| 项目 | 内容 |
| --- | --- |
| 新增路径 | `rust-toolchain` |
| 来源 | rCore commit `66cb4181ec6d3336d507c7c1ff100127f56fcc0a` |
| 对应 `kernel.rs` 语义 | 无直接对应。 |
| 迁移状态 | `BASELINE_RESTORED` |
| 后续处理 | 保持上游工具链约束，除非单独批准工具链升级。 |

## 注意事项

- 如果后续运行 `cargo` 命令触发 toolchain 安装或下载，这属于环境问题，不应被记录为 `kernel.rs` 语义迁移项。
- 若需要升级 Rust toolchain，应作为独立工程任务处理，不能和 `kernel.rs` 迁移混在同一批次。

