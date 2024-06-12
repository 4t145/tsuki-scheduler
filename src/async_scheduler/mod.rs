use std::{
    collections::VecDeque,
    future::Future,
    sync::{Arc, Mutex},
};
const DEFAULT_EXECUTE_DURATION: std::time::Duration = std::time::Duration::from_millis(100);
use crate::{handle_manager::HandleManager, runtime::Runtime, Scheduler, Task, TaskUid};
#[cfg(feature = "async-std")]
mod async_std;
#[cfg(feature = "tokio")]
mod tokio;

pub trait AsyncRuntime: Runtime {
    fn sleep(&self, duration: std::time::Duration) -> impl Future<Output = ()>;
}

#[derive(Debug)]
enum Event<R: Runtime> {
    AddTask(TaskUid, Task<R>),
    RemoveTask(TaskUid),
    Stop,
}

/// A implementation of async scheduler runner
///
/// ```
/// # use tsuki_scheduler::prelude::*;
/// // create runner
/// let mut runner = AsyncSchedulerRunner::<Tokio>::default();
/// // get client
/// let client = runner.client();
/// let task = async move {
///     runner.run().await;
/// };
/// ```
#[derive(Debug)]
pub struct AsyncSchedulerRunner<R: AsyncRuntime, H = ()> {
    /// inner scheduler
    pub scheduler: Scheduler<R, H>,
    /// execute duration
    pub execute_duration: std::time::Duration,
    event_queue: Arc<Mutex<VecDeque<Event<R>>>>,
}

impl<R, H> Default for AsyncSchedulerRunner<R, H>
where
    R: AsyncRuntime + Default,
    H: Default,
{
    fn default() -> Self {
        Self {
            scheduler: Scheduler::default(),
            execute_duration: DEFAULT_EXECUTE_DURATION,
            event_queue: Default::default(),
        }
    }
}

impl<R: AsyncRuntime, H: HandleManager<R::Handle>> AsyncSchedulerRunner<R, H> {
    /// create a new async scheduler runner
    pub fn new(scheduler: Scheduler<R, H>) -> Self {
        Self {
            scheduler,
            execute_duration: DEFAULT_EXECUTE_DURATION,
            event_queue: Default::default(),
        }
    }
    /// set execute duration
    pub fn with_execute_duration(mut self, duration: std::time::Duration) -> Self {
        self.execute_duration = duration;
        self
    }
    /// get runner client
    pub fn client(&self) -> AsyncSchedulerClient<R> {
        AsyncSchedulerClient {
            event_queue: self.event_queue.clone(),
        }
    }
    /// start running
    pub async fn run(&mut self) {
        let mut local_event_queue = VecDeque::new();
        loop {
            self.scheduler.runtime.sleep(self.execute_duration).await;
            {
                let mut add_task_queue = self.event_queue.lock().expect("lock event queue failed");
                std::mem::swap(&mut local_event_queue, &mut add_task_queue);
            }
            while let Some(evt) = local_event_queue.pop_front() {
                match evt {
                    Event::AddTask(key, task) => {
                        self.scheduler.add_task(key, task);
                    }
                    Event::RemoveTask(key) => {
                        self.scheduler.delete_task(key);
                    }
                    Event::Stop => {
                        // restore the event queue
                        let mut add_task_queue =
                            self.event_queue.lock().expect("lock event queue failed");
                        add_task_queue.extend(local_event_queue);
                        return;
                    }
                }
            }
            self.scheduler.execute_by_now();
        }
    }
}

/// Client for [`AsyncSchedulerRunner`]
///
/// created by [`AsyncSchedulerRunner::client`].
///
/// # Clone
/// this client is cheap to clone.
#[derive(Debug)]
pub struct AsyncSchedulerClient<R: AsyncRuntime> {
    event_queue: Arc<Mutex<VecDeque<Event<R>>>>,
}

impl<R: AsyncRuntime> Clone for AsyncSchedulerClient<R> {
    fn clone(&self) -> Self {
        Self {
            event_queue: self.event_queue.clone(),
        }
    }
}

impl<R: AsyncRuntime> AsyncSchedulerClient<R> {
    /// add a new task
    pub fn add_task(&self, key: TaskUid, task: Task<R>) {
        let mut queue = self.event_queue.lock().expect("lock event queue failed");
        queue.push_back(Event::AddTask(key, task));
    }
    /// remove a task by id
    pub fn remove_task(&self, key: TaskUid) {
        let mut queue = self.event_queue.lock().expect("lock event queue failed");
        queue.push_back(Event::RemoveTask(key));
    }
    /// stop the scheduler, if this method is called, the tasks in the queue will not be executed in next loop, and the scheduler will stop.
    pub fn stop(&self) {
        let mut queue = self.event_queue.lock().expect("lock event queue failed");
        queue.push_back(Event::Stop);
    }
}
