#![allow(unused_imports)]

use crate::consts::*;
use crate::fs::{EpInst, FHandle, FLike, FdOpt};
use crate::ipc::{SemCtx, ShmCtx};
use crate::memory::KStk;
use crate::sync::{EventBus, EventFlag, FutexBucket};
use crate::trap::Context;
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

pub struct CapSet {
    pub bits: u64,
    pub effective: u64,
    pub ambient: u64,
}

impl CapSet {
    pub fn new() -> Self {
        Self {
            bits: 0,
            effective: 0,
            ambient: 0,
        }
    }

    pub fn full() -> Self {
        Self {
            bits: !0u64,
            effective: !0u64,
            ambient: 0,
        }
    }

    pub fn check(&self, cap: u32) -> bool {
        if cap >= 64 {
            return false;
        }
        (self.effective & (1u64 << cap)) != 0
    }

    pub fn grant(&mut self, cap: u32) {
        if cap < 64 {
            self.bits |= 1u64 << cap;
            self.effective |= 1u64 << cap;
        }
    }

    pub fn drop_cap(&mut self, cap: u32) {
        if cap < 64 {
            self.bits &= !(1u64 << cap);
            self.effective &= !(1u64 << cap);
        }
    }

    pub fn inherit(parent: &CapSet) -> CapSet {
        let mask = INHERITABLE_MASK;
        let parent_bits = parent.bits;
        let parent_effective = parent.effective;
        // Debug fix: inheritance keeps only bits allowed by INHERITABLE_MASK.
        let filtered_bits = parent_bits & mask;
        let filtered_effective = parent_effective & filtered_bits;
        CapSet {
            bits: filtered_bits,
            effective: filtered_effective,
            // Debug fix: ambient capabilities must remain a subset of inherited bits.
            ambient: parent.ambient & filtered_bits,
        }
    }

    pub fn has_any(&self, mask: u64) -> bool {
        (self.effective & mask) != 0
    }

    pub fn clear_ambient(&mut self) {
        self.ambient = 0;
    }

    pub fn raise_ambient(&mut self, cap: u32) -> bool {
        if cap >= 64 {
            return false;
        }
        let cap_bit = 1u64 << cap;
        if (self.bits & cap_bit) != 0 {
            self.ambient |= cap_bit;
            true
        } else {
            false
        }
    }
}

pub struct ProcInit {
    pub args: Vec<String>,
    pub envs: Vec<String>,
    pub auxv: BTreeMap<u8, usize>,
}

impl ProcInit {
    fn reserve_stack_bytes(stack_pointer: &mut usize, byte_count: usize) -> bool {
        match stack_pointer.checked_sub(byte_count) {
            Some(next_stack_pointer) => {
                *stack_pointer = next_stack_pointer;
                true
            }
            None => false,
        }
    }

    pub fn push_at(&self, top: usize) -> usize {
        let word_size = std::mem::size_of::<usize>();
        let mut stack_pointer = top;

        for env in self.envs.iter() {
            if !Self::reserve_stack_bytes(&mut stack_pointer, env.len().saturating_add(1)) {
                return 0;
            }
        }

        for arg in self.args.iter() {
            if !Self::reserve_stack_bytes(&mut stack_pointer, arg.len().saturating_add(1)) {
                return 0;
            }
        }

        let aux_pairs = self.auxv.len();
        let aux_bytes = match aux_pairs
            .checked_mul(2)
            .and_then(|value| value.checked_add(2))
            .and_then(|value| value.checked_mul(word_size))
        {
            Some(value) => value,
            None => return 0,
        };
        if !Self::reserve_stack_bytes(&mut stack_pointer, aux_bytes) {
            return 0;
        }

        let env_ptrs_bytes = match self
            .envs
            .len()
            .checked_add(1)
            .and_then(|value| value.checked_mul(word_size))
        {
            Some(value) => value,
            None => return 0,
        };
        if !Self::reserve_stack_bytes(&mut stack_pointer, env_ptrs_bytes) {
            return 0;
        }

        let arg_ptrs_bytes = match self
            .args
            .len()
            .checked_add(1)
            .and_then(|value| value.checked_mul(word_size))
        {
            Some(value) => value,
            None => return 0,
        };
        if !Self::reserve_stack_bytes(&mut stack_pointer, arg_ptrs_bytes) {
            return 0;
        }

        if !Self::reserve_stack_bytes(&mut stack_pointer, word_size) {
            return 0;
        }

        let alignment_offset = stack_pointer & 0xF;
        if alignment_offset != 0 {
            // Debug fix: every downward adjustment uses checked_sub to avoid underflow.
            if !Self::reserve_stack_bytes(&mut stack_pointer, alignment_offset) {
                return 0;
            }
        }
        stack_pointer
    }

    pub fn total_size(&self) -> usize {
        let mut size = 0usize;
        for arg in &self.args {
            size += arg.len() + 1;
        }
        for env in &self.envs {
            size += env.len() + 1;
        }
        size += (self.auxv.len() * 2 + 2 + self.args.len() + 1 + self.envs.len() + 1 + 1)
            * std::mem::size_of::<usize>();
        size
    }
}

pub struct SchedulePolicy {
    pub policy: u8,
    pub prio: i32,
    pub nice: i32,
    pub time_slice: usize,
    pub vruntime: u64,
}

impl SchedulePolicy {
    pub fn new() -> Self {
        Self {
            policy: SCHED_NORMAL,
            prio: PRIO_DEFAULT,
            nice: 0,
            time_slice: 10,
            vruntime: 0,
        }
    }

    pub fn with_prio(prio: i32) -> Self {
        // Debug fix: clamp in signed space before deriving a time slice.
        let prio = prio.clamp(-20, 19);
        Self {
            policy: SCHED_NORMAL,
            prio,
            nice: prio,
            time_slice: (20i32 - prio).max(1) as usize,
            vruntime: 0,
        }
    }

    // Note: its about 1024 * (1.25)^(-nice).
    pub fn weight(&self) -> u64 {
        match self.nice {
            nice if nice < -10 => 88761,
            nice if nice < 0 => 29154,
            0 => 1024,
            nice if nice < 10 => 335,
            _ => 110,
        }
    }
}

/// Runnable task queue used by the simulated scheduler.
///
/// The queue stores task ids with their scheduling policy. `current` records the
/// task currently considered running together with its policy, and
/// `preempt_count` disables preemption while it is nonzero.
///
/// Refactor: the queue is kept sorted by vruntime.
/// Refactor: 'current' is changed to store both task id and scheduling policy.
/// Note: the relationship between 'prio' and 'nice' is not fully clear, we set 'nice' to be equal to 'prio' for now.
pub struct RunQueue {
    pub queue: Mutex<Vec<(usize, SchedulePolicy)>>,
    pub current: Mutex<Option<(usize, SchedulePolicy)>>,
    pub preempt_count: AtomicUsize,
}

impl RunQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            current: Mutex::new(None),
            preempt_count: AtomicUsize::new(0),
        }
    }

    // Refactor: insert the new task into the vruntime-sorted queue directly.
    pub fn enqueue(&self, task_id: usize, policy: SchedulePolicy) {
        let mut queue = self.queue.lock().unwrap();
        if queue.iter().any(|(queued_id, _)| *queued_id == task_id) {
            return;
        }

        let insert_index = queue
            .iter()
            .position(|(_, queued_policy)| queued_policy.vruntime > policy.vruntime)
            .unwrap_or(queue.len());
        queue.insert(insert_index, (task_id, policy));
    }

    // Refactor: dequeue the task with the smallest vruntime.
    pub fn dequeue(&self) -> Option<(usize, SchedulePolicy)> {
        let mut queue = self.queue.lock().unwrap();
        if queue.is_empty() {
            return None;
        }

        Some(queue.remove(0))
    }

    pub fn pick_next(&self) -> Option<usize> {
        self.queue
            .lock()
            .unwrap()
            .first()
            .map(|(task_id, _)| *task_id)
    }

    // Refactor: rebalance should only reorder tasks by current vruntime.
    pub fn rebalance(&self) {
        let mut queue = self.queue.lock().unwrap();
        queue.sort_by_key(|(_, policy)| policy.vruntime);
    }

    pub fn set_current(&self, task_id: usize) {
        self.set_current_with_policy(task_id, SchedulePolicy::new());
    }

    pub fn set_current_with_policy(&self, task_id: usize, policy: SchedulePolicy) {
        *self.current.lock().unwrap() = Some((task_id, policy));
    }

    pub fn clear_current(&self) {
        *self.current.lock().unwrap() = None;
    }

    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    pub fn remove(&self, task_id: usize) -> bool {
        let mut queue = self.queue.lock().unwrap();
        let original_len = queue.len();
        queue.retain(|(queued_task_id, _)| *queued_task_id != task_id);
        queue.len() < original_len
    }

    pub fn update_vruntime(&self, task_id: usize, delta: u64) {
        let mut queue = self.queue.lock().unwrap();
        if let Some((_, policy)) = queue
            .iter_mut()
            .find(|(queued_task_id, _)| *queued_task_id == task_id)
        {
            let scaled_delta = delta.saturating_mul(1024) / policy.weight();
            policy.vruntime = policy.vruntime.saturating_add(scaled_delta);
        }
    }

    pub fn preempt_disable(&self) {
        self.preempt_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn preempt_enable(&self) {
        let mut previous_count = self.preempt_count.load(Ordering::Relaxed);
        // Debug fix: enabling preemption at zero must not underflow.
        while previous_count != 0 {
            match self.preempt_count.compare_exchange_weak(
                previous_count,
                previous_count - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(next_count) => previous_count = next_count,
            }
        }
    }

    pub fn preemptible(&self) -> bool {
        self.preempt_count.load(Ordering::Relaxed) == 0
    }

    pub fn boost_priority(&self, task_id: usize, amount: i32) {
        let mut queue = self.queue.lock().unwrap();
        for (queued_id, policy) in queue.iter_mut() {
            if *queued_id == task_id {
                let boosted_priority = (policy.prio - amount).clamp(-20, 19);
                policy.prio = boosted_priority;
                policy.nice = boosted_priority;
                policy.time_slice = (20i32 - boosted_priority).max(1) as usize;
                break;
            }
        }
    }

    pub fn yield_current(&self) -> bool {
        let current_task = self.current.lock().unwrap().take();
        if let Some((task_id, policy)) = current_task {
            self.enqueue(task_id, policy);
            true
        } else {
            false
        }
    }
}

/// Thread id used by task/thread-facing helpers.
pub type Tid = usize;

/// Process group id used for group signal delivery and session logic.
pub type Pgid = i32;

/// Process identifier wrapper.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pid(pub usize);

impl Pid {
    pub const INIT: usize = 1;

    pub fn new() -> Self {
        Pid(0)
    }

    pub fn get(&self) -> usize {
        self.0
    }

    pub fn is_init(&self) -> bool {
        self.0 == Self::INIT
    }
}

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Public task snapshot used by inspection/status APIs.
#[derive(Clone, Debug)]
pub struct TaskInfo {
    /// Stable task id used by task-table lookups.
    pub id: usize,
    /// Human-readable task name or label.
    pub tag: String,
    /// Exit status when the task has finished.
    pub status: Option<i32>,
    /// Snapshot of file-descriptor names for inspection.
    pub fds: Vec<String>,
}

/// Per-thread execution context and thread-local syscall state.
pub struct ThdCtx {
    /// Saved user CPU context.
    pub uctx: Context,
    /// Userspace address cleared on thread exit.
    pub clear_tid: usize,
    /// Signal mask saved with this thread context.
    pub smask: u64,
}

impl Default for ThdCtx {
    fn default() -> Self {
        Self {
            uctx: Context::new(),
            clear_tid: 0,
            smask: 0,
        }
    }
}

/// Simulated process/task object.
///
/// It owns task metadata, parent/child links, file descriptors, IPC contexts,
/// signal state, epoll instances, and the saved thread context.
///
/// Note: this looks like a process with a control of a main thread. For now we will not decouple it.
///
/// Fix: a lot of redundant code around locking and cloning.
pub struct Task {
    /// Public task snapshot; `info.id` is the stable key used by this simulator.
    pub info: Mutex<TaskInfo>,
    pub parent: Mutex<Option<Arc<Task>>>,
    pub subtasks: Mutex<Vec<Arc<Task>>>,
    pub files: Mutex<BTreeMap<usize, FLike>>,
    pub cwd: Mutex<String>,
    pub exec_path: Mutex<String>,
    pub futexes: Mutex<BTreeMap<usize, Arc<FutexBucket>>>,
    pub sem_ctx: Mutex<SemCtx>,
    pub shm_ctx: Mutex<ShmCtx>,
    /// rCore-like process id. It usually mirrors `info.id` after task-table registration.
    pub pid: Mutex<Pid>,
    pub pgid: Mutex<Pgid>,
    pub threads: Mutex<Vec<Tid>>,
    pub ev: Arc<Mutex<EventBus>>,
    pub exit_code: Mutex<usize>,
    pub sig_queue: Mutex<VecDeque<(i32, isize)>>,
    pub sig_mask: Mutex<u64>,
    pub ep_inst: Mutex<BTreeMap<usize, EpInst>>,
    pub kstk: Mutex<Option<KStk>>,
    pub thd_ctx: Mutex<Option<ThdCtx>>,
    pub vm_token: AtomicUsize,
}

impl Task {
    pub fn make(id: usize, tag: &str) -> Arc<Self> {
        Arc::new(Self {
            info: Mutex::new(TaskInfo {
                id,
                tag: tag.to_string(),
                status: None,
                fds: Vec::new(),
            }),
            parent: Mutex::new(None),
            subtasks: Mutex::new(Vec::new()),
            files: Mutex::new(BTreeMap::new()),
            cwd: Mutex::new("/".to_string()),
            exec_path: Mutex::new(String::new()),
            futexes: Mutex::new(BTreeMap::new()),
            sem_ctx: Mutex::new(SemCtx::default()),
            shm_ctx: Mutex::new(ShmCtx::default()),
            pid: Mutex::new(Pid::new()),
            pgid: Mutex::new(0),
            threads: Mutex::new(Vec::new()),
            ev: EventBus::make(),
            exit_code: Mutex::new(0),
            sig_queue: Mutex::new(VecDeque::new()),
            sig_mask: Mutex::new(0),
            ep_inst: Mutex::new(BTreeMap::new()),
            kstk: Mutex::new(None),
            thd_ctx: Mutex::new(Some(ThdCtx::default())),
            vm_token: AtomicUsize::new(0),
        })
    }

    pub fn id(&self) -> usize {
        self.info.lock().unwrap().id
    }

    pub fn tag(&self) -> String {
        self.info.lock().unwrap().tag.clone()
    }

    pub fn link_parent(&self, parent: &Arc<Task>) {
        *self.parent.lock().unwrap() = Some(parent.clone());
    }

    pub fn link_child(&self, child: &Arc<Task>) {
        self.subtasks.lock().unwrap().push(child.clone());
    }

    pub fn done(&self) -> bool {
        self.info.lock().unwrap().status.is_some()
    }

    pub fn n_children(&self) -> usize {
        self.subtasks.lock().unwrap().len()
    }

    pub fn get_free_fd(&self) -> usize {
        let files = self.files.lock().unwrap();
        (0..)
            .find(|candidate_fd| !files.contains_key(candidate_fd))
            .unwrap()
    }

    pub fn get_free_fd_from(&self, start_fd: usize) -> usize {
        let files = self.files.lock().unwrap();
        (start_fd..)
            .find(|candidate_fd| !files.contains_key(candidate_fd))
            .unwrap()
    }

    //Debug fix: should not lock twice
    pub fn add_file(&self, file: FLike) -> usize {
        let mut files = self.files.lock().unwrap();
        let fd = (0..)
            .find(|candidate_fd| !files.contains_key(candidate_fd))
            .unwrap();
        files.insert(fd, file);
        fd
    }

    pub fn get_file(&self, fd: usize) -> Option<FLike> {
        self.files.lock().unwrap().get(&fd).cloned()
    }

    pub fn get_futex(&self, user_addr: usize) -> Arc<FutexBucket> {
        let mut futexes = self.futexes.lock().unwrap();
        futexes
            .entry(user_addr)
            .or_insert_with(|| Arc::new(FutexBucket::new()))
            .clone()
    }

    pub fn exit_proc(&self, code: usize) {
        self.files.lock().unwrap().clear();
        {
            let mut bus = self.ev.lock().unwrap();
            bus.set(EventFlag::PROC_QUIT);
        }
        {
            let parent_guard = self.parent.lock().unwrap();
            if let Some(ref parent) = *parent_guard {
                let mut parent_bus = parent.ev.lock().unwrap();
                parent_bus.set(EventFlag::CHILD_QUIT);
            }
        }
        *self.exit_code.lock().unwrap() = code;
        self.threads.lock().unwrap().clear();
        self.info.lock().unwrap().status = Some((code & 0xFF) as i32);
    }

    pub fn exited(&self) -> bool {
        let threads = self.threads.lock().unwrap();
        threads.is_empty() || self.info.lock().unwrap().status.is_some()
    }

    pub fn get_ep_mut(&self, fd: usize) -> Result<EpInst, &'static str> {
        self.ep_inst
            .lock()
            .unwrap()
            .get(&fd)
            .cloned()
            .ok_or("eperm")
    }

    pub fn get_ep_ref(&self, fd: usize) -> Result<EpInst, &'static str> {
        self.get_ep_mut(fd)
    }

    pub fn set_ep(&self, fd: usize, instance: EpInst) {
        let mut epolls = self.ep_inst.lock().unwrap();
        epolls.insert(fd, instance);
    }

    //Note: when a thread is scheduled to run, its context is moved out of 'thread_context'.
    pub fn begin_run(&self) -> ThdCtx {
        self.thd_ctx.lock().unwrap().take().unwrap_or_default()
    }

    pub fn end_run(&self, context: ThdCtx) {
        let mut thread_context = self.thd_ctx.lock().unwrap();
        *thread_context = Some(context);
    }

    // checks whether the task has any pending signals that are not masked by the signal mask.
    pub fn has_sig(&self) -> bool {
        let signal_queue = self.sig_queue.lock().unwrap();
        if signal_queue.is_empty() {
            return false;
        }
        let signal_mask = *self.sig_mask.lock().unwrap();
        let task_id = self.id();
        let mut found = false;
        for (signal, sender) in signal_queue.iter() {
            let signal_number = *signal;
            let sender_tid = *sender;
            if sender_tid != -1 && sender_tid as usize != task_id {
                continue;
            }
            let bit = if signal_number >= 0 && (signal_number as u32) < 64 {
                1u64 << (signal_number as u64)
            } else {
                0
            };
            if bit != 0 && (signal_mask & bit) == 0 {
                found = true;
                break;
            }
        }
        found
    }

    // adds a signal to the task's signal queue, set event flag to indicate that a signal has been received.
    pub fn send_sig(&self, signo: i32, sender_tid: isize) {
        let mut signal_queue = self.sig_queue.lock().unwrap();
        let duplicate = signal_queue
            .iter()
            .any(|(signal, sender)| *signal == signo && *sender == sender_tid);
        // Debug fix: standard signals coalesce instead of queueing duplicates.
        if duplicate && signo > 0 && (signo as u32) < NSIG {
            return;
        }
        signal_queue.push_back((signo, sender_tid));
        drop(signal_queue);
        let mut bus = self.ev.lock().unwrap();
        bus.set(EventFlag::RECV_SIG);
    }

    pub fn close_fd(&self, fd: usize) -> Result<(), &'static str> {
        self.files
            .lock()
            .unwrap()
            .remove(&fd)
            .map(|_| ())
            .ok_or("ebadf")
    }

    //Debug fix: dup_fd should not lock twice, so we inline the logic of find_free_fd here.
    pub fn dup_fd(&self, old_fd: usize, cloexec: bool) -> Result<usize, &'static str> {
        let mut files = self.files.lock().unwrap();
        let file = files.get(&old_fd).cloned().ok_or("ebadf")?;
        let new_fd = (0..).find(|fd| !files.contains_key(fd)).unwrap();
        files.insert(new_fd, file.dup(cloexec));
        Ok(new_fd)
    }

    //Debug fix: should not lock twice, also should check if old_fd exists even if old_fd == new_fd.
    pub fn dup2_fd(&self, old_fd: usize, new_fd: usize) -> Result<usize, &'static str> {
        let mut files = self.files.lock().unwrap();
        if old_fd == new_fd {
            if files.contains_key(&old_fd) {
                return Ok(new_fd);
            }
            return Err("ebadf");
        }
        let new_file = files.get(&old_fd).cloned().ok_or("ebadf")?.dup(false);
        files.insert(new_fd, new_file);
        Ok(new_fd)
    }

    pub fn fd_count(&self) -> usize {
        self.files.lock().unwrap().len()
    }

    pub fn set_cloexec(&self, fd: usize, value: bool) -> Result<(), &'static str> {
        let mut files = self.files.lock().unwrap();
        match files.get_mut(&fd) {
            Some(FLike::File(file)) => {
                // Debug fix: validation is not enough; the stored fd state must change.
                file.cloexec = value;
                Ok(())
            }
            Some(_) => Ok(()),
            None => Err("ebadf"),
        }
    }
}

impl fmt::Debug for Task {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let info = self.info.lock().unwrap();
        formatter
            .debug_struct("T")
            .field("id", &info.id)
            .field("tag", &info.tag)
            .finish()
    }
}

/// Global simulated task table.
///
/// This plays the role of rCore's process/thread lookup tables in the single-file
/// simulator: it assigns ids, stores live tasks, remembers init, and owns fork/reap
/// bookkeeping.
pub struct TaskTable {
    /// Task lookup table keyed by the simulator task id / process id.
    pub map: RwLock<BTreeMap<usize, Arc<Task>>>,
    /// Monotonic id allocator used by spawn, fork, and clone_thread.
    pub seq: AtomicUsize,
    /// Init/root task used as the reparenting target for orphan children.
    pub root: Mutex<Option<Arc<Task>>>,
}

impl TaskTable {
    pub fn new() -> Self {
        Self {
            map: RwLock::new(BTreeMap::new()),
            seq: AtomicUsize::new(1),
            root: Mutex::new(None),
        }
    }

    pub fn spawn(&self, tag: &str) -> Arc<Task> {
        let task_id = self.seq.fetch_add(1, Ordering::SeqCst);
        let task = Task::make(task_id, tag);
        self.map.write().unwrap().insert(task_id, task.clone());
        task
    }

    pub fn spawn_root(&self) -> Arc<Task> {
        let task = self.spawn("init");
        *self.root.lock().unwrap() = Some(task.clone());
        task
    }

    pub fn find(&self, task_id: usize) -> Option<Arc<Task>> {
        self.map.read().unwrap().get(&task_id).cloned()
    }

    pub fn find_by_tag(&self, tag: &str) -> Vec<Arc<Task>> {
        self.map
            .read()
            .unwrap()
            .values()
            .filter(|task| task.tag() == tag)
            .cloned()
            .collect()
    }

    pub fn process_of_tid(&self, tid: usize) -> Option<Arc<Task>> {
        self.map
            .read()
            .unwrap()
            .values()
            .find(|task| task.threads.lock().unwrap().contains(&tid))
            .cloned()
    }

    pub fn pgid_group(&self, pgid: Pgid) -> Vec<Arc<Task>> {
        self.map
            .read()
            .unwrap()
            .values()
            .filter(|task| *task.pgid.lock().unwrap() == pgid)
            .cloned()
            .collect()
    }

    pub fn register(&self, task: &Arc<Task>, pid: Pid) {
        // In this simulator, registered pids are expected to mirror TaskInfo::id.
        *task.pid.lock().unwrap() = pid.clone();
        self.map.write().unwrap().insert(pid.get(), task.clone());
    }

    //Note: reaping a task means deleting it from the task table and reparenting its children to init for now.
    pub fn reap(&self, task_id: usize) {
        let task = { self.map.read().unwrap().get(&task_id).cloned() };
        if let Some(task) = task {
            if let Some(parent) = task.parent.lock().unwrap().clone() {
                // Debug fix: reaping a child must also remove it from the parent's child list.
                parent
                    .subtasks
                    .lock()
                    .unwrap()
                    .retain(|child| child.id() != task_id);
            }
            task.info.lock().unwrap().status = Some(0);
            let children: Vec<Arc<Task>> = task.subtasks.lock().unwrap().drain(..).collect();
            let root_task = self.root.lock().unwrap().clone();
            if let Some(ref root) = root_task {
                for child in children {
                    child.link_parent(root);
                    root.link_child(&child);
                }
            }
            self.map.write().unwrap().remove(&task_id);
        }
    }

    pub fn count(&self) -> usize {
        self.map.read().unwrap().len()
    }

    pub fn fork_task(&self, source: &Arc<Task>) -> Arc<Task> {
        let child_id = self.seq.fetch_add(1, Ordering::SeqCst);
        let child = Task::make(child_id, &source.tag());
        *child.cwd.lock().unwrap() = source.cwd.lock().unwrap().clone();
        *child.exec_path.lock().unwrap() = source.exec_path.lock().unwrap().clone();
        {
            let source_files = source.files.lock().unwrap();
            let mut child_files = child.files.lock().unwrap();
            for (&fd, file) in source_files.iter() {
                let duplicated_file = file.dup(false);
                child_files.insert(fd, duplicated_file);
            }
        }
        *child.pgid.lock().unwrap() = *source.pgid.lock().unwrap();
        *child.sem_ctx.lock().unwrap() = source.sem_ctx.lock().unwrap().clone();
        *child.shm_ctx.lock().unwrap() = source.shm_ctx.lock().unwrap().clone();
        *child.sig_mask.lock().unwrap() = *source.sig_mask.lock().unwrap();
        child.link_parent(source);
        source.link_child(&child);
        self.register(&child, Pid(child_id));
        child.threads.lock().unwrap().push(child_id);
        child
    }

    //Note: now we cannot distinguish between a thread and a process, so we just clone a task.
    pub fn clone_thread(
        &self,
        source: &Arc<Task>,
        stack_top: u64,
        tls: u64,
        clear_tid: usize,
    ) -> Arc<Task> {
        let task_id = self.seq.fetch_add(1, Ordering::SeqCst);
        let task = Task::make(task_id, &source.tag());
        let mut context = ThdCtx::default();
        context.uctx.set_ret(0);
        context.uctx.set_sp(stack_top);
        context.uctx.set_tls(tls);
        context.clear_tid = clear_tid;
        context.smask = *source.sig_mask.lock().unwrap();
        task.end_run(context);
        task.vm_token
            .store(source.vm_token.load(Ordering::Relaxed), Ordering::Relaxed);
        self.map.write().unwrap().insert(task_id, task.clone());
        source.threads.lock().unwrap().push(task_id);
        task
    }

    pub fn new_user_task(&self, path: &str, args: Vec<String>, envs: Vec<String>) -> Arc<Task> {
        let task = self.spawn(path);
        *task.exec_path.lock().unwrap() = path.to_string();
        let mut context = ThdCtx::default();
        let init = ProcInit {
            args,
            envs,
            auxv: BTreeMap::new(),
        };
        let stack_pointer = init.push_at(USR_STK_OFF + USR_STK_SZ);
        context.uctx.set_sp(stack_pointer as u64);
        task.end_run(context);
        let stdin = FHandle::new(
            "/dev/tty",
            FdOpt {
                rd: true,
                wr: false,
                ap: false,
                nb: false,
            },
            false,
            false,
        );
        let stdout = FHandle::new(
            "/dev/tty",
            FdOpt {
                rd: false,
                wr: true,
                ap: false,
                nb: false,
            },
            false,
            false,
        );
        let stderr = stdout.dup(false);
        {
            let mut files = task.files.lock().unwrap();
            files.insert(0, FLike::File(stdin));
            files.insert(1, FLike::File(stdout));
            files.insert(2, FLike::File(stderr));
        }
        self.register(&task, Pid(task.id()));
        task.threads.lock().unwrap().push(task.id());
        task
    }

    pub fn terminate_and_collect(&self, task_id: usize, code: usize) -> bool {
        if let Some(task) = self.find(task_id) {
            task.exit_proc(code);
            self.reap(task_id);
            true
        } else {
            false
        }
    }

    pub fn active_tasks(&self) -> Vec<usize> {
        self.map
            .read()
            .unwrap()
            .iter()
            .filter(|(_, task)| !task.done())
            .map(|(task_id, _)| *task_id)
            .collect()
    }

    pub fn zombie_tasks(&self) -> Vec<usize> {
        self.map
            .read()
            .unwrap()
            .iter()
            .filter(|(_, task)| task.done())
            .map(|(task_id, _)| *task_id)
            .collect()
    }

    pub fn send_signal_group(&self, pgid: Pgid, signo: i32) -> usize {
        let group = self.pgid_group(pgid);
        let count = group.len();
        for task in group {
            task.send_sig(signo, -1);
        }
        count
    }
}

/// Yield the current host thread in the std-based simulator.
pub fn yield_now_sync() {
    thread::yield_now();
}

/// Top-level simulation kernel facade.
///
/// It owns the task table, block/cache devices, frame allocator, per-CPU current
/// task slots, mount table, IPC stores, and the simulated TTY input buffer.
pub struct ProcessGroup {
    pub pgid: Pgid,
    pub leader: usize,
    pub members: Mutex<Vec<usize>>,
    pub session_id: usize,
    pub foreground: AtomicBool, // is current process at foregroound of terminal.
}

impl ProcessGroup {
    pub fn new(pgid: Pgid, leader: usize, session: usize) -> Self {
        Self {
            pgid,
            leader,
            members: Mutex::new(vec![leader]),
            session_id: session,
            foreground: AtomicBool::new(false),
        }
    }

    pub fn add_member(&self, pid: usize) {
        let mut members = self.members.lock().unwrap();
        if !members.contains(&pid) {
            members.push(pid);
        }
    }

    pub fn remove_member(&self, pid: usize) -> bool {
        let mut members = self.members.lock().unwrap();
        let before = members.len();
        members.retain(|&m| m != pid);
        members.len() < before
    }

    pub fn is_empty(&self) -> bool {
        self.members.lock().unwrap().is_empty()
    }

    pub fn member_count(&self) -> usize {
        self.members.lock().unwrap().len()
    }

    pub fn is_leader(&self, pid: usize) -> bool {
        self.leader == pid
    }

    pub fn set_foreground(&self, fg: bool) {
        self.foreground.store(fg, Ordering::Relaxed);
    }

    pub fn is_foreground(&self) -> bool {
        self.foreground.load(Ordering::Relaxed)
    }

    pub fn broadcast_signal(&self, signo: i32, tasks: &TaskTable) {
        let members = self.members.lock().unwrap();
        let member_ids = members.clone();
        drop(members);
        for pid in member_ids {
            let task = tasks.find(pid);
            match task {
                Some(t) => {
                    t.send_sig(signo, self.leader as isize);
                }
                None => {}
            }
        }
    }
}

/// Simulated wait queue keyed by an address or object id. This looks like the implementation of futex wait queues.
/// but we already have a futex waitqueue.
///
/// It parks host threads and wakes waiters by matching `key`;
pub struct WaitQueue {
    pub inner: Mutex<VecDeque<(usize, thread::Thread, u32)>>,
    pub wake_count: AtomicUsize,
}

impl WaitQueue {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(VecDeque::new()),
            wake_count: AtomicUsize::new(0),
        }
    }

    pub fn sleep(&self, key: usize, flags: u32) {
        let mut queue = self.inner.lock().unwrap();
        queue.push_back((key, thread::current(), flags));
        drop(queue);
        thread::park();
    }

    pub fn sleep_timeout(&self, key: usize, flags: u32, timeout: Duration) -> bool {
        let current_thread = thread::current();
        let current_thread_id = current_thread.id();
        let mut queue = self.inner.lock().unwrap();
        queue.push_back((key, current_thread, flags));
        drop(queue);
        thread::park_timeout(timeout);
        let mut queue = self.inner.lock().unwrap();
        let waiter_index = queue
            .iter()
            .enumerate()
            .rev()
            .find(|(_, (wait_key, waiter_thread, waiter_flags))| {
                *wait_key == key
                    && waiter_thread.id() == current_thread_id
                    && *waiter_flags == flags
            })
            .map(|(index, _)| index);
        if let Some(waiter_index) = waiter_index {
            queue.remove(waiter_index);
            false
        } else {
            true
        }
    }

    pub fn wake_one(&self, key: usize) -> bool {
        let mut queue = self.inner.lock().unwrap();
        if let Some(waiter_index) = queue.iter().position(|(wait_key, _, _)| *wait_key == key) {
            let (_, waiter_thread, _) = queue.remove(waiter_index).unwrap();
            waiter_thread.unpark();
            self.wake_count.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    // Refactor: can be implemented by wake_filtered.
    pub fn wake_all(&self, key: usize) -> usize {
        self.wake_filtered(|wait_key, _| wait_key == key)
    }

    pub fn wake_filtered(&self, predicate: impl Fn(usize, u32) -> bool) -> usize {
        let mut queue = self.inner.lock().unwrap();
        let mut woken_count = 0;
        let mut remaining_waiters = VecDeque::new();
        for waiter_entry in queue.drain(..) {
            if predicate(waiter_entry.0, waiter_entry.2) {
                waiter_entry.1.unpark();
                woken_count += 1;
            } else {
                remaining_waiters.push_back(waiter_entry);
            }
        }
        *queue = remaining_waiters;
        self.wake_count.fetch_add(woken_count, Ordering::Relaxed);
        woken_count
    }

    pub fn pending_count(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    pub fn total_wakes(&self) -> usize {
        self.wake_count.load(Ordering::Relaxed)
    }

    pub fn has_waiters_for(&self, key: usize) -> bool {
        self.inner
            .lock()
            .unwrap()
            .iter()
            .any(|(wait_key, _, _)| *wait_key == key)
    }

    // Note: this function takes the third element of the tuple as priority, however, it should be flags.
    pub fn reorder_by_priority(&self) {
        let mut queue = self.inner.lock().unwrap();
        queue
            .make_contiguous()
            .sort_by(|left_waiter, right_waiter| left_waiter.2.cmp(&right_waiter.2));
    }
}

/// Simulated per-process resource limits.
///
/// This stores a small subset of Unix-style resource ceilings.
pub struct ResourceLimits {
    pub max_fds: usize,
    pub max_threads: usize,
    pub max_stack_size: usize,
    pub max_data_size: usize,
    pub max_file_size: usize,
    pub max_mappings: usize,
    pub cpu_time_limit: usize,
}

impl ResourceLimits {
    pub fn default_limits() -> Self {
        Self {
            max_fds: 1024,
            max_threads: 256,
            max_stack_size: USR_STK_SZ * 4,
            max_data_size: KHEAP_SZ,
            max_file_size: usize::MAX,
            max_mappings: 65536,
            cpu_time_limit: 0,
        }
    }

    pub fn check_fd(&self, current: usize) -> bool {
        current < self.max_fds
    }
    pub fn check_threads(&self, current: usize) -> bool {
        current < self.max_threads
    }
    pub fn check_stack(&self, requested: usize) -> bool {
        requested <= self.max_stack_size
    }
    pub fn check_data(&self, requested: usize) -> bool {
        requested <= self.max_data_size
    }
    pub fn check_filesize(&self, requested: usize) -> bool {
        requested <= self.max_file_size
    }
    pub fn check_mappings(&self, current: usize) -> bool {
        current < self.max_mappings
    }

    pub fn inherit(&self) -> Self {
        Self {
            max_fds: self.max_fds,
            max_threads: self.max_threads,
            max_stack_size: self.max_stack_size,
            max_data_size: self.max_data_size,
            max_file_size: self.max_file_size,
            max_mappings: self.max_mappings,
            cpu_time_limit: self.cpu_time_limit,
        }
    }

    // Note: corresponds to RLIMIT_CPU/FSIZE/DATA/STACK/NOFILE
    pub fn set_limit(&mut self, resource: usize, value: usize) -> Result<(), &'static str> {
        match resource {
            0 => {
                self.cpu_time_limit = value;
                Ok(())
            }
            1 => {
                self.max_file_size = value;
                Ok(())
            }
            2 => {
                self.max_data_size = value;
                Ok(())
            }
            3 => {
                self.max_stack_size = value;
                Ok(())
            }
            7 => {
                self.max_fds = value;
                Ok(())
            }
            _ => Err("einval"),
        }
    }

    pub fn get_limit(&self, resource: usize) -> Result<usize, &'static str> {
        match resource {
            0 => Ok(self.cpu_time_limit),
            1 => Ok(self.max_file_size),
            2 => Ok(self.max_data_size),
            3 => Ok(self.max_stack_size),
            7 => Ok(self.max_fds),
            _ => Err("einval"),
        }
    }

    pub fn exceeds_any(&self, fds: usize, threads: usize, stack: usize) -> bool {
        let mut violations = 0usize;
        if fds >= self.max_fds {
            violations += 1;
        }
        if threads >= self.max_threads {
            violations += 1;
        }
        if stack > self.max_stack_size {
            violations += 1;
        }
        violations > 0
    }
}
