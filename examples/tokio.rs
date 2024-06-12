use tsuki_scheduler::prelude::*;

pub enum Event {
    AddTask(TaskUid, Task<Tokio>),
    RemoveTask(TaskUid),
    Stop,
}

#[tokio::main]
async fn main() {
    let mut async_runner = AsyncSchedulerRunner::<Tokio, Vec<_>>::default();
    let async_client = async_runner.client();
    let runner_task = tokio::spawn(async move {
        async_runner.run().await;
        let handles = async_runner.scheduler.handle_manager;
        for handle in handles {
            handle.await.unwrap();
        }
    });
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
    async_client.add_task(
        tsuki_task_id,
        Task::tokio(
            Cron::local_from_cron_expr("*/3 * * * * *").unwrap(),
            || async {
                println!("Hello, tokio!");
            },
        ),
    );
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    async_client.remove_task(tokio_task_id);
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    async_client.stop();
    runner_task.await.unwrap();
}
