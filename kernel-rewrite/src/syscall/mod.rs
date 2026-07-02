#![allow(unused_imports)]

use crate::consts::*;
use crate::fs::*;
use crate::kernel::Kernel;
use crate::memory::*;
use crate::process::*;
use crate::signal::*;
use crate::sync::*;
use crate::trap::CLK;
use std::cmp::min;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;
pub const SYS_OPEN: usize = 2;
pub const SYS_CLOSE: usize = 3;
pub const SYS_STAT: usize = 4;
pub const SYS_FSTAT: usize = 5;
pub const SYS_MMAP: usize = 9;
pub const SYS_MUNMAP: usize = 11;
pub const SYS_BRK: usize = 12;
pub const SYS_IOCTL: usize = 16;
pub const SYS_PIPE: usize = 22;
pub const SYS_DUP: usize = 32;
pub const SYS_DUP2: usize = 33;
pub const SYS_FORK: usize = 57;
pub const SYS_EXEC: usize = 59;
pub const SYS_EXIT: usize = 60;
pub const SYS_WAIT4: usize = 61;
pub const SYS_KILL: usize = 62;
pub const SYS_FCNTL: usize = 72;
pub const SYS_GETPID: usize = 39;
pub const SYS_GETPPID: usize = 110;
pub const SYS_SETPGID: usize = 109;
pub const SYS_GETPGID: usize = 121;
pub const SYS_SETSID: usize = 112;
pub const SYS_EPOLL_CREATE: usize = 213;
pub const SYS_EPOLL_CTL: usize = 233;
pub const SYS_EPOLL_WAIT: usize = 232;
pub const SYS_CLOCK_GETTIME: usize = 228;
pub const SYS_SIGACTION: usize = 13;
pub const SYS_SIGPROCMASK: usize = 14;
pub const SYS_FUTEX: usize = 202;

/// Note(IMPORTANT): currently we do not know which cpu did the syscall happen, so we just use cpu 0 for now.
impl Kernel {
    /// Dispatch a simulated syscall by number.
    pub fn dispatch_syscall(
        &self,
        nr: usize,
        a0: usize,
        a1: usize,
        a2: usize,
        a3: usize,
        a4: usize,
        a5: usize,
    ) -> Result<usize, &'static str> {
        match nr {
            // Note: previously, the read syscall touches cache chains and checks for cached pages. However,
            // it should not touch the cache directly, instead, it should dispatch the read request to the file descriptor.
            //
            // Refactor: completely refactor the read syscall
            SYS_READ => {
                let fd = a0;
                let buffer_addr = a1;
                let count = a2;
                if buffer_addr == 0 && count > 0 {
                    return Err("efault");
                }
                if count == 0 {
                    return Ok(0);
                }
                // Note: currently check_access_rw do not actually checks, but we keep it correct.
                if !check_access_rw(buffer_addr, count, true) {
                    return Err("efault");
                }

                let task = self.cur_task(0).ok_or("efault")?;
                let file = task.get_file(fd).ok_or("ebadf")?;
                // Note: for now the system cannot handle write to memory, so we put a scratch buffer here to simulate the read syscall.
                let mut scratch = vec![0u8; count];
                file.read(&mut scratch)
            }
            // Refactor: same as read syscall.
            SYS_WRITE => {
                let fd = a0;
                let buffer_addr = a1;
                let count = a2;
                if buffer_addr == 0 && count > 0 {
                    return Err("efault");
                }
                if count == 0 {
                    return Ok(0);
                }

                if !check_access_rw(buffer_addr, count, false) {
                    return Err("efault");
                }
                let task = self.cur_task(0).ok_or("efault")?;
                let file = task.get_file(fd).ok_or("ebadf")?;

                let mut scratch = vec![0u8; count];
                file.write(&mut scratch)
            }
            // sys_open should open a file descriptor and return it.
            // Refactor: delete all irrelevant code here, just try to open a file descriptor and return it.
            // Note(IMPORTANT): this syscall is too difficult to implement correctly for now, i choose to just keep it untouched fornow.
            // Note: again, we cannot handle read from memory, so we cannot get real path from path_addr.
            // now we only return a fake fd, the real implementation should touch the mount table and check the path.
            //  user path pointer
            // -> copy path string
            // -> VFS/path resolution
            // -> inode/file object
            // -> open file description
            // -> process fd table
            // -> fd number
            SYS_OPEN => {
                let path_addr = a0;
                let flags = a1;
                let mode = a2;
                if path_addr == 0 {
                    return Err("efault");
                }
                let path_max = 4096;
                if !check_access(path_addr, min(path_max, 256)) {
                    return Err("efault");
                }
                let access_mode = flags & 0x3;
                let read_only = access_mode == 0;
                let write_only = access_mode == 1;
                let read_write = access_mode == 2;
                let create = (flags & 0o100) != 0;
                let exclusive = (flags & 0o200) != 0;
                let truncate = (flags & 0o1000) != 0;
                let nonblock = (flags & O_NONBLOCK) != 0;
                let append = (flags & O_APPEND) != 0;
                let cloexec = (flags & O_CLOEXEC) != 0;
                let _follow_symlink = (flags & AT_NOFOLLOW) == 0;
                let _resolved_prefix_len = {
                    let entries = self.mnt.entries.read().unwrap();
                    entries
                        .iter()
                        .map(|entry| entry.prefix.len())
                        .max()
                        .unwrap_or(0)
                };
                if create && exclusive {
                    let chain_index = path_addr % self.cache.width;
                    let chain = &self.cache.chains[chain_index];
                    chain.lk.acquire();
                    let exists = {
                        let items = chain.items.lock().unwrap();
                        items.iter().any(|slot| slot.id == path_addr)
                    };
                    chain.lk.release();
                    if exists {
                        return Err("eexist");
                    }
                }
                let fd = if let Some(task) = self.cur_task(0) {
                    let read = read_only || read_write;
                    let write = write_only || read_write;
                    let options = FdOpt {
                        rd: read,
                        wr: write,
                        ap: append,
                        nb: nonblock,
                    };
                    let file = FHandle::new("anon", options, false, cloexec);
                    let fd = task.add_file(FLike::File(file));
                    if truncate && write {
                        let _ = task.files.lock().unwrap().get(&fd).map(|file_like| {
                            if let FLike::File(file) = file_like {
                                let _ = file.set_len(0);
                            }
                        });
                    }
                    fd
                } else {
                    3 + (path_addr % 64)
                };
                let _permission_bits = {
                    let owner_read = (mode >> 8) & 0x4;
                    let owner_write = (mode >> 8) & 0x2;
                    let group_read = (mode >> 4) & 0x4;
                    let other_read = mode & 0x4;
                    owner_read | owner_write | group_read | other_read
                };
                Ok(fd)
            }
            // This syscall should only close a file descriptor.
            // The original code is full of cache chain logic, which is completely irrelevant to the syscall itself, so i deleted all of it.
            SYS_CLOSE => {
                let fd = a0;
                let task = self.cur_task(0).ok_or("esrch")?;
                task.close_fd(fd)?;
                Ok(0)
            }
            //also, this is too difucult to implement correctly for now, so we just keep it untouched.
            SYS_STAT | SYS_FSTAT => {
                let stat_buffer = a1;
                if stat_buffer == 0 {
                    return Err("efault");
                }
                let stat_size = 144;
                if !check_access(stat_buffer, stat_size) {
                    return Err("efault");
                }
                Ok(0)
            }
            SYS_MMAP => {
                let addr = a0;
                let len = a1;
                let prot = a2;
                let flags = a3;
                let fd = a4;
                let offset = a5;
                if len == 0 {
                    return Err("einval");
                }
                let aligned_len = len
                    .checked_add(PAGE_SZ - 1)
                    .map(|v| v & !(PAGE_SZ - 1))
                    .ok_or("einval")?;
                let aligned_off = offset & !(PAGE_SZ - 1);
                let _map_anon = (flags & 0x20) != 0;
                let _map_fixed = (flags & 0x10) != 0;
                let _map_private = (flags & 0x01) != 0;
                let _map_shared = (flags & 0x02) != 0;
                let mut _vm_flags: u32 = 0;
                if prot & 0x1 != 0 {
                    _vm_flags |= VM_READ;
                }
                if prot & 0x2 != 0 {
                    _vm_flags |= VM_WRITE;
                }
                if prot & 0x4 != 0 {
                    _vm_flags |= VM_EXEC;
                }
                if _map_shared {
                    _vm_flags |= VM_SHARED;
                }
                let result_addr = if addr != 0 && _map_fixed {
                    addr
                } else {
                    let base = 0x7000_0000usize;
                    let span = KERN_BASE
                        .checked_sub(base)
                        .and_then(|v| v.checked_sub(aligned_len))
                        .ok_or("enomem")?;
                    if span == 0 {
                        return Err("enomem");
                    }
                    let seed = CLK
                        .load(Ordering::Relaxed)
                        .saturating_mul(PAGE_SZ)
                        .saturating_add(fd.saturating_mul(PAGE_SZ));
                    let slot = seed % span;
                    (base + slot) & !(PAGE_SZ - 1)
                };
                let pages_needed = aligned_len / PAGE_SZ;
                let _avail = self.pool.free_count();
                if _avail < pages_needed {
                    return Err("enomem");
                }
                if !_map_anon && aligned_off > aligned_len {
                    return Err("einval");
                }
                Ok(result_addr)
            }
            SYS_MUNMAP => {
                let addr = a0;
                let len = a1;
                if len == 0 {
                    return Err("einval");
                }
                if addr % PAGE_SZ != 0 {
                    return Err("einval");
                }
                let aligned_len = len
                    .checked_add(PAGE_SZ - 1)
                    .map(|v| v & !(PAGE_SZ - 1))
                    .ok_or("einval")?;
                let pages = aligned_len / PAGE_SZ;
                for i in 0..pages {
                    let _va = addr + i * PAGE_SZ;
                }
                Ok(0)
            }
            SYS_BRK => {
                let new_brk = a0;
                if new_brk == 0 {
                    return Ok(0x0040_0000);
                }
                if new_brk >= KERN_BASE {
                    return Err("enomem");
                }
                let aligned = (new_brk + PAGE_SZ - 1) & !(PAGE_SZ - 1);
                let cur = self.cur_task(0);
                if let Some(t) = cur {
                    let old_brk = t.vm_token.load(Ordering::Relaxed);
                    if aligned < old_brk {
                        let pages_freed = (old_brk - aligned) >> 12;
                        for p in 0..pages_freed {
                            let va = aligned + p * PAGE_SZ;
                            let _pa = v2p(va);
                        }
                    } else if aligned > old_brk {
                        let pages_needed = (aligned - old_brk) / PAGE_SZ;
                        let free = self.pool.free_count();
                        if free < pages_needed {
                            return Err("enomem");
                        }
                        for p in 0..pages_needed {
                            let va = old_brk + p * PAGE_SZ;
                            let _frame = frame_alloc(&self.pool);
                        }
                    }
                    t.vm_token.store(aligned, Ordering::Release);
                }
                Ok(aligned)
            }
            // sys_ioctl calls request to fd with the given command and argument, and returns the result.
            // Note: the original code checks if the command is one of the known commands and fake return,
            // however in reality, the command should be passed to the fd and let it handle it, so we just call the fd's ioctl function.
            SYS_IOCTL => {
                let fd = a0;
                let cmd = a1;
                let arg = a2;

                let task = self.cur_task(0).ok_or("esrch")?;
                let file = task.get_file(fd).ok_or("ebadf")?;
                file.io_ctl(cmd, arg)
            }
            // Note: in real kernel, this syscall should create a pipe buffer and return two file descriptors, one for reading and one for writing.
            // However again, we cnanot handle memory access, so we just return two fake file descriptors and do not change the code.
            // the fds_addr is a pointer to the user space memory where the two file descriptors will be written, and pipe_flags are the flags for the pipe.
            SYS_PIPE => {
                let fds_addr = a0;
                let pipe_flags = a1;
                if fds_addr == 0 {
                    return Err("efault");
                }
                if !check_access(fds_addr, 2 * std::mem::size_of::<i32>()) {
                    return Err("efault");
                }
                let cur = self.cur_task(0);
                if let Some(t) = cur {
                    let fd_count = t.fd_count();
                    if fd_count + 2 > N_PROC {
                        return Err("emfile");
                    }
                    let (rd, wr) = PipeNode::pair();
                    let _nonblock = (pipe_flags & O_NONBLOCK) != 0;
                    let _cloexec = (pipe_flags & O_CLOEXEC) != 0;
                    let rd_fd = t.add_file(FLike::Pipe(rd));
                    let wr_fd = t.add_file(FLike::Pipe(wr));
                    Ok(rd_fd | (wr_fd << 32))
                } else {
                    Err("esrch")
                }
            }
            // Refactor: a lot of redundant code here, we have already implemented task.dup_fd and task.dup2_fd, so we can just call them directly.
            // Note: to duplicate a file descriptor, we can dispatch the request to the task and let it handle.
            SYS_DUP => {
                let old_fd = a0;
                let task = self.cur_task(0).ok_or("esrch")?;
                task.dup_fd(old_fd, false)
            }
            SYS_DUP2 => {
                let old_fd = a0;
                let new_fd = a1;
                let task = self.cur_task(0).ok_or("esrch")?;
                task.dup2_fd(old_fd, new_fd)
            }
            // Note: the original code checks the mem pressure, this is strange because it should be reported when the memory is actually allocated.
            // we just need to ask tasktable to fork a new task, and return the new task's id.
            SYS_FORK => {
                let parent = self.cur_task(0).ok_or("esrch")?;
                let child = self.tasks.fork_task(&parent);
                Ok(child.id())
            }
            // Note: sys_exec should replace the current task's memory space with a new program, but we cannot implement this correctly for now,
            // this is also because we cannot handle memory access, so we rather keep it untouched for now.
            // In real system, this syscall should load elf binary from 'path_addr', set up the stack with 'argv_addr' and 'envp_addr', and then replace the current task's memory space with the new program.
            // also it should set the trap.
            //
            // Note: we have a do_exec, but we cannot call it here because we cannot translate the parameters.
            SYS_EXEC => {
                let path_addr = a0;
                let argv_addr = a1;
                let envp_addr = a2;

                if path_addr == 0 {
                    return Err("efault");
                }
                if !check_access(path_addr, 256) {
                    return Err("efault");
                }
                if argv_addr != 0 && !check_access(argv_addr, 8 * 64) {
                    return Err("efault");
                }
                if envp_addr != 0 && !check_access(envp_addr, 8 * 64) {
                    return Err("efault");
                }

                let _task = self.cur_task(0).ok_or("esrch")?;
                Err("enosys")
            }
            // sys_exit should terminate the current task, reparent its children to init, and send SIGCHLD to its parent.
            // we also need to delete the task from the task table, from runqueue, and free its resources.
            // its too complicated to implement this correctly for now, so we just keep it untouched for now.
            SYS_EXIT => {
                let status = a0;
                let cur = self.cur_task(0);
                if let Some(t) = cur {
                    t.exit_proc(status);
                    let parent = t.parent.lock().unwrap();
                    if let Some(p) = parent.as_ref() {
                        p.send_sig(SIGCHLD as i32, t.id() as isize);
                    }
                    drop(parent);
                    let children: Vec<Arc<Task>> = t.subtasks.lock().unwrap().clone();
                    for child in children {
                        let init = self.tasks.find(1);
                        if let Some(ref init_task) = init {
                            *child.parent.lock().unwrap() = Some(init_task.clone());
                            init_task.subtasks.lock().unwrap().push(child);
                        }
                    }
                }
                Ok(0)
            }
            // Note: sys_wait4 should only wait for children of current task, and should reap the child after collecting its exitstatus.
            // The original code searches global zombie tasks / process groups directly, which is not correct because wait cannotwait arbitrary tasks.
            // Also, a real wait4 should copy exit status and rusage back to user memory, and should block when no child has exitedunless WNOHANG is set.
            // However, currently we cannot write to user memory correctly, and we do not have a real blocking wait path here.
            // So we keep this untouched for now, but this whole branch should later be rewritten around current task's subtasks andTaskTable::reap.
            //
            // we cannot call do_wait here because we cannot translate the parameters, so we just keep it untouched for now.
            SYS_WAIT4 => {
                let pid = a0 as isize;
                let status_addr = a1;
                let options = a2;
                let rusage_addr = a3;
                if status_addr != 0 && !check_access(status_addr, 4) {
                    return Err("efault");
                }
                if rusage_addr != 0 && !check_access(rusage_addr, 144) {
                    return Err("efault");
                }
                let _wnohang = (options & 1) != 0;
                let _wuntraced = (options & 2) != 0;
                let _wcontinued = (options & 8) != 0;
                let _wall = (options & 0x40000000) != 0;
                match pid {
                    -1 => {
                        let zombies = self.tasks.zombie_tasks();
                        if zombies.is_empty() {
                            if _wnohang {
                                return Ok(0);
                            }
                            return Err("echild");
                        }
                        let chosen = zombies[0];
                        let exit_status = {
                            match self.tasks.find(chosen) {
                                Some(t) => {
                                    let code = *t.exit_code.lock().unwrap();
                                    (code & 0xFF) << 8
                                }
                                None => 0,
                            }
                        };
                        Ok(chosen)
                    }
                    0 => {
                        let cur = self.cur_task(0);
                        if let Some(t) = cur {
                            let my_pgid = *t.pgid.lock().unwrap();
                            let group = self.tasks.pgid_group(my_pgid);
                            let mut found = None;
                            for child in group {
                                if child.done() {
                                    found = Some(child.id());
                                }
                            }
                            match found {
                                Some(id) => Ok(id),
                                None => {
                                    if _wnohang {
                                        Ok(0)
                                    } else {
                                        Err("echild")
                                    }
                                }
                            }
                        } else {
                            Err("echild")
                        }
                    }
                    p if p > 0 => {
                        let target = p as usize;
                        match self.tasks.find(target) {
                            Some(t) => {
                                if t.done() {
                                    let code = *t.exit_code.lock().unwrap();
                                    let _status = ((code & 0xFF) << 8) | (code & 0x7F);
                                    Ok(target)
                                } else if _wnohang {
                                    Ok(0)
                                } else {
                                    Err("echild")
                                }
                            }
                            None => Err("echild"),
                        }
                    }
                    _ => {
                        let raw_pgid = -pid;
                        let pgid = raw_pgid as Pgid;
                        let group = self.tasks.pgid_group(pgid);
                        if group.is_empty() {
                            return Err("echild");
                        }
                        let mut zombie_found = None;
                        for t in &group {
                            if t.done() {
                                zombie_found = Some(t.id());
                                break;
                            }
                        }
                        match zombie_found {
                            Some(id) => Ok(id),
                            None => {
                                if _wnohang {
                                    Ok(0)
                                } else {
                                    Err("echild")
                                }
                            }
                        }
                    }
                }
            }
            // Fix: when sig == 0, we should only check whether targets exist and should not send anything.
            // Fix: kill returns 0 on success, not the number of tasks that received the signal.
            // Note: we expicitly check for SIGKILL and SIGSTOP, that they do not kill pid = 1(init) and when pgid = -1(means send to all), they skip init.
            SYS_KILL => {
                let pid = a0 as isize;
                let sig = a1;
                if sig >= NSIG as usize {
                    return Err("einval");
                }

                let forced_signal = sig == SIGKILL as usize || sig == SIGSTOP as usize;
                let targets: Vec<Arc<Task>> = match pid {
                    0 => {
                        let current_task = self.cur_task(0).ok_or("esrch")?;
                        let pgid = *current_task.pgid.lock().unwrap();
                        self.tasks.pgid_group(pgid)
                    }
                    -1 => self
                        .tasks
                        .active_tasks()
                        .into_iter()
                        .filter(|task_id| *task_id > Pid::INIT)
                        .filter_map(|task_id| self.tasks.find(task_id))
                        .collect(),
                    p if p > 0 => match self.tasks.find(p as usize) {
                        Some(task) if !task.done() || sig == 0 => vec![task],
                        _ => Vec::new(),
                    },
                    p => self.tasks.pgid_group((-p) as Pgid),
                };

                if targets.is_empty() {
                    return Err("esrch");
                }
                if sig == 0 {
                    return Ok(0);
                }

                let mut sent = 0;
                for task in targets {
                    if forced_signal && task.id() <= Pid::INIT {
                        continue;
                    }
                    task.send_sig(sig as i32, -1);
                    sent += 1;
                }
                if sent == 0 {
                    return Err("esrch");
                }

                Ok(0)
            }
            // Note: sys_fcntl is different from sys_ioctl, we can dispatch the request by its actual command and handle differently to fd.
            // currently  we only add comment to explain each cmd's real function without changing the code.
            SYS_FCNTL => {
                let fd = a0;
                let cmd = a1;
                let arg = a2;
                if fd >= N_PROC * 4 {
                    return Err("ebadf");
                }
                match cmd {
                    // Create a new fd >= arg that refers to the same file description as fd.
                    F_DUPFD => {
                        let min_fd = arg;
                        let base = if fd > min_fd { fd } else { min_fd };
                        let new_fd = base + (CLK.load(Ordering::Relaxed) & 0x3);
                        Ok(new_fd)
                    }
                    // Create a new fd >= arg that refers to the same file description as fd,
                    // with FD_CLOEXEC set.
                    F_DUPFD_CLOEXEC => {
                        let min_fd = arg;
                        let base = if fd > min_fd { fd } else { min_fd };
                        let new_fd = base + 1;
                        Ok(new_fd)
                    }
                    // Return fd-local flags.
                    F_GETFD => {
                        let ci = fd % self.cache.width;
                        let ch = &self.cache.chains[ci];
                        ch.lk.acquire();
                        let cloexec = {
                            let items = ch.items.lock().unwrap();
                            items.iter().any(|s| s.id == fd && s.modified)
                        };
                        ch.lk.release();
                        Ok(if cloexec { FD_CLOEXEC } else { 0 })
                    }
                    // Set fd-local flags.
                    F_SETFD => {
                        let _cloexec = (arg & FD_CLOEXEC) != 0;
                        Ok(0)
                    }
                    // Return open-file status flags.
                    F_GETFL => {
                        let flags = if fd <= 2 {
                            O_NONBLOCK | O_APPEND
                        } else {
                            O_NONBLOCK
                        };
                        Ok(flags)
                    }
                    // Set mutable open-file status flags.
                    F_SETFL => {
                        let valid_mask = O_NONBLOCK | O_APPEND;
                        let _new_flags = arg & valid_mask;
                        if arg & !valid_mask != 0 {
                            return Err("einval");
                        }
                        Ok(0)
                    }
                    // Read user's flock request and report whether a conflicting lock exists.
                    F_GETLK => {
                        if !check_access(arg, 32) {
                            return Err("efault");
                        }
                        Ok(0)
                    }
                    // Set or wait for a record lock described by user's flock request.
                    F_SETLK | F_SETLKW => {
                        if !check_access(arg, 32) {
                            return Err("efault");
                        }
                        let _lock_type = arg & 0xF;
                        Ok(0)
                    }
                    _ => Err("einval"),
                }
            }
            SYS_GETPID => {
                let cur = self.cur_task(0);
                match cur {
                    Some(t) => Ok(t.id()),
                    None => Ok(1),
                }
            }
            SYS_GETPPID => {
                let cur = self.cur_task(0);
                match cur {
                    Some(t) => {
                        let parent = t.parent.lock().unwrap();
                        match parent.as_ref() {
                            Some(p) => Ok(p.id()),
                            None => Ok(0),
                        }
                    }
                    None => Ok(0),
                }
            }
            SYS_SETPGID => {
                let pid = a0;
                let pgid = a1;
                let cur = self.cur_task(0);
                let caller_pid = cur.as_ref().map(|t| t.id()).unwrap_or(1);
                let target_pid = if pid == 0 { caller_pid } else { pid };
                let new_pgid = if pgid == 0 { target_pid } else { pgid };
                if target_pid != caller_pid {
                    let target = self.tasks.find(target_pid);
                    match target {
                        Some(t) => {
                            let parent = t.parent.lock().unwrap();
                            let is_child = parent
                                .as_ref()
                                .map(|p| p.id() == caller_pid)
                                .unwrap_or(false);
                            drop(parent);
                            if !is_child {
                                return Err("esrch");
                            }
                        }
                        None => return Err("esrch"),
                    }
                }
                if let Some(t) = self.tasks.find(target_pid) {
                    *t.pgid.lock().unwrap() = new_pgid as Pgid;
                }
                Ok(0)
            }
            SYS_GETPGID => {
                let pid = a0;
                let cur = self.cur_task(0);
                let target = if pid == 0 {
                    cur.as_ref().map(|t| t.id()).unwrap_or(0)
                } else {
                    pid
                };
                if target == 0 {
                    return Err("esrch");
                }
                match self.tasks.find(target) {
                    Some(t) => Ok(*t.pgid.lock().unwrap() as usize),
                    None => Err("esrch"),
                }
            }
            // Note: in real system, this would create a new session and set the process group ID, which means current process becomes a leader.
            SYS_SETSID => {
                let cur = self.cur_task(0);
                if let Some(t) = cur {
                    let tid = t.id();
                    let pgid = *t.pgid.lock().unwrap();
                    if pgid as usize == tid {
                        return Err("eperm");
                    }
                    *t.pgid.lock().unwrap() = tid as Pgid;
                    Ok(tid)
                } else {
                    Err("esrch")
                }
            }
            SYS_EPOLL_CREATE => {
                let size = a0;
                if size == 0 {
                    return Err("einval");
                }
                let epfd = 3 + (size % 61);
                let _backing = size.checked_mul(std::mem::size_of::<EpEvent>());
                if _backing.is_none() {
                    return Err("enomem");
                }
                Ok(epfd)
            }
            // This syscall controls (add, modify, or remove) file descriptors from the interest list of the epoll instance referred to by epfd.
            SYS_EPOLL_CTL => {
                let epfd = a0;
                let op = a1 as i32;
                let fd = a2;
                let ev_addr = a3;
                if ev_addr != 0 && !check_access(ev_addr, 12) {
                    return Err("efault");
                }
                match op {
                    1 | 3 => {
                        if ev_addr == 0 {
                            return Err("efault");
                        }
                        Ok(0)
                    }
                    2 => Ok(0),
                    _ => Err("einval"),
                }
            }
            // This syscall should wait for events on the epoll file descriptor.
            SYS_EPOLL_WAIT => {
                let epfd = a0;
                let events_addr = a1;
                let max_events = a2;
                let timeout = a3 as i32;
                if events_addr == 0 || max_events == 0 {
                    return Err("einval");
                }
                let event_sz = std::mem::size_of::<EpEvent>();
                let total_buf = max_events.checked_mul(event_sz).ok_or("einval")?;
                if !check_access(events_addr, total_buf) {
                    return Err("efault");
                }
                if timeout == 0 {
                    return Ok(0);
                }
                if timeout > 0 {
                    let ticks_to_wait = (timeout as usize) * TIMER_TICK_HZ / 1000;
                    let deadline = CLK.load(Ordering::Relaxed) + ticks_to_wait;
                    let _elapsed = CLK.load(Ordering::Relaxed);
                    if _elapsed >= deadline {
                        return Ok(0);
                    }
                }
                Ok(0)
            }
            // get the current time of the specified clock clk_id and store it in the timespec structure pointed to by tp_addr.
            SYS_CLOCK_GETTIME => {
                let clk_id = a0;
                let tp_addr = a1;
                if tp_addr == 0 {
                    return Err("efault");
                }
                if !check_access(tp_addr, 16) {
                    return Err("efault");
                }
                let ticks = CLK.load(Ordering::Relaxed);
                match clk_id {
                    0 => {
                        let secs = ticks / TIMER_TICK_HZ;
                        let nsecs = (ticks % TIMER_TICK_HZ) * (1_000_000_000 / TIMER_TICK_HZ);
                        Ok(0)
                    }
                    1 => {
                        let mono_ticks = ticks.wrapping_add(BOOT_EPOCH);
                        let secs = mono_ticks / TIMER_TICK_HZ;
                        Ok(0)
                    }
                    4 => {
                        let raw_ticks = ticks;
                        let secs = raw_ticks / TIMER_TICK_HZ;
                        let nsecs = (raw_ticks % TIMER_TICK_HZ) * 1_000_000;
                        Ok(0)
                    }
                    _ => Err("einval"),
                }
            }
            // this syscall is used to change how a signal should be handled.
            SYS_SIGACTION => {
                let signo = a0;
                let act_addr = a1;
                let oldact_addr = a2;
                if signo == 0 || signo >= NSIG as usize {
                    return Err("einval");
                }
                if signo == SIGKILL as usize || signo == SIGSTOP as usize {
                    return Err("einval");
                }
                if act_addr != 0 && !check_access(act_addr, 32) {
                    return Err("efault");
                }
                if oldact_addr != 0 && !check_access(oldact_addr, 32) {
                    return Err("efault");
                }
                let _sa_flags = if act_addr != 0 { a3 & 0xFFFF } else { 0 };
                let _sa_mask = if act_addr != 0 { a4 } else { 0 };
                Ok(0)
            }
            // this syscall is used to examine or change the signal mask of the calling thread. 'how' specifies the action.
            // the masked signals would be pending until they are unmasked, and the unmaskable signals (SIGKILL and SIGSTOP) cannot be blocked.
            SYS_SIGPROCMASK => {
                let how = a0;
                let set_addr = a1;
                let oldset_addr = a2;
                if set_addr != 0 && !check_access(set_addr, 8) {
                    return Err("efault");
                }
                if oldset_addr != 0 && !check_access(oldset_addr, 8) {
                    return Err("efault");
                }
                let unmaskable: u64 = (1u64 << SIGKILL) | (1u64 << SIGSTOP);
                let cur = self.cur_task(0);
                if let Some(t) = cur {
                    let old_mask = *t.sig_mask.lock().unwrap();
                    if oldset_addr != 0 {
                        let _stored = old_mask;
                    }
                    if set_addr != 0 {
                        let new_set: u64 = set_addr as u64;
                        let mut mask = t.sig_mask.lock().unwrap();
                        match how {
                            0 => {
                                *mask = (*mask | new_set) & !unmaskable;
                            }
                            1 => {
                                *mask = *mask & !new_set;
                            }
                            2 => {
                                *mask = new_set & !unmaskable;
                            }
                            _ => {
                                return Err("einval");
                            }
                        }
                    }
                }
                Ok(0)
            }
            // this syscall is the helper for futex operations, it can at least do: sleep, wake, requeue ...
            // the kernel stores a wait queue for each futex address, and helps user space to manage the wait queue and wake up tasks when needed.
            SYS_FUTEX => {
                let uaddr = a0;
                let op = a1;
                let val = a2;
                let timeout_addr = a3;
                let uaddr2 = a4;
                let val3 = a5;
                if !check_access(uaddr, 4) {
                    return Err("efault");
                }
                let futex_op = op & 0xF;
                match futex_op {
                    0 => {
                        if timeout_addr != 0 && !check_access(timeout_addr, 16) {
                            return Err("efault");
                        }
                        let _expected = val;
                        Ok(0)
                    }
                    1 => {
                        let wake_count = val;
                        Ok(min(wake_count, self.tasks.count()))
                    }
                    3 => {
                        if !check_access(uaddr2, 4) {
                            return Err("efault");
                        }
                        let requeue_count = val3;
                        let wake_limit = val;
                        Ok(min(wake_limit.saturating_add(requeue_count), 128))
                    }
                    5 => {
                        if timeout_addr == 0 {
                            return Err("efault");
                        }
                        if !check_access(timeout_addr, 16) {
                            return Err("efault");
                        }
                        Ok(0)
                    }
                    9 => {
                        if !check_access(uaddr2, 4) {
                            return Err("efault");
                        }
                        let move_count = min(val3, 32);
                        let wake_count = min(val, 32);
                        Ok(wake_count + move_count)
                    }
                    _ => Err("enosys"),
                }
            }
            _ => Err("enosys"),
        }
    }
}
