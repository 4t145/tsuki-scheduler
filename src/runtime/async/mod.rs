use std::{
    future::Future,
    sync::{
        self,
        atomic::{AtomicBool, AtomicU64},
    },
};

use chrono::Utc;

use crate::{IntoSchedule, Scheduler, Task, TaskUid};

#[cfg(feature = "async-std")]
pub mod async_std;
#[cfg(feature = "tokio")]
pub mod tokio;

#[derive(Debug, Default)]
pub struct AsyncScheduler<R> {
    runtime: R,
    next_id: AtomicU64,
    running: AtomicBool,
}

pub enum Signal<R> {
    AddTask(TaskUid, Task<R>),
    DeleteTask(TaskUid),
    Execute,
    Quit,
}

pub trait AsyncRuntime: Clone {
    fn spawn<F>(&self, task: F)
    where
        F: Future<Output = ()> + Send + 'static;
    fn send_signal(&self, signal: Signal<Self>);
    fn recv_signal(&self) -> impl Future<Output = Signal<Self>> + Send;
}

impl<R> AsyncScheduler<R>
where
    R: AsyncRuntime + Send + 'static,
{
    pub fn new(runtime: R) -> Self {
        Self {
            next_id: AtomicU64::new(0),
            running: AtomicBool::new(false),
            runtime,
        }
    }
    pub fn start(&self) {
        if self.is_running() {
            return;
        }
        let main_loop_runtime = self.runtime.clone();
        self.running.store(true, sync::atomic::Ordering::SeqCst);
        self.runtime.spawn(async move {
            let mut scheduler = Scheduler::new(main_loop_runtime.clone());
            loop {
                let signal = main_loop_runtime.recv_signal().await;
                match signal {
                    Signal::AddTask(key, task) => {
                        scheduler.add_task(key, task);
                    }
                    Signal::DeleteTask(key) => {
                        scheduler.delete_task(key);
                    }
                    Signal::Execute => {
                        let now = Utc::now();
                        scheduler.execute(now);
                    }
                    Signal::Quit => {
                        break;
                    }
                }
            }
        });
    }
    #[inline]
    fn send_signal(&self, signal: Signal<R>) {
        self.runtime.send_signal(signal);
    }
    pub fn add_task(&self, task: Task<R>) -> TaskUid {
        let id = TaskUid(self.next_id.fetch_add(1, sync::atomic::Ordering::SeqCst));
        self.send_signal(Signal::AddTask(id, task));
        id
    }
    pub fn delete_task(&self, key: TaskUid) {
        self.send_signal(Signal::DeleteTask(key));
    }
    pub fn execute(&self) {
        self.send_signal(Signal::Execute);
    }
    pub fn quit(self) {
        if self.is_running() {
            self.send_signal(Signal::Quit);
            self.running.store(false, sync::atomic::Ordering::SeqCst);
        }
    }
    #[inline]
    pub fn is_running(&self) -> bool {
        self.running.load(sync::atomic::Ordering::SeqCst)
    }
}

impl<R: AsyncRuntime> Task<R> {
    pub fn by_spawn<S, F, Fut>(schedule: S, task: F) -> Self
    where
        S: IntoSchedule,
        S::Output: 'static + Send,
        F: Fn() -> Fut + 'static + Send,
        Fut: Future<Output = ()> + 'static + Send,
    {
        Task {
            schedule: Box::new(schedule.into_schedule()),
            run: Box::new(move |runtime: &R| {
                runtime.spawn(task());
            }),
        }
    }
}
