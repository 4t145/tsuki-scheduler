use std::thread::JoinHandle;

use crate::schedule::IntoSchedule;
use crate::{Runtime, Task};

/// thread based runtime
/// # Errors
/// if system hasn't enough resource to create new thread, a io::Error will be returned.
#[derive(Debug, Clone, Default)]
pub struct Thread;

impl Thread {
    pub fn new() -> Self {
        Thread
    }
}

impl Runtime for Thread {
    type Handle = std::io::Result<JoinHandle<()>>;
}

impl Task<Thread> {
    pub fn thread<S, F>(schedule: S, task: F) -> Self
    where
        S: IntoSchedule,
        S::Output: Send + 'static,
        F: Fn() + Send + 'static + Clone,
    {
        Task {
            schedule: Box::new(schedule.into_schedule()),
            run: Box::new(move |_: _, task_run: _| {
                std::thread::Builder::new()
                    .name(task_run.to_string())
                    .spawn(task.clone())
            }),
        }
    }
}
