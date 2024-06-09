use std::future::Future;

use crate::schedule::IntoSchedule;
use crate::{Runtime, Task};

/// Tokio runtime.
/// 
/// The task is spawned using [`tokio::task::spawn`].
/// 
/// # Create a new task
/// see [`Task::tokio`]
#[derive(Debug, Default)]
pub struct Tokio;

impl Runtime for Tokio {
    type Handle = tokio::task::JoinHandle<()>;
}

impl Tokio {
    pub fn new() -> Self {
        Self
    }
}

impl Task<Tokio> {
    /// Create a new task that will be executed with tokio.
    /// 
    /// # Example
    /// ```
    /// # use tsuki_scheduler::prelude::*;
    /// let task = Task::tokio(now(), || async {
    ///   println!("Hello, world!");
    /// });
    /// ```
    pub fn tokio<S, F, Fut>(schedule: S, task: F) -> Self
    where
        S: IntoSchedule,
        S::Output: 'static + Send,
        F: Fn() -> Fut + 'static + Send,
        Fut: Future<Output = ()> + 'static + Send,
    {
        Task {
            schedule: Box::new(schedule.into_schedule()),
            run: Box::new(move |_: _, _: _| tokio::task::spawn(task())),
        }
    }
}
