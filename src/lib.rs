#[cfg(feature = "async-scheduler")]
mod async_scheduler;
#[cfg(feature = "async-scheduler")]
pub use async_scheduler::*;
/// prelude for tsuki_scheduler
pub mod prelude;
use std::{
    collections::{BinaryHeap, HashMap},
    hash::Hash,
};

use handle_manager::HandleManager;
use runtime::Runtime;
use schedule::Schedule;
/// alias for [`chrono::DateTime`] in [`chrono::Utc`] timezone
pub type Dtu = chrono::DateTime<chrono::Utc>;
/// Process the handlers of the tasks
pub mod handle_manager;
/// Runtime to run the tasks
pub mod runtime;
/// Schedules and combinators
pub mod schedule;
/// unique identifier for a task
///
/// # Using uuid
/// enable feature `uuid` and create a new task uid with [`TaskUid::uuid()`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskUid(pub(crate) u128);
impl TaskUid {
    #[cfg(feature = "uuid")]
    pub fn uuid() -> Self {
        Self(uuid::Uuid::new_v4().as_u128())
    }
    pub fn into_inner(self) -> u128 {
        self.0
    }
    pub fn new(inner: u128) -> Self {
        Self(inner)
    }
}

pub type RunTaskFn<R> = dyn Fn(&mut R, &TaskRun) -> <R as Runtime>::Handle + Send;

/// Task to be scheduled
///
/// # Fields
/// - schedule: a [`Schedule`] trait object
/// - run: function to create a new task in specific runtime `R`
///
pub struct Task<R: Runtime> {
    pub schedule: Box<dyn Schedule + Send>,
    pub run: Box<RunTaskFn<R>>,
}

impl<R: Runtime> std::fmt::Debug for Task<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task").finish_non_exhaustive()
    }
}

/// Scheduler to manage tasks
///
/// # Usage
/// ```
/// # use tsuki_scheduler::prelude::*;
/// # use chrono::Utc;
/// let mut scheduler = Scheduler::new(Thread::new());
/// let id = TaskUid::uuid();
/// // add a new task
/// scheduler.add_task(TaskUid::uuid(), Task::thread(Utc::now(), || {
///     println!("Hello, world!");
/// }));
/// // execute all tasks by now
/// scheduler.execute_by_now();
/// // delete task by id
/// scheduler.delete_task(id);
/// ```
///
/// # Manage handles
/// The handle manager is used to manage the handles of the tasks.
///
/// The default one is `()`, which does nothing.
///
/// And you can implement your own handle manager to manage the handles, see [HandleManager](`crate::handle_manager::HandleManager`).
#[derive(Debug)]
pub struct Scheduler<R: Runtime, H = ()> {
    pub(crate) next_up_heap: BinaryHeap<TaskRun>,
    pub(crate) task_map: HashMap<TaskUid, Task<R>>,
    pub(crate) runtime: R,
    pub handle_manager: H,
}

impl<R, H> Default for Scheduler<R, H>
where
    R: Runtime + Default,
    H: Default,
{
    fn default() -> Self {
        Self {
            next_up_heap: BinaryHeap::new(),
            task_map: HashMap::new(),
            runtime: R::default(),
            handle_manager: H::default(),
        }
    }
}
/// A single task running schedule
#[derive(Debug, Clone)]
pub struct TaskRun {
    key: TaskUid,
    time: chrono::DateTime<chrono::Utc>,
}

impl std::fmt::Display for TaskRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}-[{:032x}]-[{}]",
            env!("CARGO_CRATE_NAME"),
            self.key.0,
            self.time
        )
    }
}

impl PartialEq for TaskRun {
    fn eq(&self, other: &Self) -> bool {
        self.time.eq(&other.time)
    }
}

impl Eq for TaskRun {}

impl PartialOrd for TaskRun {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TaskRun {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // the task with earlier next time has higher priority
        self.time.cmp(&other.time).reverse()
    }
}

impl TaskRun {
    pub fn key(&self) -> TaskUid {
        self.key
    }
    pub fn time(&self) -> Dtu {
        self.time
    }
}

impl<R: Runtime> Scheduler<R, ()> {
    pub fn new(runtime: R) -> Self {
        Self {
            next_up_heap: BinaryHeap::new(),
            task_map: HashMap::new(),
            runtime,
            handle_manager: (),
        }
    }
}

impl<R: Runtime, H> Scheduler<R, H> {
    pub fn runtime(&self) -> &R {
        &self.runtime
    }
    /// set handle manager
    /// # Example
    /// ```
    /// # use tsuki_scheduler::prelude::*;
    /// // use a vector to collect handles
    /// let handles: Vec<<Thread as Runtime>::Handle> = vec![];
    /// let scheduler = Scheduler::new(Thread::new()).with_handle_manager(handles);
    /// ```
    pub fn with_handle_manager<H2>(self, handle_manager: H2) -> Scheduler<R, H2> {
        Scheduler {
            next_up_heap: self.next_up_heap,
            task_map: self.task_map,
            runtime: self.runtime,
            handle_manager,
        }
    }
}

impl<R: Runtime, H: HandleManager<R::Handle>> Scheduler<R, H> {
    /// add a new task
    pub fn add_task(&mut self, key: TaskUid, mut task: Task<R>) {
        if let Some(next) = task.schedule.next() {
            let next_up = TaskRun { key, time: next };
            self.task_map.insert(key, task);
            self.next_up_heap.push(next_up);
        }
    }
    /// delete a task by id
    pub fn delete_task(&mut self, key: TaskUid) -> Option<Task<R>> {
        self.task_map.remove(&key)
    }
    /// execute all tasks by now
    #[inline]
    pub fn execute_by_now(&mut self) {
        self.execute(chrono::Utc::now())
    }
    /// execute all tasks by a specific time
    pub fn execute(&mut self, base_time: Dtu) {
        let now = base_time;
        while let Some(peek) = self.next_up_heap.peek() {
            if peek.time > now {
                break;
            } else {
                let mut next_up = self.next_up_heap.pop().expect("should has peek");
                let Some(task) = self.task_map.get_mut(&next_up.key) else {
                    // has been deleted
                    continue;
                };
                let handle = (task.run)(&mut self.runtime, &next_up);
                self.handle_manager.manage(&next_up, handle);
                if let Some(next_call) = task.schedule.next() {
                    next_up.time = next_call;
                    self.next_up_heap.push(next_up);
                } else {
                    self.task_map.remove(&next_up.key);
                }
            }
        }
    }
}

#[inline]
/// a shortcut to call [`chrono::Utc::now()`]
pub fn now() -> Dtu {
    chrono::Utc::now()
}
