use std::{
    collections::{BinaryHeap, HashMap},
    future::Future,
    hash::Hash,
    sync::{
        self,
        atomic::{AtomicBool, AtomicU64},
    },
    time::Instant,
};

pub mod runtime;
pub mod schedule;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskUid(pub(crate) u64);

pub trait Schedule {
    fn next(&self) -> Option<std::time::Instant>;
    fn merge<S>(self, schedule: S) -> MergedSchedule<Self, S>
    where
        Self: Sized,
    {
        MergedSchedule(self, schedule)
    }
}

pub struct MergedSchedule<S0, S1>(S0, S1);
impl<S0, S1> Schedule for MergedSchedule<S0, S1>
where
    S0: Schedule,
    S1: Schedule,
{
    fn next(&self) -> Option<std::time::Instant> {
        match (self.0.next(), self.1.next()) {
            (None, None) => None,
            (None, Some(next)) => Some(next),
            (Some(next), None) => Some(next),
            (Some(next_0), Some(next_1)) => Some(next_0.min(next_1)),
        }
    }
}

pub trait Timer {
    fn tick(&self) -> impl Future<Output = ()>;
}
pub struct Task<R> {
    schedule: Box<dyn Schedule + Send>,
    run: Box<dyn Fn(&R) + Send>,
}

impl<R: Runtime> Task<R> {
    pub fn by_spawn<F, Fut, S>(schedule: S, task: F) -> Self
    where
        S: Schedule + Send + 'static,
        F: Fn() -> Fut + 'static + Send,
        Fut: Future<Output = ()> + 'static + Send,
    {
        Task {
            schedule: Box::new(schedule),
            run: Box::new(move |runtime: &R| {
                runtime.spawn(task());
            }),
        }
    }
}

pub trait Runtime: Clone {
    fn spawn<F>(&self, task: F)
    where
        F: Future<Output = ()> + Send + 'static;
    fn send_signal(&self, signal: Signal<Self>);
    fn recv_signal(&self) -> impl Future<Output = Signal<Self>> + Send;
    fn sleep(&self, duration: std::time::Duration) -> impl Future<Output = ()> + Send;
}

pub struct Scheduler<R> {
    pub next: Option<Instant>,
    pub next_up_qu: BinaryHeap<NextUp>,
    pub task: HashMap<TaskUid, Task<R>>,
    pub runtime: R,
}

pub struct NextUp {
    key: TaskUid,
    time: Instant,
}
impl PartialEq for NextUp {
    fn eq(&self, other: &Self) -> bool {
        self.time.eq(&other.time)
    }
}

impl Eq for NextUp {}

impl PartialOrd for NextUp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for NextUp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

impl<R> Scheduler<R> {
    pub fn new(runtime: R) -> Self {
        Self {
            next: None,
            next_up_qu: BinaryHeap::new(),
            task: HashMap::new(),
            runtime,
        }
    }
}
impl<R> Scheduler<R>
where
    R: Runtime,
{
    pub fn add_task(&mut self, key: TaskUid, task: Task<R>) {
        if let Some(next) = task.schedule.next() {
            let next_up = NextUp { key, time: next };
            self.task.insert(key, task);
            self.next_up_qu.push(next_up);
        }
    }
    pub fn delete_task(&mut self, key: TaskUid) -> Option<Task<R>> {
        self.task.remove(&key)
    }

    pub fn execute(&mut self) {
        while let Some(peek) = self.next_up_qu.peek() {
            if peek.time.elapsed().is_zero() {
                self.next = Some(peek.time);
                break;
            } else {
                let mut next_up = self.next_up_qu.pop().expect("should has peek");
                let Some(task) = self.task.get(&next_up.key) else {
                    // has been deleted
                    continue;
                };
                (task.run)(&self.runtime);
                if let Some(next_call) = task.schedule.next() {
                    next_up.time = next_call;
                    self.next_up_qu.push(next_up);
                }
            }
        }
    }
}

pub struct SharedScheduler<R> {
    runtime: R,
    next_id: AtomicU64,
    running: AtomicBool,
}

pub enum Signal<R> {
    AddTask(TaskUid, Task<R>),
    DeleteTask(TaskUid),
    Execute,
    Quit,
}

impl<R> SharedScheduler<R>
where
    R: Runtime + Send + 'static,
{
    pub fn new(runtime: R) -> Self {
        Self {
            next_id: AtomicU64::new(0),
            running: AtomicBool::new(false),
            runtime,
        }
    }
    pub fn start(&self) {
        let main_loop_runtime = self.runtime.clone();
        self.running.store(true, sync::atomic::Ordering::SeqCst);
        self.runtime.spawn(async move {
            let mut scheduler = Scheduler::new(main_loop_runtime.clone());
            loop {
                let signal = main_loop_runtime.recv_signal().await;
                match signal {
                    Signal::AddTask(key, task) => {
                        scheduler.add_task(key, task);
                    }
                    Signal::DeleteTask(key) => {
                        scheduler.delete_task(key);
                    }
                    Signal::Execute => {
                        scheduler.execute();
                    }
                    Signal::Quit => {
                        break;
                    }
                }
            }
        });
    }
    #[inline]
    fn send_signal(&self, signal: Signal<R>) {
        self.runtime.send_signal(signal);
    }
    pub fn add_task(&self, task: Task<R>) -> TaskUid {
        let id = TaskUid(self.next_id.fetch_add(1, sync::atomic::Ordering::SeqCst));
        self.send_signal(Signal::AddTask(id, task));
        id
    }
    pub fn delete_task(&self, key: TaskUid) {
        self.send_signal(Signal::DeleteTask(key));
    }
    pub fn execute(&self) {
        self.send_signal(Signal::Execute);
    }
    pub fn quit(self) {
        self.send_signal(Signal::Quit);
        self.running.store(false, sync::atomic::Ordering::SeqCst);
    }
    pub fn is_running(&self) -> bool {
        self.running.load(sync::atomic::Ordering::SeqCst)
    }
    pub fn start_with_interval_execution(&self, interval: std::time::Duration) {
        let main_loop_runtime = self.runtime.clone();
        self.start();
        self.runtime.spawn(async move {
            loop {
                main_loop_runtime.sleep(interval).await;
                main_loop_runtime.send_signal(Signal::Execute);
            }
        });
    }
}
