use std::future::Future;

use crate::schedule::IntoSchedule;
use crate::{Runtime, Task};

use async_std::task::JoinHandle;

/// AsyncStd runtime.
///
/// The task is spawned using [`async_std::task::spawn`].
///
/// # Create a new task
/// see [`Task::async_std`]
#[derive(Debug, Default)]
pub struct AsyncStd;

impl Runtime for AsyncStd {
    type Handle = JoinHandle<()>;
}

impl AsyncStd {
    pub fn new() -> Self {
        Self
    }
}

impl Task<AsyncStd> {
    /// Create a new task that will be executed with async_std.
    ///
    /// # Example
    /// ```
    /// # use tsuki_scheduler::prelude::*;
    /// let task = Task::async_std(now(), || async {
    ///    println!("Hello, world!");
    /// });
    /// ```
    pub fn async_std<S, F, Fut>(schedule: S, task: F) -> Self
    where
        S: IntoSchedule,
        S::Output: Send + 'static,
        F: Fn() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Task {
            schedule: Box::new(schedule.into_schedule()),
            run: Box::new(move |_: _, _: _| async_std::task::spawn(task())),
        }
    }
}
