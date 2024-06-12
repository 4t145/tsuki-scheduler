use std::future::Future;

use crate::schedule::IntoSchedule;
use crate::{Runtime, Task};
use wasm_bindgen_futures::spawn_local;

/// The runtime for javascript environment.
///
/// the task is spawned using [`wasm_bindgen_futures::spawn_local`], and be executed with promise.
///
/// # Create a new task
/// see [`Task::promise`]
pub struct Promise;

impl Runtime for Promise {
    type Handle = ();
}

impl Task<Promise> {
    /// Create a new task that will be executed with promise.
    ///
    /// # Example
    /// ```
    /// # use tsuki_scheduler::prelude::*;
    /// async fn some_task() {
    ///
    /// }
    ///
    /// let task = Task::promise(now(), some_task);
    /// ```
    pub fn promise<S, F, Fut>(schedule: S, task: F) -> Self
    where
        S: IntoSchedule,
        S::Output: Send + 'static,
        F: Fn() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        Task {
            schedule: Box::new(schedule.into_schedule()),
            run: Box::new(move |_: _, _: _| spawn_local(task())),
        }
    }
}
