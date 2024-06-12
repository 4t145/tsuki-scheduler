use std::future::Future;

use crate::runtime::Tokio;

use super::AsyncRuntime;

impl AsyncRuntime for Tokio {
    fn sleep(&self, duration: std::time::Duration) -> impl Future<Output = ()> {
        tokio::time::sleep(duration)
    }
}
