use std::{
    collections::VecDeque,
    future::{Future, Pending},
    sync::{Arc, Mutex},
    time::Duration,
};
const DEFAULT_EXECUTE_DURATION: std::time::Duration = std::time::Duration::from_millis(100);
use crate::{handle_manager::HandleManager, runtime::Runtime, Scheduler, Task, TaskUid};
#[cfg(feature = "async-std")]
mod async_std;
#[cfg(feature = "tokio")]
mod tokio;

pub trait AsyncRuntime: Runtime + Send + Sync {
    fn wake_after(&self, duration: Duration, ctx: &mut std::task::Context<'_>);
}

#[derive(Debug)]
enum Event<R: Runtime> {
    AddTask(TaskUid, Task<R>),
    RemoveTask(TaskUid),
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
    pub fn run(self) -> AsyncSchedulerRunning<R, H, Pending<()>> {
        self.run_with_shutdown_signal(std::future::pending())
    }

    /// start running with shutdown signal
    pub fn run_with_shutdown_signal<S>(self, shutdown_signal: S) -> AsyncSchedulerRunning<R, H, S>
    where
        S: Future<Output = ()> + Send,
    {
        AsyncSchedulerRunning {
            runner: Some(self),
            event_queue: VecDeque::new(),
            shutdown_signal,
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
}

pub struct AsyncSchedulerRunning<R, H, S>
where
    R: AsyncRuntime + Send,
    H: HandleManager<R::Handle>,
    S: Future<Output = ()> + Send,
{
    runner: Option<AsyncSchedulerRunner<R, H>>,
    event_queue: VecDeque<Event<R>>,
    shutdown_signal: S,
}

unsafe impl<R, H, S> Send for AsyncSchedulerRunning<R, H, S>
where
    R: AsyncRuntime + Send,
    H: HandleManager<R::Handle> + Send,
    S: Future<Output = ()> + Send,
{
}

impl<R, H, S> Unpin for AsyncSchedulerRunning<R, H, S>
where
    R: AsyncRuntime,
    H: HandleManager<R::Handle>,
    S: Future<Output = ()> + Unpin + Send,
{
}
impl<R, H, S> Future for AsyncSchedulerRunning<R, H, S>
where
    R: AsyncRuntime,
    H: HandleManager<R::Handle>,
    S: Future<Output = ()> + Unpin + Send,
{
    type Output = AsyncSchedulerRunner<R, H>;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        let shutdown_signal = std::pin::pin!(&mut this.shutdown_signal);
        match shutdown_signal.poll(cx) {
            std::task::Poll::Ready(_) => {
                let runner = this.runner.take().expect("missing runner");
                {
                    let mut add_task_queue =
                        runner.event_queue.lock().expect("lock event queue failed");
                    while let Some(event) = this.event_queue.pop_back() {
                        add_task_queue.push_front(event)
                    }
                }
                std::task::Poll::Ready(runner)
            }
            std::task::Poll::Pending => {
                let runner = this.runner.as_mut().expect("missing runner");
                {
                    let mut add_task_queue =
                        runner.event_queue.lock().expect("lock event queue failed");
                    std::mem::swap(&mut this.event_queue, &mut add_task_queue);
                }
                while let Some(evt) = this.event_queue.pop_front() {
                    match evt {
                        Event::AddTask(key, task) => {
                            runner.scheduler.add_task(key, task);
                        }
                        Event::RemoveTask(key) => {
                            runner.scheduler.delete_task(key);
                        }
                    }
                }
                runner.scheduler.execute_by_now();
                runner
                    .scheduler
                    .runtime
                    .wake_after(runner.execute_duration, cx);
                std::task::Poll::Pending
            }
        }
    }
}
