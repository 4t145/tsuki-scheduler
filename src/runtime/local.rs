use crate::schedule::IntoSchedule;
use crate::{Runtime, Task};

/// run in local thread
#[derive(Debug, Clone, Default)]
pub struct Local;

impl Local {
    pub fn new() -> Self {
        Local
    }
}

impl Runtime for Local {
    type Handle = ();
}

impl Task<Local> {
    pub fn local<S, F>(schedule: S, task: F) -> Self
    where
        S: IntoSchedule,
        S::Output: Send + 'static,
        F: Fn() + Send + 'static + Clone,
    {
        Task {
            schedule: Box::new(schedule.into_schedule()),
            run: Box::new(move |_: _, _: _| (task)()),
        }
    }
}
