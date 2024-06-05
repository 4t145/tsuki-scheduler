use std::sync::Arc;

use async_std::sync::Mutex;

use crate::{Runtime, Signal};
#[derive(Debug, Clone)]
pub struct AsyncStdRuntime {
    pub signal_sender: Arc<async_std::channel::Sender<Signal<Self>>>,
    pub signal_receiver: Arc<Mutex<async_std::channel::Receiver<Signal<Self>>>>,
}

impl Runtime for AsyncStdRuntime {
    fn spawn<F>(&self, task: F)
    where
        F: std::future::Future<Output = ()> + 'static + Send,
    {
        async_std::task::spawn(task);
    }

    fn send_signal(&self, signal: Signal<Self>) {
        self.signal_sender
            .try_send(signal)
            .expect("fail to send signal");
    }

    async fn recv_signal(&self) -> Signal<Self> {
        self.signal_receiver
            .lock()
            .await
            .recv()
            .await
            .expect("fail to recv signal")
    }

    fn sleep(&self, duration: std::time::Duration) -> impl std::future::Future<Output = ()> {
        async_std::task::sleep(duration)
    }
}


impl Default for AsyncStdRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncStdRuntime {
    pub fn new() -> Self {
        let (signal_sender, signal_receiver) = async_std::channel::unbounded();
        Self {
            signal_sender: Arc::new(signal_sender),
            signal_receiver: Arc::new(Mutex::new(signal_receiver)),
        }
    }
}