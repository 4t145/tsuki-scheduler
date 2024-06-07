use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use crate::{IntoSchedule, Task};
#[derive(Debug, Clone, Default)]
pub struct ThreadRuntime {
    inner: Arc<Mutex<Inner>>,
}
impl ThreadRuntime {
    pub fn new() -> Self {
        ThreadRuntime::default()
    }
    pub fn push_handle(&self, handle: JoinHandle<()>) {
        self.inner.lock().expect("poisoned").push_handle(handle)
    }
    pub fn join_all(&self) {
        self.inner.lock().expect("poisoned").join_all()
    }
}

#[derive(Debug, Default)]
struct Inner {
    tasks: Vec<JoinHandle<()>>,
}
impl Inner {
    fn push_handle(&mut self, handle: JoinHandle<()>) {
        self.tasks.push(handle)
    }
    fn join_all(&mut self) {
        for task in self.tasks.drain(..) {
            let _ = task.join();
        }
    }
}

impl Task<ThreadRuntime> {
    pub fn by_spawn<S, F>(schedule: S, task: F) -> Self
    where
        S: IntoSchedule,
        S::Output: 'static + Send,
        F: Fn() + 'static + Send + Clone,
    {
        Task {
            schedule: Box::new(schedule.into_schedule()),
            run: Box::new(move |runtime: &ThreadRuntime| {
                let handle = std::thread::spawn(task.clone());
                runtime.push_handle(handle)
            }),
        }
    }
}

#[test]
fn test_thread_schedule() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let mut scheduler = crate::Scheduler::new(ThreadRuntime::new());
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
        Task::<ThreadRuntime>::by_spawn(Some(first_call), task_0),
    );
    scheduler.add_task(
        crate::TaskUid(1),
        Task::<ThreadRuntime>::by_spawn([first_call, second_call], task_1),
    );
    std::thread::sleep(std::time::Duration::from_secs(1));
    scheduler.execute_by_now();
    std::thread::sleep(std::time::Duration::from_secs(1));
    scheduler.execute_by_now();
    scheduler.runtime().join_all();
    assert_eq!(task_0_run_count.load(Ordering::SeqCst), 1);
    assert_eq!(task_1_run_count.load(Ordering::SeqCst), 2);
}
