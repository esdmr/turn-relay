use std::mem::take;
use std::{collections::HashMap, net::SocketAddr};

use crate::worker::types::{CommandMessage, ServiceMessage};
use crate::DEFAULT_FWD_SOCKET;
use futures::channel::mpsc;
use futures::future::join_all;
use futures::SinkExt;
use tokio::sync::broadcast::{self, error::RecvError};
use tokio::task::JoinHandle;

use crate::worker::types::{
    ToAnyhowResult, ToWorkerErr, WorkerErr, WorkerOk, WorkerResult, WorkerResultHelper,
};
use crate::worker::{peer, relay, types::DataMessage};

pub const DATA_CHANNEL_CAPACITY: usize = u8::MAX as usize;
pub const SERVICE_CHANNEL_CAPACITY: usize = u8::MAX as usize;
pub const COMMAND_CHANNEL_CAPACITY: usize = u8::MAX as usize;

pub struct Worker<F>
where
    F: Send + FnMut() -> broadcast::Receiver<CommandMessage>,
{
    subscribe_command: F,
    command_rcv: broadcast::Receiver<CommandMessage>,
    service_snd: mpsc::Sender<ServiceMessage>,
    upstream_snd: mpsc::Sender<DataMessage>,
    downstream_snd: broadcast::Sender<DataMessage>,
    relay: JoinHandle<()>,
    peers: HashMap<String, JoinHandle<()>>,
    fwd_addr: SocketAddr,
}

impl<F> Worker<F>
where
    F: Send + FnMut() -> broadcast::Receiver<CommandMessage>,
{
    pub fn new(mut subscribe_command: F, service_snd: mpsc::Sender<ServiceMessage>) -> Self {
        let command_rcv = subscribe_command();

        let (upstream_snd, upstream_rcv) = mpsc::channel::<DataMessage>(DATA_CHANNEL_CAPACITY);
        let (downstream_snd, _) = broadcast::channel::<DataMessage>(DATA_CHANNEL_CAPACITY);

        let relay = tokio::spawn(
            relay::Worker::new(
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
            fwd_addr: DEFAULT_FWD_SOCKET,
        }
    }

    async fn handle_command_message(
        &mut self,
        command_message: Result<CommandMessage, RecvError>,
    ) -> WorkerResult {
        match command_message.anyhow().into_recoverable()? {
            CommandMessage::ConnectRelay { .. } => WorkerResult::continued(),

            CommandMessage::ConnectPeer {
                peer_addr,
                local_addr,
            } => {
                self.peers.insert(
                    peer_addr.to_string(),
                    tokio::spawn(
                        peer::Worker::new(
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

            CommandMessage::ChangeFwdAddr(i) => {
                println!("Coordinator: New peers will forward to {i}");
                self.fwd_addr = i;
                WorkerResult::continued()
            }

            CommandMessage::DisconnectAll => {
                println!("Coordinator: Disconnecting everything");

                let peers = take(&mut self.peers);

                join_all(peers.into_values())
                    .await
                    .into_iter()
                    .collect::<Result<Vec<()>, _>>()
                    .anyhow()
                    .into_recoverable()?;

                WorkerResult::continued()
            }

            CommandMessage::DisconnectPeer(peer_addr) => {
                if let Some(peer) = self.peers.remove(&peer_addr.to_string()) {
                    peer.await.anyhow().into_recoverable()?;
                } else {
                    eprintln!(
                        "Coordinator: Warning: Could not find peer {peer_addr} to disconnect"
                    );

                    self.service_snd
                        .send(ServiceMessage::PeerUnbound(peer_addr))
                        .await
                        .anyhow()
                        .into_recoverable()?;
                }

                WorkerResult::continued()
            }

            CommandMessage::TerminateAll => {
                println!("Coordinator: Terminating");

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
                    eprintln!("Coordinator: Error: {error}");
                }
                Err(WorkerErr::UnrecoverableError(error)) => {
                    eprintln!("Coordinator: Fatal: {error}");
                    break;
                }
            }
        }

        (&mut self.relay).await.unwrap();

        let peers: HashMap<String, JoinHandle<()>> = take(&mut self.peers);

        join_all(peers.into_values())
            .await
            .into_iter()
            .collect::<Result<Vec<()>, _>>()
            .unwrap();

        println!("Coordinator: Worker stopped");
    }
}
