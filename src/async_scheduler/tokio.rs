use crate::runtime::Tokio;

use super::AsyncRuntime;

impl AsyncRuntime for Tokio {
    fn wake_after(
        &self,
        duration: std::time::Duration,
        ctx: &mut std::task::Context<'_>,
    ) {
        let waker = ctx.waker().clone();
        tokio::task::spawn(async move {
            tokio::time::sleep(duration).await;
            waker.wake()
        });
    }
}
