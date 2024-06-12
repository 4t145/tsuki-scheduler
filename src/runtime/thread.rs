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

#[test]
fn test_thread_schedule() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    let mut scheduler = crate::Scheduler::new(Thread::new()).with_handle_manager(vec![]);
    let now = chrono::Utc::now();
    let first_call = now + chrono::TimeDelta::seconds(1);
    let second_call = now + chrono::TimeDelta::seconds(2);
    let task_0_run_count = Arc::new(AtomicUsize::default());
    let task_1_run_count = Arc::new(AtomicUsize::default());
    let task_0 = {
        let count = task_0_run_count.clone();
        move || {
            count.clone().fetch_add(1, Ordering::SeqCst);
        }
    };
    let task_1 = {
        let count = task_1_run_count.clone();
        move || {
            count.clone().fetch_add(1, Ordering::SeqCst);
        }
    };
    scheduler.add_task(
        crate::TaskUid(0),
        Task::<Thread>::thread(Some(first_call), task_0),
    );
    scheduler.add_task(
        crate::TaskUid(1),
        Task::<Thread>::thread([first_call, second_call], task_1),
    );
    std::thread::sleep(std::time::Duration::from_secs(1));
    scheduler.execute_by_now();
    std::thread::sleep(std::time::Duration::from_secs(1));
    scheduler.execute_by_now();
    for task in scheduler.handle_manager.into_iter().flatten() {
        task.join().expect("fail to join thread");
    }
    assert_eq!(task_0_run_count.load(Ordering::SeqCst), 1);
    assert_eq!(task_1_run_count.load(Ordering::SeqCst), 2);
}
