use std::sync::Arc;

use tokio::sync::Mutex;

use super::{AsyncRuntime, AsyncScheduler, Signal};

#[derive(Debug, Clone)]
pub struct Tokio {
    pub signal_sender: Arc<tokio::sync::mpsc::UnboundedSender<Signal<Self>>>,
    pub signal_receiver: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Signal<Self>>>>,
}

impl Default for Tokio {
    fn default() -> Self {
        Self::new()
    }
}
impl Tokio {
    pub fn new() -> Self {
        let (signal_sender, signal_receiver) = tokio::sync::mpsc::unbounded_channel();
        Self {
            signal_sender: Arc::new(signal_sender),
            signal_receiver: Arc::new(Mutex::new(signal_receiver)),
        }
    }
}

impl AsyncRuntime for Tokio {
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
}

impl AsyncScheduler<Tokio> {
    pub fn tokio() -> Self {
        Self::new(Tokio::default())
    }
}
