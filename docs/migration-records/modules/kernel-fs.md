# `kernel/src/fs` 迁移记录

## 模块定位

`kernel/src/fs` 是恢复后 rCore 的 VFS、文件句柄、pipe、epoll、devfs 和 ioctl/fcntl 层。它对应 `kernel.rs` 中 `FHandle`、`PipeNode`、`FLike`、`EpInst`、terminal/channel、mount/cache/disk 概念中的真实 rCore 文件系统部分。

| 对应 `kernel.rs` 行段 | 迁移主题 |
| --- | --- |
| `2208-2991` | file handle、pipe、FileLike、epoll、terminal/ioctl。 |
| `2992-3214` | channel 语义落到 TTY/serial/pipe-like blocking。 |
| `3216-4154` | mount/cache/device 概念，真实 rCore 主要由 rCore-fs 和 drivers 承担。 |

## 子目录记录

| 子目录 | 职责 | 迁移状态 |
| --- | --- | --- |
| `kernel/src/fs/` | VFS facade、file、pipe、epoll、fcntl/ioctl、pseudo/device。 | `PARTIAL_MIGRATED` |
| `kernel/src/fs/devfs/` | devfs 下的 TTY、serial、random、shm、fbdev。 | `PARTIAL_MIGRATED` |

## 文件-功能记录

| 文件 | 主要结构/函数 | `kernel.rs` 对齐语义 | 迁移记录 |
| --- | --- | --- | --- |
| `mod.rs` | `ROOT_INODE`、`FOLLOW_MAX_DEPTH`、`INodeExt::lookup_follow`。 | path/mount lookup、component boundary。 | 对齐路径解析和 follow depth；mount table 模拟语义只迁移真实 rCore 有调用面的部分。 |
| `file.rs` | `FileHandle`、`OpenFileDescription`、`OpenOptions`、`SeekFrom`、`read/write/seek/mmap/poll`。 | `FHandle` shared offset、append、negative seek、overflow。 | 已迁移 append 后 offset 到实际写入终点、negative seek 返回错误、write offset checked arithmetic 和 file mmap range checked arithmetic。 |
| `file_like.rs` | `FileLike::{File,Socket,EpollInstance}`、`dup/read/write/ioctl/mmap/poll`。 | `FLike` 分发和 dup 语义。 | epoll dup 已通过 `EpollInstance::clone` 共享 registration/ready/new_ctl；fd-local cloexec 已迁到 `Process.fd_cloexec`，不改变 `FileLike` 共享对象。 |
| `pipe.rs` | `PipeEnd`、`PipeData`、`Pipe`、`INode for Pipe`。 | `PipeNode` clone/drop、read-end close、poll。 | 已迁移 reader/writer endpoint 计数、无 reader 时 write 错误、read EOF readiness 和 poll error。 |
| `epoll.rs` | `EpollInstance`、`EpollEvent`、`EPollCtlOp`、`Process::get_epoll_instance*`。 | `EpInst` ADD/MOD/DEL、ready/control cleanup、dup sharing。 | 已迁移 ADD 重复拒绝、DEL 清理 ready/new_ctl、dup 后共享注册表；event ABI 常量保持上游。 |
| `fcntl.rs` | `F_DUPFD`、`F_GETFD`、`F_SETFD`、`O_NONBLOCK`、`O_APPEND` 等。 | fd flags、cloexec、append/nonblock。 | 保持上游常量；`F_DUPFD`、`F_DUPFD_CLOEXEC`、`F_GETFD`、`F_SETFD` 语义已在 `syscall/fs.rs` 与 `Process` fd-local metadata 落地。 |
| `ioctl.rs` | `Termios`、`Winsize`、TTY ioctl 常量。 | terminal metadata、winsize。 | 对齐 `TrmIO`、`WinSz` 语义；真实行为落到 TTY inode。 |
| `device.rs` | `MemBuf`、`Device for MemBuf`。 | 模拟 disk/block/device buffer 概念。 | 当前只保持上游设备 buffer；cache/disk 模拟结构不直接迁移。 |
| `pseudo.rs` | `Pseudo` inode wrapper。 | kernel object registry/pseudo inode。 | 只迁移真实需要的 pseudo inode 语义，不恢复模拟 registry。 |
| `devfs/mod.rs` | devfs 子模块导出。 | terminal/channel 入口。 | 保持模块边界。 |
| `devfs/tty.rs` | `TtyINode`、`foreground_pgid`、`push`、`read_at`、`io_control`、`poll`。 | `Channel`、terminal、foreground process group。 | 已迁移 TTY buffer read、readable 清理和 async poll 去重唤醒；termios/ioctl ABI 保持上游。 |
| `devfs/serial.rs` | `Serial` inode。 | serial write/read/poll helper。 | 与 `trap.rs::serial` 配合，保持输入输出路径。 |
| `devfs/random.rs` | `RandomINode`。 | 无直接 `kernel.rs` 对应。 | `NO_DIRECT_PORT`，保持上游。 |
| `devfs/shm.rs` | `ShmINode`。 | shared memory devfs hook。 | 与 `kernel/src/ipc` shared memory 记录关联。 |
| `devfs/fbdev.rs` | `Fbdev`、framebuffer ioctl structs。 | 无直接 `kernel.rs` 对应。 | `NO_DIRECT_PORT`，保持上游设备实现。 |

## 子模块-功能迁移记录

| 功能项 | rCore 落点 | `kernel.rs` 来源 | 当前迁移状态 |
| --- | --- | --- | --- |
| shared open-file offset | `file.rs::OpenFileDescription`、`FileHandle::dup`、`Process` fd table | `FHandle`、`Task::dup_fd` | `MIGRATED` for dup/open-file sharing |
| fd-local cloexec | `proc.rs::Process.fd_cloexec`、`syscall/fs.rs::{sys_fcntl,dup_impl}`、`syscall/proc.rs::sys_exec` | `Task::set_cloexec`、exec close-on-exec | `MIGRATED` |
| append write offset | `FileHandle::write` | `FHandle::write` | `MIGRATED` |
| write offset overflow | `FileHandle::write_at` | `FHandle::write_at` | `MIGRATED` |
| negative seek | `FileHandle::seek`、`syscall/fs.rs::sys_lseek` | `FHandle::seek` | `MIGRATED` |
| mmap range overflow | `FileHandle::mmap`、`FileLike::mmap`、`syscall/mem.rs` | `FLike::mmap_fl`、`SYS_MMAP` | `MIGRATED` |
| pipe endpoint lifetime | `Pipe` clone/drop and process fd table | `PipeNode::clone/drop` | `MIGRATED` |
| write after no reader | `Pipe::write_at`、`FileLike::write` | `PipeNode::write_at` | `MIGRATED` |
| epoll duplicate add | `EpollInstance::control` | `EpInst::control ADD` | `MIGRATED` |
| epoll delete cleanup | `EpollInstance::control` | `EpInst::control DEL` | `MIGRATED` |
| epoll dup sharing | `FileLike::dup`、`EpollInstance::clone` | `FLike::dup` | `MIGRATED` |
| epoll ctl/wait boundary | `syscall/fs.rs::{sys_epoll_ctl,sys_epoll_pwait}` | `SYS_EPOLL_CTL`、`SYS_EPOLL_WAIT` | `MIGRATED` |
| TTY/channel wakeup | `devfs/tty.rs`、`sync/event_bus.rs`、`trap.rs::serial` | `Channel::send/send_batch` | `MIGRATED` |

## 待批准迁移候选

| 优先级 | 位置 | 迁移语义 | 最小范围 |
| --- | --- | --- | --- |
| 已完成 | `kernel/src/fs/file.rs::write` | append write 后 fd offset 更新到实际 append 终点。 | `FileHandle::write`。 |
| 已完成 | `kernel/src/fs/file.rs::{write_at,seek}`、`kernel/src/syscall/fs.rs::sys_lseek` | write range checked；negative seek 返回错误，不 cast 成巨大 `u64`。 | `FileHandle::write_at`、`FileHandle::seek`、`sys_lseek`。 |
| 已完成 | `kernel/src/fs/epoll.rs`、`kernel/src/syscall/fs.rs` | ADD 重复拒绝、DEL 清理 ready/new_ctl、dup 共享状态；DEL 不强制读取 event，wait 写回不超过 `maxevents`。 | `EpollInstance`、`FileLike::dup`、`sys_epoll_ctl`、`sys_epoll_pwait`。 |
| 已完成 | `kernel/src/fs/pipe.rs`、`kernel/src/fs/file.rs`、`kernel/src/fs/file_like.rs` | reader 关闭后 writer 返回 `EPIPE`，并保持 endpoint 生命周期。 | `Pipe` state、read/write/drop/poll、`FileLike::write` 错误桥接。 |
| 已完成 | `kernel/src/fs/devfs/tty.rs`、`kernel/src/sync/event_bus.rs` | TTY push/read/poll 和 channel wakeup 语义对齐。 | `TtyINode` read/async_poll 和 `EventBus::subscribe_waker`。 |
