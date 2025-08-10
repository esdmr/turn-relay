use std::mem::replace;
use std::{collections::HashMap, net::SocketAddr};

use crate::worker::types::{CommandMessage, ServiceMessage};
use futures::channel::mpsc;
use futures::future::join_all;
use tokio::sync::broadcast::{self, error::RecvError};
use tokio::task::JoinHandle;

use crate::worker::types::{
    ToAnyhowResult, ToWorkerErr, WorkerErr, WorkerOk, WorkerResult, WorkerResultHelper,
};
use crate::{
    worker::{peer::PeerWorker, relay::RelayWorker, types::DataMessage},
    DEFAULT_FWD_SOCKET_ADDR,
};

pub const DATA_CHANNEL_CAPACITY: usize = u8::MAX as usize;
pub const SERVICE_CHANNEL_CAPACITY: usize = u8::MAX as usize;
pub const COMMAND_CHANNEL_CAPACITY: usize = u8::MAX as usize;

pub struct CoordinatorWorker<F: FnMut() -> broadcast::Receiver<CommandMessage>> {
    subscribe_command: F,
    command_rcv: broadcast::Receiver<CommandMessage>,
    service_snd: mpsc::Sender<ServiceMessage>,
    upstream_snd: mpsc::Sender<DataMessage>,
    downstream_snd: broadcast::Sender<DataMessage>,
    relay: JoinHandle<()>,
    peers: HashMap<String, JoinHandle<()>>,
    fwd_addr: SocketAddr,
}

impl<F> CoordinatorWorker<F>
where
    F: FnMut() -> broadcast::Receiver<CommandMessage>,
{
    pub fn new(mut subscribe_command: F, service_snd: mpsc::Sender<ServiceMessage>) -> Self {
        let command_rcv = subscribe_command();

        let (upstream_snd, upstream_rcv) = mpsc::channel::<DataMessage>(DATA_CHANNEL_CAPACITY);
        let (downstream_snd, _) = broadcast::channel::<DataMessage>(DATA_CHANNEL_CAPACITY);

        let relay = tokio::spawn(
            RelayWorker::new(
                upstream_rcv,
                downstream_snd.clone(),
                subscribe_command(),
                service_snd.clone(),
            )
            .start(),
        );

        Self {
            subscribe_command,
            command_rcv,
            service_snd,
            upstream_snd,
            downstream_snd,
            relay,
            peers: HashMap::new(),
            fwd_addr: DEFAULT_FWD_SOCKET_ADDR,
        }
    }

    async fn handle_command_message(
        &mut self,
        command_message: Result<CommandMessage, RecvError>,
    ) -> WorkerResult {
        use CommandMessage::*;

        match command_message.anyhow().as_recoverable()? {
            ConnectRelay { .. } => WorkerResult::continued(),

            ConnectPeer {
                peer_addr,
                local_addr,
            } => {
                self.peers.insert(
                    peer_addr.to_string(),
                    tokio::spawn(
                        PeerWorker::new(
                            peer_addr,
                            local_addr,
                            self.fwd_addr,
                            self.upstream_snd.clone(),
                            self.downstream_snd.subscribe(),
                            (self.subscribe_command)(),
                            self.service_snd.clone(),
                        )
                        .start(),
                    ),
                );

                WorkerResult::continued()
            }

            ChangeFwdAddr(i) => {
                println!("Coordinator: New peers will forward to {}", i);
                self.fwd_addr = i;
                WorkerResult::continued()
            }

            DisconnectAll => {
                println!("Coordinator: Disconnecting everything");

                let peers = replace(&mut self.peers, HashMap::new());

                join_all(peers.into_values())
                    .await
                    .into_iter()
                    .collect::<Result<Vec<()>, _>>()
                    .anyhow()
                    .as_recoverable()?;

                WorkerResult::continued()
            }

            DisconnectPeer(peer_addr) => {
                if let Some(peer) = self.peers.remove(&peer_addr.to_string()) {
                    peer.await.anyhow().as_recoverable()?;
                } else {
                    eprintln!(
                        "Coordinator: Warning: Could not find peer {} to disconnect",
                        peer_addr
                    );
                }

                WorkerResult::continued()
            }

            TerminateAll => {
                println!("Coordinator: Terminating");

                (&mut self.relay).await.anyhow().as_unrecoverable()?;

                let peers = replace(&mut self.peers, HashMap::new());

                join_all(peers.into_values())
                    .await
                    .into_iter()
                    .collect::<Result<Vec<()>, _>>()
                    .anyhow()
                    .as_unrecoverable()?;

                WorkerResult::terminate()
            }
        }
    }

    async fn handle_loop(&mut self) -> WorkerResult {
        let command_message = self.command_rcv.recv().await;
        self.handle_command_message(command_message).await
    }

    pub async fn start(mut self) {
        println!("Coordinator: Worker started");

        loop {
            match self.handle_loop().await {
                Ok(WorkerOk::Continue) => {}
                Ok(WorkerOk::Terminate) => break,
                Err(WorkerErr::RecoverableError(error)) => {
                    eprintln!("Coordinator: Error: {}", error);
                }
                Err(WorkerErr::UnrecoverableError(error)) => {
                    eprintln!("Coordinator: Fatal: {}", error);
                    break;
                }
            }
        }

        println!("Coordinator: Worker stopped");
    }
}
