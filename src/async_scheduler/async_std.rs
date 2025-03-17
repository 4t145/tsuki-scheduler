use crate::{AsyncSchedulerRunner, prelude::AsyncRuntime, runtime::AsyncStd};

impl AsyncRuntime for AsyncStd {
    fn wake_after(&self, duration: std::time::Duration, ctx: &mut std::task::Context<'_>) {
        let waker = ctx.waker().clone();
        async_std::task::spawn(async move {
            async_std::task::sleep(duration).await;
            waker.wake()
        });
    }
    fn spawn<F>(task: F) -> Self::Handle
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        async_std::task::spawn(task)
    }
}

impl AsyncSchedulerRunner<AsyncStd> {
    pub fn async_std() -> Self {
        Self::default()
    }
}
