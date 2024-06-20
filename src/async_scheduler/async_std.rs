use crate::{runtime::AsyncStd, AsyncSchedulerRunner};

use super::AsyncRuntime;

impl AsyncRuntime for AsyncStd {
    fn wake_after(&self, duration: std::time::Duration, ctx: &mut std::task::Context<'_>) {
        let waker = ctx.waker().clone();
        async_std::task::spawn(async move {
            async_std::task::sleep(duration).await;
            waker.wake()
        });
    }
}

impl AsyncSchedulerRunner<AsyncStd> {
    pub fn async_std() -> Self {
        Self::default()
    }
}
