mod coordinator;
mod peer;
mod relay;
mod types;

pub use crate::worker::types::{CommandMessage, ServiceMessage};
pub use crate::worker::coordinator::{COMMAND_CHANNEL_CAPACITY, SERVICE_CHANNEL_CAPACITY};

use futures::channel::mpsc;
use tokio::sync::broadcast;

use crate::worker::coordinator::CoordinatorWorker;

pub async fn run_worker<F>(subscribe_command: F, service_snd: mpsc::Sender<ServiceMessage>)
where
    F: FnMut() -> broadcast::Receiver<CommandMessage>,
{
    CoordinatorWorker::new(subscribe_command, service_snd).start().await;
}
