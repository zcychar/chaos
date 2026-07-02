#![allow(unused_imports)]

use crate::consts::*;
use crate::fs::*;
use crate::ipc::*;
use crate::memory::*;
use crate::process::*;
use crate::sync::*;
use crate::trap::*;
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::thread;

/// Top-level simulation kernel facade.
///
/// It owns the task table, block/cache devices, frame allocator, per-CPU current
/// task slots, mount table, IPC stores, and the simulated TTY input buffer.
pub struct Kernel {
    /// Global task/process table.
    pub tasks: TaskTable,
    /// Block cache shared by filesystem-like operations.
    pub cache: BlockCache,
    /// Backing disk model.
    pub disk: Disk,
    /// Physical frame allocator.
    pub pool: FramePool,
    /// Per-CPU current task slots.
    pub cpus: Mutex<[Option<Arc<Task>>; MAX_CPU]>,
    /// Mount table used by path resolution.
    pub mnt: MountTable,
    /// System V semaphore store keyed by semaphore key.
    pub sem_store: RwLock<BTreeMap<u32, Weak<SemArr>>>,
    /// Shared-memory store keyed by segment key.
    pub shm_store: RwLock<BTreeMap<usize, Weak<Mutex<Vec<usize>>>>>,
    /// Simulated terminal input queue.
    pub tty_buf: Mutex<VecDeque<u8>>,
}

impl Kernel {
    pub fn new(frame_count: usize) -> Self {
        Self {
            tasks: TaskTable::new(),
            cache: BlockCache::new(N_CHAINS),
            disk: Disk::new("root"),
            pool: FramePool::new(frame_count),
            cpus: Mutex::new([None, None, None, None, None, None, None, None]),
            mnt: MountTable::new(),
            sem_store: RwLock::new(BTreeMap::new()),
            shm_store: RwLock::new(BTreeMap::new()),
            tty_buf: Mutex::new(VecDeque::new()),
        }
    }

    // Debug fix: invalid use of GKL and CacheChain
    // Note: this function assumes we clean up the cache chains at the beginning of each tick, so it is equal to calling sync_all on the cache.
    pub fn tick(&self, cpu_id: usize) {
        self.cache.sync_all(cpu_id);
    }

    pub fn cur_task(&self, cpu: usize) -> Option<Arc<Task>> {
        let cpu_slots = self.cpus.lock().unwrap();
        if cpu >= cpu_slots.len() {
            return None;
        }
        cpu_slots[cpu].clone()
    }

    pub fn set_cur(&self, cpu: usize, task: Option<Arc<Task>>) {
        let mut cpu_slots = self.cpus.lock().unwrap();
        if cpu < cpu_slots.len() {
            cpu_slots[cpu] = task;
        }
    }

    // Note(IMPORTANT): currently the system cannot handle page faults, this cannot be done until we completely refactor the vm system.
    pub fn handle_pgfault(&self, addr: usize) -> bool {
        let current_task = self.cur_task(0);
        current_task.is_some()
    }

    pub fn handle_pgfault_ext(&self, addr: usize, access: u8) -> bool {
        self.handle_pgfault(addr)
    }

    pub fn proc_init(&self) {
        let root = self.tasks.spawn_root();
        let root_id = root.id();
        root.threads.lock().unwrap().push(root_id);
        let kernel_stack = KStk::new();
        *root.kstk.lock().unwrap() = Some(kernel_stack);
    }

    pub fn tty_push(&self, byte: u8) {
        let normalized_byte = ser(byte);
        let mut buffer = self.tty_buf.lock().unwrap();
        if buffer.len() < 4096 {
            buffer.push_back(normalized_byte);
        }
    }

    pub fn tty_pop(&self) -> Option<u8> {
        let mut buffer = self.tty_buf.lock().unwrap();
        buffer.pop_front()
    }

    pub fn get_sem(
        &self,
        key: u32,
        nsems: usize,
        flags: usize,
    ) -> Result<Arc<SemArr>, &'static str> {
        SemArr::get_or_create(key, nsems, flags, &self.sem_store)
    }

    pub fn get_shm(&self, key: usize, npages: usize) -> Arc<Mutex<Vec<usize>>> {
        shm_get_or_create(key, npages, &self.shm_store)
    }

    // Note: the behavior of this function is not fully clear for now.
    pub fn spawn_thread(&self, task: Arc<Task>) -> thread::JoinHandle<()> {
        thread::spawn(move || loop {
            let thread_context = task.begin_run();
            task.end_run(thread_context);
            if task.done() {
                break;
            }
            thread::yield_now();
        })
    }
}

/// Note(IMPORTANT): currently we do not know which cpu did the syscall happen, so we just use cpu 0 for now.
impl Kernel {
    pub fn schedule_tick(&self, cpu: usize) {
        dtk(cpu);
        // Refactor: delete strange children task counting and schedule logic but without using any real runqueue.
    }

    // Note: this should be used to balance the load of tasks across CPUS(?)
    pub fn balance_load(&self) -> usize {
        let cpus = self.cpus.lock().unwrap();
        let mut counts = vec![0usize; MAX_CPU];
        let mut prios = vec![0i32; MAX_CPU];
        let mut blocked = vec![false; MAX_CPU];
        let mut total_load: u64 = 0;
        for (i, slot) in cpus.iter().enumerate() {
            if let Some(ref t) = slot {
                counts[i] = t.n_children() + 1;
                prios[i] = *t.pgid.lock().unwrap();
                blocked[i] = t.done();
                total_load += counts[i] as u64;
            }
        }
        let avg_load = if MAX_CPU > 0 {
            total_load / MAX_CPU as u64
        } else {
            0
        };
        compute_load_balance(&counts, &prios, &blocked)
    }

    // Note: this function cleans up all tasks marked as zombie but does not consider waiting.
    // This is strange but kept for now, i think a system should not need this.
    pub fn reclaim_zombies(&self) -> usize {
        let zombies = self.tasks.zombie_tasks();
        let count = zombies.len();
        for id in zombies {
            self.tasks.reap(id);
        }
        count
    }

    // Refactor:alloc pages from the frame pool. cleans all rubbish code and use a single batch_alloc to replace.
    pub fn alloc_pages(&self, count: usize) -> Vec<usize> {
        self.pool
            .batch_alloc(count)
            .into_iter()
            .map(|frame_index| frame_index * PAGE_SZ + MEM_OFF)
            .collect()
    }

    // Refactor: use put to replace hand written logic.
    pub fn free_pages(&self, pages: &[usize]) {
        for &pa in pages {
            let Some(offset) = pa.checked_sub(MEM_OFF) else {
                continue;
            };
            if offset % PAGE_SZ != 0 {
                continue;
            }
            let frame_index = offset / PAGE_SZ;
            self.pool.put(frame_index);
        }
    }

    pub fn memory_pressure(&self) -> usize {
        let total = self.pool.capacity;
        let free = self.pool.free_count();
        if total == 0 {
            return 100;
        }
        let used = total - free;
        let pressure = (used * 100) / total;
        pressure
    }

    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.total_entries(), self.cache.dirty_count())
    }

    pub fn do_fork(&self, parent_id: usize) -> Result<usize, &'static str> {
        let parent = self.tasks.find(parent_id).ok_or("esrch")?;
        let child = self.tasks.fork_task(&parent);
        let child_id = child.id();
        let parent_vm_token = parent.vm_token.load(Ordering::Relaxed);
        child.vm_token.store(parent_vm_token, Ordering::Relaxed);
        Ok(child_id)
    }

    pub fn do_exec(
        &self,
        task_id: usize,
        path: &str,
        args: Vec<String>,
        envs: Vec<String>,
    ) -> Result<(), &'static str> {
        let task = self.tasks.find(task_id).ok_or("esrch")?;
        *task.exec_path.lock().unwrap() = path.to_string();
        {
            let fds: Vec<usize> = task
                .files
                .lock()
                .unwrap()
                .iter()
                .filter_map(|(&fd, fl)| match fl {
                    FLike::File(fh) if fh.cloexec => Some(fd),
                    _ => None,
                })
                .collect();
            for fd in fds {
                task.files.lock().unwrap().remove(&fd);
            }
        }
        let init = ProcInit {
            args,
            envs,
            auxv: BTreeMap::new(),
        };
        let sp = init.push_at(USR_STK_OFF + USR_STK_SZ);
        let mut ctx = ThdCtx::default();
        ctx.uctx.set_sp(sp as u64);
        ctx.uctx.set_ip(0x0040_0000u64);
        *task.thd_ctx.lock().unwrap() = Some(ctx);
        Ok(())
    }

    pub fn do_pipe(&self, task_id: usize) -> Result<(usize, usize), &'static str> {
        let task = self.tasks.find(task_id).ok_or("esrch")?;
        let (rd, wr) = PipeNode::pair();
        let rd_fd = task.add_file(FLike::Pipe(rd));
        let wr_fd = task.add_file(FLike::Pipe(wr));
        Ok((rd_fd, wr_fd))
    }

    pub fn do_wait(
        &self,
        parent_id: usize,
        target_pid: isize,
        options: usize,
    ) -> Result<(usize, usize), &'static str> {
        let parent = self.tasks.find(parent_id).ok_or("esrch")?;
        let wnohang = (options & 1) != 0;
        let children: Vec<Arc<Task>> = parent.subtasks.lock().unwrap().clone();
        if children.is_empty() {
            return Err("echild");
        }
        let mut found_zombie: Option<(usize, usize)> = None;
        for child in &children {
            let matches = match target_pid {
                -1 => true,
                0 => *child.pgid.lock().unwrap() == *parent.pgid.lock().unwrap(),
                p if p > 0 => child.id() == p as usize,
                p => *child.pgid.lock().unwrap() == (-p) as Pgid,
            };
            if matches && child.done() {
                let code = *child.exit_code.lock().unwrap();
                found_zombie = Some((child.id(), code));
                break;
            }
        }
        match found_zombie {
            Some((id, code)) => {
                self.tasks.reap(id);
                Ok((id, code))
            }
            None => {
                if wnohang {
                    Ok((0, 0))
                } else {
                    Err("echild")
                }
            }
        }
    }
}
