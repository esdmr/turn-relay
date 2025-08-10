use std::collections::HashSet;
use std::net::{SocketAddr, ToSocketAddrs};

use anyhow::anyhow;
use futures::{SinkExt, StreamExt};
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::broadcast;

use futures::channel::mpsc;
use turnclient::{ChannelUsage, MessageFromTurnServer, MessageToTurnServer, TurnClientBuilder};

use crate::worker::types::{
    CommandMessage, DataMessage, MaybeTurnClient, ServiceMessage, ToAnyhowResult, ToWorkerErr,
    WorkerErr, WorkerErrHelper, WorkerOk, WorkerResult, WorkerResultHelper,
};
use crate::ALL_DYN_SOCKET;

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
        use MessageFromTurnServer::{
            APacketIsReceivedAndAutomaticallyHandled, AllocationGranted, Disconnected,
            ForeignPacket, NetworkChange, PermissionCreated, PermissionNotCreated, RecvFrom,
            RedirectedToAlternateServer,
        };

        match turn_message {
            Some(Ok(AllocationGranted { relay_address, .. })) => {
                println!("Relay: Available at {relay_address}");

                self.service_snd
                    .send(ServiceMessage::RelayAllocated(relay_address))
                    .await
                    .anyhow()
                    .into_unrecoverable()?;

                WorkerResult::continued()
            }

            Some(Ok(RecvFrom(src, data))) => {
                self.downstream_snd
                    .send((src, data))
                    .anyhow()
                    .into_recoverable()?;

                WorkerResult::continued()
            }

            Some(Ok(RedirectedToAlternateServer(new_addr))) => {
                println!("Relay: Redirected to {new_addr}");

                self.service_snd
                    .send(ServiceMessage::RelayRedirected(new_addr))
                    .await
                    .anyhow()
                    .into_unrecoverable()?;

                WorkerResult::continued()
            }

            Some(Ok(PermissionCreated(peer_addr))) => {
                println!("Relay: Granted send permission to {peer_addr}");

                self.granted_peers.insert(format!("{peer_addr}"));

                self.service_snd
                    .send(ServiceMessage::RelayPeerGranted(peer_addr))
                    .await
                    .anyhow()
                    .into_recoverable()?;

                WorkerResult::continued()
            }

            Some(Ok(PermissionNotCreated(peer_addr))) => {
                println!("Relay: Denied send permission to {peer_addr}");

                self.service_snd
                    .send(ServiceMessage::RelayPeerDenied(peer_addr))
                    .await
                    .anyhow()
                    .into_recoverable()?;

                WorkerResult::continued()
            }

            Some(Ok(Disconnected)) => {
                println!("Relay: Disconnected");

                self.service_snd
                    .send(ServiceMessage::RelayDisconnected)
                    .await
                    .anyhow()
                    .into_unrecoverable()?;

                WorkerResult::continued()
            }

            Some(Ok(APacketIsReceivedAndAutomaticallyHandled)) => WorkerResult::continued(),
            Some(Ok(ForeignPacket(src, _))) => {
                #[cfg(debug_assertions)]
                eprintln!("Relay: Warning: Ignoring an invalid packet from {src}");
                #[cfg(not(debug_assertions))]
                let _ = src;

                WorkerResult::continued()
            }
            Some(Ok(NetworkChange)) => {
                eprintln!("Relay: Warning: Network changed");

                WorkerResult::continued()
            }
            Some(Err(e)) => Err(e).into_recoverable(),

            None => {
                self.client.0 = None;
                self.granted_peers.clear();

                println!("Relay: Disconnected");

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
            if self.granted_peers.contains(&format!("{dst}")) {
                client
                    .send(MessageToTurnServer::SendTo(dst, data))
                    .await
                    .into_recoverable()?;
            }
        }

        WorkerResult::continued()
    }

    async fn signal_connection_error(&mut self, error: String) -> WorkerResult {
        self.service_snd
            .send(ServiceMessage::RelayConnectionFailed(error))
            .await
            .anyhow()
            .into_unrecoverable()?;

        WorkerResult::continued()
    }

    async fn handle_command_message(
        &mut self,
        command_message: Result<CommandMessage, broadcast::error::RecvError>,
    ) -> WorkerResult {
        use CommandMessage::{
            ChangeFwdAddr, ConnectPeer, ConnectRelay, DisconnectAll, DisconnectPeer, TerminateAll,
        };

        match command_message.anyhow().into_recoverable()? {
            ConnectRelay {
                server,
                username,
                password,
            } => {
                assert!(self.client.0.is_none(), "Connect message received while relay is already connected; GUI is malfunctioning");

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
                .catch_async(|e| self.signal_connection_error(format!("{e}")))
                .await?;

                self.client.0 = Some(
                    TurnClientBuilder::new(server, username, password).build_and_send_request(
                        UdpSocket::bind(ALL_DYN_SOCKET)
                            .await
                            .anyhow()
                            .catch_async(|e| self.signal_connection_error(format!("{e}")))
                            .await?,
                    ),
                );

                println!("Relay: Connected to {server}; Waiting for allocation");

                WorkerResult::continued()
            }

            ConnectPeer {
                peer_addr,
                local_addr: _,
            } => {
                if let Some(client) = &mut self.client.0 {
                    if self.granted_peers.contains(&format!("{peer_addr}")) {
                        println!("Relay: Send permission for {peer_addr} was already granted");

                        self.service_snd
                            .send(ServiceMessage::RelayPeerGranted(peer_addr))
                            .await
                            .anyhow()
                            .into_recoverable()?;
                    } else {
                        println!("Relay: Requesting send permission for {peer_addr}");

                        client
                            .send(MessageToTurnServer::AddPermission(
                                peer_addr,
                                ChannelUsage::WithChannel,
                            ))
                            .await
                            .into_recoverable()?;
                    }
                } else {
                    eprintln!("Relay: Warning: Ignoring permission request while client is not connected yet");

                    self.service_snd
                        .send(ServiceMessage::RelayDisconnected)
                        .await
                        .anyhow()
                        .into_recoverable()?;
                }

                WorkerResult::continued()
            }

            DisconnectAll => {
                if let Some(client) = &mut self.client.0 {
                    println!("Relay: Disconnecting");

                    client
                        .send(MessageToTurnServer::Disconnect)
                        .await
                        .into_recoverable()?;
                }

                WorkerResult::continued()
            }

            TerminateAll => {
                self.will_terminate = true;

                if let Some(client) = &mut self.client.0 {
                    println!("Relay: Disconnecting");

                    client
                        .send(MessageToTurnServer::Disconnect)
                        .await
                        .into_unrecoverable()?;
                }

                WorkerResult::continued()
            }

            ChangeFwdAddr(..) | DisconnectPeer(..) => WorkerResult::continued(),
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
                Ok(WorkerOk::Continue) => {}
                Ok(WorkerOk::Terminate) => break,
                Err(WorkerErr::RecoverableError(error)) => {
                    eprintln!("Relay: Error: {error}");
                }
                Err(WorkerErr::UnrecoverableError(error)) => {
                    eprintln!("Relay: Fatal: {error}");
                    break;
                }
            }
        }

        println!("Relay: Worker stopped");
    }
}
