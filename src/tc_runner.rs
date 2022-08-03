use std::future::Future;
use tokio::task::JoinHandle;

/// Async Task Runner with Traffic-Control ability
pub struct TcRunner {
    sender: async_channel::Sender<()>,
    receiver: async_channel::Receiver<()>,
}

impl TcRunner {
    pub fn new(limit: usize) -> Self {
        let (tx, rx) = async_channel::bounded(limit);
        TcRunner {
            sender: tx,
            receiver: rx,
        }
    }

    pub async fn spawn<T>(&self, task: T) -> JoinHandle<T::Output>
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        let _ = self.sender.send(()).await;
        let rx = self.receiver.clone();
        tokio::spawn(async move {
            let res = task.await;
            let _ = rx.recv().await;
            res
        })
    }
}
