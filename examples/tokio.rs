use tsuki_scheduler::prelude::*;

pub enum Event {
    AddTask(TaskUid, Task<Tokio>),
    RemoveTask(TaskUid),
    Stop,
}

#[tokio::main]
async fn main() {
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<Event>();
    let scheduler_task = tokio::spawn(async move {
        let mut scheduler = Scheduler::new(Tokio).with_handle_manager(vec![]);
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    scheduler.execute_by_now();
                }
                event = event_rx.recv() => {
                    match event {
                        Some(Event::AddTask(id, task)) => {
                            scheduler.add_task(id, task);
                        }
                        Some(Event::RemoveTask(uid)) => {
                            scheduler.delete_task(uid);
                        }
                        Some(Event::Stop) => {
                            break;
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
        }
        scheduler.handle_manager
    });
    let hello_tokio_task = Task::tokio(
        Cron::local_from_cron_expr("*/3 * * * * *").unwrap(),
        || async {
            println!("Hello, tokio!");
        },
    );
    let tokio_task_id = TaskUid::uuid();
    let hello_tsuki_task = Task::tokio(
        Cron::local_from_cron_expr("*/2 * * * * *").unwrap(),
        || async {
            println!("Hello, tsuki!");
        },
    );
    let tsuki_task_id = TaskUid::uuid();
    event_tx
        .send(Event::AddTask(tokio_task_id, hello_tokio_task))
        .unwrap();
    event_tx
        .send(Event::AddTask(tsuki_task_id, hello_tsuki_task))
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    event_tx.send(Event::RemoveTask(tokio_task_id)).unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    event_tx.send(Event::Stop).unwrap();
    let handles = scheduler_task.await.unwrap();
    for handle in handles {
        handle.await.unwrap();
    }
}
