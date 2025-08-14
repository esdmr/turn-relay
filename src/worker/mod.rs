mod coordinator;
mod peer;
mod relay;
mod types;

pub use crate::worker::coordinator::{COMMAND_CHANNEL_CAPACITY, SERVICE_CHANNEL_CAPACITY};
pub use crate::worker::types::{CommandMessage, ServiceMessage};

use futures::channel::mpsc;
use tokio::sync::broadcast;

use crate::worker::coordinator::Worker;

pub async fn run<F>(subscribe_command: F, service_snd: mpsc::Sender<ServiceMessage>)
where
    F: Send + FnMut() -> broadcast::Receiver<CommandMessage>,
{
    Worker::new(subscribe_command, service_snd).start().await;
}
