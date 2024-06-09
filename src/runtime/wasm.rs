use std::future::Future;

use crate::schedule::IntoSchedule;
use crate::{Runtime, Task};
use wasm_bindgen_futures::spawn_local;

/// The runtime for WebAssembly.
///
/// the task is spawned using [`wasm_bindgen_futures::spawn_local`], and be executed with promise.
///
/// # Create a new task
/// see [`Task::promise`]
pub struct Wasm;

impl Runtime for Wasm {
    type Handle = ();
}

impl Task<Wasm> {
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
        S::Output: 'static + Send,
        F: Fn() -> Fut + 'static + Send,
        Fut: Future<Output = ()> + 'static,
    {
        Task {
            schedule: Box::new(schedule.into_schedule()),
            run: Box::new(move |_: _, _: _| spawn_local(task())),
        }
    }
}
