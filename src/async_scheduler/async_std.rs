use crate::runtime::AsyncStd;

use super::AsyncRuntime;

impl AsyncRuntime for AsyncStd {
    fn sleep(&self, duration: std::time::Duration) -> impl async_std::prelude::Future<Output = ()> {
        async_std::task::sleep(duration)
    }
}
