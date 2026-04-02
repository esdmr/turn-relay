use std::{
    future::Future,
    sync::{Arc, Mutex},
};

use futures::Stream;
use tokio::sync::broadcast;

use std::pin::pin;

#[derive(Debug, Clone)]
pub struct BroadcastStream<T>(pub Arc<Mutex<broadcast::Receiver<T>>>);

impl<T> BroadcastStream<T>
where
    T: Clone + Unpin,
{
    pub fn new(rcv: broadcast::Receiver<T>) -> Self {
        Self(Arc::new(Mutex::new(rcv)))
    }
}

impl<T> Stream for BroadcastStream<T>
where
    T: Clone + Unpin,
{
    type Item = T;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        pin!(self.0.lock().unwrap().recv()).poll(cx).map(|i| i.ok())
    }
}
