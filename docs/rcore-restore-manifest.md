# rCore 恢复清单

本文固定 Phase 1 从上游恢复的精确文件，并记录当前恢复结果。当前这些文件已经恢复到工作区，且本地文件 hash 与上游对象一致。

## 上游固定版本

```text
repository: https://github.com/rcore-os/rCore.git
commit:     66cb4181ec6d3336d507c7c1ff100127f56fcc0a
date:       2023-08-24 21:46:33 +0800
subject:    Add maintenance notice
```

用于生成本文的本地参考克隆：

```text
/tmp/rcore-upstream
```

## 恢复范围

恢复范围固定为匹配上游中原先缺失的 rCore 模块面：

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

Phase 1 不覆盖这些当前已经与上游固定版本一致的文件：

```text
kernel/Cargo.toml
kernel/Cargo.lock
kernel/Makefile
kernel/build.rs
kernel/src/lib.rs
```

Phase 1 不修改 `kernel/src/kernel.rs`。

## Tree 对象

```text
040000 tree f7173fb0c03535376e82c4c1996418ea2f29b2e7 crate
040000 tree 72d06f6b2ca05c1bea770388292636a00b939984 kernel/src/fs
040000 tree 3cad9750c5d4812d34a871465609f60de266f076 kernel/src/ipc
100644 blob 551439eece7594e89a101c86000474b63681d233 kernel/src/memory.rs
040000 tree 3e963066ab6f6ae781aa5e075c06908f4627c9a2 kernel/src/process
040000 tree 125ef96b961aff502434e926b9ec5f591063030d kernel/src/sync
100644 blob 67d8b08d85b5317d20399cc8366c53102304734b kernel/src/trap.rs
100644 blob 039471e8c72711468d14c46b7fafb44eab0d6e01 rust-toolchain
```

## Blob 清单

### `crate/memory`

```text
100644 15007b051b47476e3d96d1997f6c896e882eda62 crate/memory/Cargo.toml
100644 f03610392ca08a276e3170bf1acd3fe488b88efb crate/memory/src/addr.rs
100644 3735cdaf73d2e735f197f9c73e34bc55773ce07c crate/memory/src/cow.rs
100644 7969ad308f9f0a4e2f44b3c6cdc814fbc79178da crate/memory/src/lib.rs
100644 202287a1bef49c6bf958bdf015a902a166f61005 crate/memory/src/memory_set/handler/byframe.rs
100644 fa66043a02842e50fc22396b0964386c6fbf589f crate/memory/src/memory_set/handler/delay.rs
100644 55871896b260abb99d15c6848bf51409acbc464a crate/memory/src/memory_set/handler/file.rs
100644 a32ea798789844a96399f33aa87c435dbc8be788 crate/memory/src/memory_set/handler/linear.rs
100644 a7b5d9fcb8e3cb4e13a9daa18540da4420fc8c7c crate/memory/src/memory_set/handler/mod.rs
100644 e9ee61db791eb075f2436fb95666d9277a240ec7 crate/memory/src/memory_set/handler/shared.rs
100644 a408331705f7a6be557dbe880ad7756868e321b7 crate/memory/src/memory_set/mod.rs
100644 e1d128f093ffabe7cc12bf7a1082a3a3d6d69f0b crate/memory/src/no_mmu.rs
100644 da69d97056bd1c361a001d16608f00f49da26a8b crate/memory/src/paging/mock_page_table.rs
100644 777cee78d76f56c50576bab97226c7aeabd23edf crate/memory/src/paging/mod.rs
100644 2651ba757f9c1d9df68da0af47be6cf4df77a013 crate/memory/src/swap/enhanced_clock.rs
100644 70e67b9d64161733d00515a1744458e763e932a9 crate/memory/src/swap/fifo.rs
100644 0301abd9120d9fa3e60b62becae8084e629f690d crate/memory/src/swap/mock_swapper.rs
100644 3451df884aa2b1d701b9a69301e67a7b883b6e7e crate/memory/src/swap/mod.rs
```

### `kernel/src/fs`

```text
100755 e4650ac3b4dc046dc9cb0bf6eb563a67d2dcea74 kernel/src/fs/devfs/fbdev.rs
100644 4d7dfdcc970a848b47938d561a3cfc3b64deff7d kernel/src/fs/devfs/mod.rs
100644 425774e0fe36127b41e37c381818048c7b8b1ae3 kernel/src/fs/devfs/random.rs
100644 39b9e8e953a7c2495b0045fb88924bf102d1a6bd kernel/src/fs/devfs/serial.rs
100644 780a06c6d1f5a88591de23dbbab138758ca02458 kernel/src/fs/devfs/shm.rs
100644 1eb0218ed721c2acc399c2f86fd3b933b78a993b kernel/src/fs/devfs/tty.rs
100644 bb8e860d377fa0b14cbcbeca6fbc4885f5037144 kernel/src/fs/device.rs
100644 fbf74476dd12b9cfd8501a751eca45c9cac8d144 kernel/src/fs/epoll.rs
100644 4908628be404133b8356107ca38c0facec430e21 kernel/src/fs/fcntl.rs
100644 f286fd001181b68ab95c26a2cdcfa004f82f69e1 kernel/src/fs/file.rs
100644 166e278353e5da4ee0432d0d3a3560170d919fa3 kernel/src/fs/file_like.rs
100644 8a797b9377ba8c271a247e6978f029ce6906f20d kernel/src/fs/ioctl.rs
100644 eddebd61b73caaa9caa3514814aa5fb0cf5e43d1 kernel/src/fs/mod.rs
100644 809c7cd39e72058c505ff6171bff6d469b14087b kernel/src/fs/pipe.rs
100644 8ae3b4acfc0052623961b0247f9bf283fbfa0fee kernel/src/fs/pseudo.rs
```

### `kernel/src/ipc`

```text
100644 ed76b780df0bd000816599543f3bc408934448ab kernel/src/ipc/mod.rs
100644 5335e41eef9bf93104508ece771cea55c3ba3aa5 kernel/src/ipc/semary.rs
100644 96bca7e5a4b7600fe917a4926e69986d7a65ef67 kernel/src/ipc/shared_mem.rs
```

### `kernel/src/process`

```text
100644 fa79b0015231dd9bb912e8ab4ae157fef4a85954 kernel/src/process/abi.rs
100644 8bcd447cf2f5578749e8b909dc6a368543dd73b8 kernel/src/process/futex.rs
100644 0366db199aa56ac1d8b8cb9060641c7a9df59d7c kernel/src/process/mod.rs
100644 dc1bfce4ed0182e5f3b1a527e15c22f48b3e1b0e kernel/src/process/proc.rs
100644 a44f752615bb0cff9abd488d14cde2784d2e8d30 kernel/src/process/structs.rs
100644 adb598eeba2601d46e8aed87ce75f226293a48b7 kernel/src/process/thread.rs
```

### `kernel/src/sync`

```text
100644 e2fd8948fe792ff6797b758ee9bd66189b7769f0 kernel/src/sync/condvar.rs
100644 263ebdac0f4e2a202af90cd96f5d4b61a2eddf2c kernel/src/sync/event_bus.rs
100644 12acc5e05655a9f349c585453e30082877efce69 kernel/src/sync/mod.rs
100644 7f5e813514b60ab4179df426652dd8b73d9c2b62 kernel/src/sync/mutex.rs
100644 b63fc7e0d55a6e14d9513804ec8eee3d21d3a1f9 kernel/src/sync/semaphore.rs
```

### 单文件

```text
100644 551439eece7594e89a101c86000474b63681d233 kernel/src/memory.rs
100644 67d8b08d85b5317d20399cc8366c53102304734b kernel/src/trap.rs
100644 039471e8c72711468d14c46b7fafb44eab0d6e01 rust-toolchain
```

## 已执行的恢复流程

用户明确确认方案后，本节命令已执行。

1. 从固定上游 checkout 恢复文件：

```bash
git -C /tmp/rcore-upstream archive 66cb4181ec6d3336d507c7c1ff100127f56fcc0a crate/memory kernel/src/memory.rs kernel/src/trap.rs kernel/src/fs kernel/src/ipc kernel/src/process kernel/src/sync rust-toolchain | tar -x -C /home/zcychar/chaos
```

2. 确认没有意外覆盖当前 tracked 文件：

```bash
git status --short
```

3. 确认恢复文件与 manifest 对应的上游对象一致：

```bash
git -C /tmp/rcore-upstream ls-tree -r 66cb4181ec6d3336d507c7c1ff100127f56fcc0a crate/memory kernel/src/memory.rs kernel/src/trap.rs kernel/src/fs kernel/src/ipc kernel/src/process kernel/src/sync rust-toolchain
```

4. 可选第一层 rCore 基线检查：

```bash
cd kernel && cargo check
```

5. 进入 rCore 运行验收：

```bash
cd kernel && make build ARCH=riscv64
cd kernel && make run ARCH=riscv64 GRAPHIC=off
```

## 预期结果

恢复前缺失证据：

```text
failed to read `/home/zcychar/chaos/crate/memory/Cargo.toml`
```

已在恢复 `crate/memory` 后消失。本文不把后续工作定义为“寻找接口阻塞”；完全按上游恢复后的模块面视为正确基线，后续重点是按 [docs/kernel-to-rcore-map.md](/home/zcychar/chaos/docs/kernel-to-rcore-map.md) 和 [docs/migration-records/README.md](/home/zcychar/chaos/docs/migration-records/README.md) 做 `kernel.rs` 语义迁移和对齐。
