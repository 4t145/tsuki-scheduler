use chrono::TimeDelta;
use tsuki_scheduler::prelude::*;

pub enum Event {
    AddTask(TaskUid, Task<Tokio>),
    RemoveTask(TaskUid),
    Stop,
}

#[tokio::main]
async fn main() {
    let async_runner = AsyncSchedulerRunner::<Tokio, Vec<_>>::default();
    let async_client = async_runner.client();
    let shutdown_signal = Box::pin(tokio::time::sleep(std::time::Duration::from_secs(10)));
    let running_handle = tokio::spawn(async_runner.run_with_shutdown_signal(shutdown_signal));

    let tokio_task_id = TaskUid::uuid();
    let tsuki_task_id = TaskUid::uuid();
    async_client.add_task(
        tokio_task_id,
        Task::tokio(
            Cron::local_from_cron_expr("*/2 * * * * *").unwrap(),
            || async {
                println!("Hello, tsuki!");
            },
        ),
    );
    let now = now();
    async_client.add_task(
        tsuki_task_id,
        Task::tokio(
            Cron::local_from_cron_expr("*/3 * * * * *")
                .unwrap()
                .after(now + TimeDelta::seconds(1))
                .before(now + TimeDelta::seconds(6))
                .then(Cron::local_from_cron_expr("*/1 * * * * *").unwrap()),
            || async {
                println!("Hello, tokio!");
            },
        ),
    );
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    async_client.remove_task(tokio_task_id);
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let runner = running_handle.await.unwrap();
    for task in runner.scheduler.handle_manager {
        task.await.unwrap();
    }
}
