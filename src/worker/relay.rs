use std::collections::HashSet;
use std::net::{SocketAddr, ToSocketAddrs};

use anyhow::anyhow;
use futures::{SinkExt, StreamExt};
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::broadcast;

use futures::channel::mpsc;
use turnclient::{
    ChannelUsage, MessageFromTurnServer, MessageToTurnServer, TurnClientBuilder,
};

use crate::worker::types::{
    CommandMessage, DataMessage, ServiceMessage, ToAnyhowResult, ToWorkerErr, MaybeTurnClient, WorkerErr, WorkerErrHelper, WorkerOk, WorkerResult, WorkerResultHelper
};
use crate::DEFAULT_SOCKET_ADDR;

#[derive(Debug)]
pub struct RelayWorker {
    upstream_rcv: mpsc::Receiver<DataMessage>,
    downstream_snd: broadcast::Sender<DataMessage>,
    command_rcv: broadcast::Receiver<CommandMessage>,
    service_snd: mpsc::Sender<ServiceMessage>,
    client: MaybeTurnClient,
    granted_peers: HashSet<String>,
    will_terminate: bool,
}

impl RelayWorker {
    pub fn new(
        upstream_rcv: mpsc::Receiver<DataMessage>,
        downstream_snd: broadcast::Sender<DataMessage>,
        command_rcv: broadcast::Receiver<CommandMessage>,
        service_snd: mpsc::Sender<ServiceMessage>,
    ) -> Self {
        Self {
            upstream_rcv,
            downstream_snd,
            command_rcv,
            service_snd,
            client: MaybeTurnClient(None),
            granted_peers: HashSet::new(),
            will_terminate: false,
        }
    }

    async fn handle_turn_message(
        &mut self,
        turn_message: Option<Result<MessageFromTurnServer, anyhow::Error>>,
    ) -> WorkerResult {
        use MessageFromTurnServer::*;

        match turn_message {
            Some(Ok(AllocationGranted { relay_address, .. })) => {
                self.service_snd
                    .send(ServiceMessage::RelayAllocated(relay_address))
                    .await
                    .anyhow()
                    .as_unrecoverable()?;

                WorkerResult::continued()
            }

            Some(Ok(RecvFrom(src, data))) => {
                self.downstream_snd
                    .send((src, data))
                    .anyhow()
                    .as_recoverable()?;

                WorkerResult::continued()
            }

            Some(Ok(RedirectedToAlternateServer(new_addr))) => {
                self.service_snd
                    .send(ServiceMessage::RelayRedirected(new_addr))
                    .await
                    .anyhow()
                    .as_unrecoverable()?;

                WorkerResult::continued()
            }

            Some(Ok(PermissionCreated(peer_addr))) => {
                self.granted_peers.insert(format!("{}", peer_addr));

                self.service_snd
                    .send(ServiceMessage::RelayPeerGranted(peer_addr))
                    .await
                    .anyhow()
                    .as_recoverable()?;

                WorkerResult::continued()
            }

            Some(Ok(PermissionNotCreated(peer_addr))) => {
                self.service_snd
                    .send(ServiceMessage::RelayPeerDenied(peer_addr))
                    .await
                    .anyhow()
                    .as_recoverable()?;

                WorkerResult::continued()
            }

            Some(Ok(Disconnected)) => {
                self.service_snd
                    .send(ServiceMessage::RelayDisconnected)
                    .await
                    .anyhow()
                    .as_unrecoverable()?;

                WorkerResult::continued()
            }

            Some(Ok(APacketIsReceivedAndAutomaticallyHandled)) => WorkerResult::continued(),
            Some(Ok(ForeignPacket(..))) => WorkerResult::continued(),
            Some(Ok(NetworkChange)) => WorkerResult::continued(),
            Some(Err(e)) => Err(e).as_recoverable(),

            None => {
                self.client.0 = None;
                self.granted_peers.clear();
                WorkerResult::terminate_if(self.will_terminate)
            }
        }
    }

    async fn handle_peer_message(
        &mut self,
        peer_message: Option<(SocketAddr, Vec<u8>)>,
    ) -> WorkerResult {
        let (dst, data) = peer_message.unwrap();

        if let Some(client) = &mut self.client.0 {
            if self.granted_peers.contains(&format!("{}", dst)) {
                client
                    .send(MessageToTurnServer::SendTo(dst, data))
                    .await
                    .as_recoverable()?;
            }
        }

        WorkerResult::continued()
    }

    async fn signal_connection_error(&mut self, error: String) -> WorkerResult {
        self.service_snd
            .send(ServiceMessage::RelayConnectionFailed(error))
            .await
            .anyhow()
            .as_unrecoverable()?;

        WorkerResult::continued()
    }

    async fn signal_authorization_failure(
        &mut self,
        peer_addr: SocketAddr,
        error: String,
    ) -> WorkerResult {
        eprintln!("Relay: Failed to authorize {}: {}", peer_addr, error);

        self.service_snd
            .send(ServiceMessage::RelayPeerDenied(peer_addr))
            .await
            .anyhow()
            .as_recoverable()?;

        WorkerResult::continued()
    }

    async fn handle_command_message(
        &mut self,
        command_message: Result<CommandMessage, broadcast::error::RecvError>,
    ) -> WorkerResult {
        use CommandMessage::*;

        match command_message.anyhow().as_recoverable()? {
            ConnectRelay {
                server,
                username,
                password,
            } => {
                assert!(self.client.0.is_none(), "Connect message received while relay is already connected. GUI is malfunctioning.");

                let server = if server.contains(':') {
                    server.to_socket_addrs()
                } else {
                    (server.as_ref(), 3478).to_socket_addrs()
                }
                .anyhow()
                .and_then(|mut i| {
                    i.next()
                        .ok_or_else(|| anyhow!("Could not resolve {}", server))
                })
                .catch_async(|e| self.signal_connection_error(format!("{}", e)))
                .await?;

                self.client.0 = Some(
                    TurnClientBuilder::new(server, username, password).build_and_send_request(
                        UdpSocket::bind(DEFAULT_SOCKET_ADDR)
                            .await
                            .anyhow()
                            .catch_async(|e| self.signal_connection_error(format!("{}", e)))
                            .await?,
                    ),
                );

                WorkerResult::continued()
            }

            ConnectPeer {
                peer_addr,
                local_addr: _,
            } => {
                if let Some(client) = &mut self.client.0 {
                    if self.granted_peers.contains(&format!("{}", peer_addr)) {
                        self.service_snd
                            .send(ServiceMessage::RelayPeerGranted(peer_addr))
                            .await
                            .anyhow()
                            .as_recoverable()?;
                    } else {
                        client
                            .send(MessageToTurnServer::AddPermission(
                                peer_addr,
                                ChannelUsage::WithChannel,
                            ))
                            .await
                            .catch_async(|e| {
                                self.signal_authorization_failure(peer_addr, format!("{}", e))
                            })
                            .await?;
                    }
                } else {
                    self.service_snd
                        .send(ServiceMessage::RelayDisconnected)
                        .await
                        .anyhow()
                        .as_recoverable()?;
                }

                WorkerResult::continued()
            }

            DisconnectAll => {
                if let Some(client) = &mut self.client.0 {
                    client
                        .send(MessageToTurnServer::Disconnect)
                        .await
                        .as_recoverable()?;
                }

                WorkerResult::continued()
            }

            TerminateAll => {
                self.will_terminate = true;

                if let Some(client) = &mut self.client.0 {
                    client
                        .send(MessageToTurnServer::Disconnect)
                        .await
                        .as_unrecoverable()?;
                }

                WorkerResult::continued()
            }

            ChangeFwdAddr(..) => WorkerResult::continued(),
            DisconnectPeer(..) => WorkerResult::continued(),
        }
    }

    async fn handle_loop(&mut self) -> WorkerResult {
        select! {
            turn_message = self.client.next() => {
                self.handle_turn_message(turn_message).await
            },
            peer_message = self.upstream_rcv.next() => {
                self.handle_peer_message(peer_message).await
            },
            command_message = self.command_rcv.recv() => {
                self.handle_command_message(command_message).await
            },
        }
    }

    pub async fn start(mut self) {
        println!("Relay: Worker started");

        loop {
            match self.handle_loop().await {
                Ok(WorkerOk::Continue) => {},
                Ok(WorkerOk::Terminate) => break,
                Err(WorkerErr::RecoverableError(error)) => {
                    eprintln!("Relay: Error: {}", error);
                },
                Err(WorkerErr::UnrecoverableError(error)) => {
                    eprintln!("Relay: Fatal: {}", error);
                    break;
                },
            }
        }

        println!("Relay: Worker stopped");
    }
}
