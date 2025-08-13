use std::io;
use std::net::SocketAddr;

use anyhow::{anyhow, ensure};
use bytes::{Bytes, BytesMut};
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;

use crate::worker::types::{
    CommandMessage, DataMessage, ServiceMessage, ToAnyhowResult, ToWorkerErr, WorkerErr, WorkerOk,
    WorkerResult, WorkerResultHelper,
};
use crate::LOCAL_DYN_SOCKET;

#[derive(Debug)]
pub struct PeerWorker {
    peer_addr: SocketAddr,
    pinned_addr: Option<SocketAddr>,
    fwd_addr: SocketAddr,
    upstream_snd: mpsc::Sender<DataMessage>,
    downstream_rcv: broadcast::Receiver<DataMessage>,
    command_rcv: broadcast::Receiver<CommandMessage>,
    service_snd: mpsc::Sender<ServiceMessage>,
    socket: Option<UdpFramed<BytesCodec>>,
    local_addr: SocketAddr,
}

impl PeerWorker {
    pub const fn new(
        peer_addr: SocketAddr,
        pinned_addr: Option<SocketAddr>,
        fwd_addr: SocketAddr,
        upstream_snd: mpsc::Sender<DataMessage>,
        downstream_rcv: broadcast::Receiver<DataMessage>,
        command_rcv: broadcast::Receiver<CommandMessage>,
        service_snd: mpsc::Sender<ServiceMessage>,
    ) -> Self {
        Self {
            peer_addr,
            pinned_addr,
            fwd_addr,
            upstream_snd,
            downstream_rcv,
            command_rcv,
            service_snd,
            socket: None,
            local_addr: LOCAL_DYN_SOCKET,
        }
    }

    async fn setup_socket(&mut self) -> anyhow::Result<()> {
        let socket = UdpSocket::bind(self.pinned_addr.unwrap_or(LOCAL_DYN_SOCKET)).await?;

        self.local_addr = socket.local_addr()?;
        ensure!(
            self.local_addr != self.fwd_addr,
            "Refusing to bind to the forward address"
        );

        self.socket = Some(UdpFramed::new(socket, BytesCodec::new()));

        self.service_snd
            .send(ServiceMessage::PeerBound {
                peer_addr: self.peer_addr,
                local_addr: self.local_addr,
            })
            .await?;

        Ok(())
    }

    async fn handle_socket_message(
        &mut self,
        socket_message: Option<Result<(BytesMut, SocketAddr), io::Error>>,
    ) -> WorkerResult {
        match socket_message {
            Some(Ok((data, src))) => {
                #[cfg(debug_assertions)]
                println!("Peer {} < {} < {}", self.peer_addr, self.local_addr, src);
                #[cfg(not(debug_assertions))]
                let _ = src;

                self.upstream_snd
                    .send((self.peer_addr, data.to_vec()))
                    .await
                    .anyhow()
                    .into_recoverable()?;

                WorkerResult::continued()
            }

            Some(Err(e)) => Err(e).anyhow().into_recoverable(),

            None => {
                eprintln!(
                    "Peer {}: Warning: Socket {} is closed",
                    self.peer_addr, self.local_addr
                );

                self.service_snd
                    .send(ServiceMessage::PeerUnbound(self.peer_addr))
                    .await
                    .anyhow()
                    .into_unrecoverable()?;

                WorkerResult::terminate()
            }
        }
    }

    async fn handle_relay_message(
        &mut self,
        relay_message: Result<(SocketAddr, Vec<u8>), RecvError>,
    ) -> WorkerResult {
        let (src, data) = relay_message.anyhow().into_recoverable()?;

        if src == self.peer_addr {
            #[cfg(debug_assertions)]
            println!(
                "Peer {} > {} > {}",
                self.peer_addr, self.local_addr, self.fwd_addr
            );

            self.socket
                .as_mut()
                .unwrap()
                .send((Bytes::from(data), self.fwd_addr))
                .await
                .anyhow()
                .into_recoverable()?;
        }

        WorkerResult::continued()
    }

    async fn handle_command_message(
        &mut self,
        command_message: Result<CommandMessage, RecvError>,
    ) -> WorkerResult {
        match command_message.anyhow().into_recoverable()? {
            CommandMessage::ConnectRelay { .. } | CommandMessage::ConnectPeer { .. } => {
                WorkerResult::continued()
            }

            CommandMessage::ChangeFwdAddr(i) => {
                self.fwd_addr = i;

                if self.local_addr == self.fwd_addr {
                    Err(anyhow!("Refusing to bind to the forward address")).into_unrecoverable()?;
                }

                println!("Peer {} <> {} <> {}", self.peer_addr, self.local_addr, i);
                WorkerResult::continued()
            }

            CommandMessage::DisconnectAll | CommandMessage::TerminateAll => {
                self.socket = None;

                self.service_snd
                    .send(ServiceMessage::PeerUnbound(self.peer_addr))
                    .await
                    .anyhow()
                    .into_unrecoverable()?;

                WorkerResult::terminate()
            }

            CommandMessage::DisconnectPeer(i) => {
                if self.peer_addr == i {
                    self.socket = None;
                    self.service_snd
                        .send(ServiceMessage::PeerUnbound(self.peer_addr))
                        .await
                        .anyhow()
                        .into_unrecoverable()?;
                    WorkerResult::terminate()
                } else {
                    WorkerResult::continued()
                }
            }
        }
    }

    async fn handle_loop(&mut self) -> WorkerResult {
        select! {
            socket_message = self.socket.as_mut().unwrap().next() => {
                self.handle_socket_message(socket_message).await
            }
            relay_message = self.downstream_rcv.recv() => {
                self.handle_relay_message(relay_message).await
            }
            command_message = self.command_rcv.recv() => {
                self.handle_command_message(command_message).await
            }
        }
    }

    pub async fn start(mut self) {
        if let Err(error) = self.setup_socket().await {
            eprintln!(
                "Peer {} <> {:?}: Failed to bind: {}",
                self.peer_addr, self.pinned_addr, error
            );

            let _ = self
                .service_snd
                .send(ServiceMessage::PeerBindFailed(self.peer_addr))
                .await;

            return;
        }

        println!(
            "Peer {} <> {} <> {}: Worker started",
            self.peer_addr, self.local_addr, self.fwd_addr
        );

        loop {
            match self.handle_loop().await {
                Ok(WorkerOk::Continue) => {}
                Ok(WorkerOk::Terminate) => break,
                Err(WorkerErr::RecoverableError(error)) => {
                    eprintln!("Peer {}: Error: {}", self.peer_addr, error);
                }
                Err(WorkerErr::UnrecoverableError(error)) => {
                    eprintln!("Peer {}: Fatal: {}", self.peer_addr, error);
                    break;
                }
            }
        }

        println!("Peer {}: Worker stopped", self.peer_addr);
    }
}
