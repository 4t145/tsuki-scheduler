use crate::{AsyncSchedulerRunner, runtime::Tokio};

use crate::runtime::AsyncRuntime;

impl AsyncRuntime for Tokio {
    fn wake_after(&self, duration: std::time::Duration, ctx: &mut std::task::Context<'_>) {
        let waker = ctx.waker().clone();
        tokio::task::spawn(async move {
            tokio::time::sleep(duration).await;
            waker.wake()
        });
    }
    fn spawn<F>(task: F) -> Self::Handle
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        tokio::task::spawn(task)
    }
}

impl AsyncSchedulerRunner<Tokio> {
    pub fn tokio() -> Self {
        Self::default()
    }
}
