#![allow(unused_imports)]

use super::abi::ProcInit;
use super::futex::FutexBucket;
use crate::consts::*;
use crate::fs::{EpInst, FHandle, FLike, FdOpt};
use crate::ipc::{SemCtx, ShmCtx};
use crate::memory::KStk;
use crate::sync::{EventBus, EventFlag};
use crate::trap::Context;
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

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

/// Simulated Unix process group.
///
/// Note: its confusing that this struct is standalone and not the part of the Task struct, the only way of accessing is through 'broadcast_signal'.
///
/// Session is a collection of process groups.
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
