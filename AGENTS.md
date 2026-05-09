# Repository Guidelines

## Project Structure

This is an rCore-derived teaching OS with a Rust kernel and C/Rust user programs. The main debugging target is `kernel/src/kernel.rs`. The test crate is `chaos-tests/`; `chaos-tests/src/lib.rs` points at `../../kernel/src/kernel.rs`, so test runs compile the kernel simulation directly. User programs are in `user/`, loadable module examples in `modules/hello_rust/`, bootloader code in `rboot/`, and helper scripts/configuration in `tools/` and `tests/`.

## Build and Test Commands

Run commands from the listed directory:

- `cd chaos-tests && cargo test --test basic`: run the visible basic suite.
- `cd chaos-tests && cargo test --test basic -- group_01`: run one group while debugging.
- `cd chaos-tests && cargo test --test advanced` and `cargo test --test pressure`: run grading suites when present.
- `cd kernel && make build ARCH=riscv64`: build the default RISC-V kernel image.
- `cd kernel && make run ARCH=riscv64 GRAPHIC=off`: run the kernel in QEMU.
- `cd kernel && make clean`: remove kernel and user build outputs.

## Coding Style

Rust uses edition 2018 in `kernel/` and 2021 in `chaos-tests/`. Follow the existing single-file layout while correctness is unstable, use 4-space indentation, and run `cargo fmt` before submitting Rust changes. Keep comments focused on invariants, safety assumptions, and non-obvious OS behavior.

## Bug-Fix Workflow

Before editing code for a located bug, stop and report: file/line, failing symptom or test, root cause, expected behavior, and proposed minimal fix. Wait for user approval before applying that code change. Fix compile blockers first, but report each blocker the same way before patching it.

## Coverage and Recheck Policy

Visible tests are insufficient. After test-driven fixes, perform a module-by-module manual recheck of `kernel/src/kernel.rs`, including sync, memory/VM, files/pipes/epoll, terminal/channel, cache/mount/disk, IPC, signals/timers, trap/context, scheduler/tasks, syscall facade, process groups/wait/resources, and allocator/utilities. Track line ranges, tests, upstream references, and invariants in `docs/kernel-map.md`.

## Commits and Disclosure

Use short focused commit messages, such as `kernel: fix scheduler wakeup`. Pull requests should include the problem, implementation summary, tests run, and linked task. The README requires disclosure for AI-assisted work; preserve complete agent dialogue logs and clearly annotate human-written versus agent-generated or suggested code.
