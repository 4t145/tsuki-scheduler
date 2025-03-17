use tsuki_scheduler::prelude::*;

#[tokio::test]
async fn test_tokio_schedule() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let mut scheduler = crate::Scheduler::new(Tokio).with_handle_manager(vec![]);
    let now = chrono::Utc::now();
    let first_call = now + chrono::TimeDelta::seconds(1);
    let second_call = now + chrono::TimeDelta::seconds(2);
    let task_0_run_count = Arc::new(AtomicUsize::default());
    let task_1_run_count = Arc::new(AtomicUsize::default());
    let task_0 = {
        let count = task_0_run_count.clone();
        async move |_time: Dtu| {
            count.clone().fetch_add(1, Ordering::SeqCst);
        }
    };
    let task_1 = {
        let count = task_1_run_count.clone();
        async move |_task_uid: TaskUid| {
            count.clone().fetch_add(1, Ordering::SeqCst);
        }
    };
    scheduler.add_task(TaskUid::new(0), Task::new_async(Some(first_call), task_0));
    scheduler.add_task(
        TaskUid::new(1),
        Task::new_async([first_call, second_call], task_1),
    );
    std::thread::sleep(std::time::Duration::from_secs(1));
    scheduler.execute_by_now();
    std::thread::sleep(std::time::Duration::from_secs(1));
    scheduler.execute_by_now();
    for task in scheduler.handle_manager {
        task.await.expect("fail to join thread");
    }
    assert_eq!(task_0_run_count.load(Ordering::SeqCst), 1);
    assert_eq!(task_1_run_count.load(Ordering::SeqCst), 2);
}
