use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{Runtime, Signal};
#[derive(Debug, Clone)]
pub struct TokioRuntime {
    pub signal_sender: Arc<tokio::sync::mpsc::UnboundedSender<Signal<Self>>>,
    pub signal_receiver: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Signal<Self>>>>,
}

impl Default for TokioRuntime {
    fn default() -> Self {
        Self::new()
    }
}
impl TokioRuntime {
    pub fn new() -> Self {
        let (signal_sender, signal_receiver) = tokio::sync::mpsc::unbounded_channel();
        Self {
            signal_sender: Arc::new(signal_sender),
            signal_receiver: Arc::new(Mutex::new(signal_receiver)),
        }
    }
}

impl Runtime for TokioRuntime {
    fn spawn<F>(&self, task: F)
    where
        F: std::future::Future<Output = ()> + 'static + Send,
    {
        tokio::task::spawn(task);
    }

    fn send_signal(&self, signal: Signal<Self>) {
        self.signal_sender
            .send(signal)
            .expect("fail to send signal");
    }

    async fn recv_signal(&self) -> Signal<Self> {
        self.signal_receiver
            .blocking_lock()
            .recv()
            .await
            .expect("fail to recv signal")
    }

    fn sleep(&self, duration: std::time::Duration) -> impl std::future::Future<Output = ()> {
        tokio::time::sleep(duration)
    }
}
