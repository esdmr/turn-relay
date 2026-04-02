use std::mem::take;
use std::{collections::HashMap, net::SocketAddr};

use crate::control::Control;
use crate::{gui, DEFAULT_FWD_SOCKET};
use futures::channel::mpsc;
use futures::future::join_all;
use tokio::sync::broadcast::{self, error::RecvError};
use tokio::task::JoinHandle;

use crate::result::{TaskErr, TaskOk, TaskResult, TaskResultHelper, ToAnyhowResult, ToTaskErr};
use crate::worker::{CommandMessage, DataMessage, PeerWorker, RelayWorker, ServiceMessage};

pub const DATA_CHANNEL_CAPACITY: usize = u8::MAX as usize;
pub const SERVICE_CHANNEL_CAPACITY: usize = u8::MAX as usize;
pub const COMMAND_CHANNEL_CAPACITY: usize = u8::MAX as usize;

pub struct Coordinator {
    command_snd: broadcast::Sender<CommandMessage>,
    service_snd: broadcast::Sender<ServiceMessage>,
    upstream_snd: mpsc::Sender<DataMessage>,
    downstream_snd: broadcast::Sender<DataMessage>,
    relay: JoinHandle<()>,
    peers: HashMap<String, JoinHandle<()>>,
    control: Option<JoinHandle<()>>,
    fwd_addr: SocketAddr,
}

impl Coordinator {
    pub fn new() -> Self {
        let (command_snd, command_rcv) =
            broadcast::channel::<CommandMessage>(COMMAND_CHANNEL_CAPACITY);
        let (service_snd, _) = broadcast::channel::<ServiceMessage>(SERVICE_CHANNEL_CAPACITY);
        let (upstream_snd, upstream_rcv) = mpsc::channel::<DataMessage>(DATA_CHANNEL_CAPACITY);
        let (downstream_snd, _) = broadcast::channel::<DataMessage>(DATA_CHANNEL_CAPACITY);

        let relay = tokio::spawn(
            RelayWorker::new(
                upstream_rcv,
                downstream_snd.clone(),
                command_rcv,
                service_snd.clone(),
            )
            .start(),
        );

        Self {
            command_snd,
            service_snd,
            upstream_snd,
            downstream_snd,
            relay,
            peers: HashMap::new(),
            control: None,
            fwd_addr: DEFAULT_FWD_SOCKET,
        }
    }

    pub fn run_gui(&mut self) -> impl FnOnce() -> anyhow::Result<()> {
        let service_rcv = self.service_snd.subscribe();
        let command_snd = self.command_snd.clone();

        move || gui::State::run(service_rcv, command_snd).anyhow()
    }

    pub fn run_control(&mut self) {
        let service_rcv = self.service_snd.subscribe();
        let command_snd = self.command_snd.clone();

        self.control = Some(tokio::spawn(Control::new(service_rcv, command_snd).start()));
    }

    async fn handle_command_message(
        &mut self,
        command_message: Result<CommandMessage, RecvError>,
    ) -> TaskResult {
        match command_message.anyhow().into_recoverable()? {
            CommandMessage::ConnectRelay { .. } => TaskResult::continued(),

            CommandMessage::ConnectPeer {
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
                            self.command_snd.subscribe(),
                            self.service_snd.clone(),
                        )
                        .start(),
                    ),
                );

                TaskResult::continued()
            }

            CommandMessage::ChangeFwdAddr(i) => {
                println!("Coordinator: New peers will forward to {i}");
                self.fwd_addr = i;
                TaskResult::continued()
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

                TaskResult::continued()
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
                        .anyhow()
                        .into_recoverable()?;
                }

                TaskResult::continued()
            }

            CommandMessage::TerminateAll => {
                println!("Coordinator: Terminating");

                TaskResult::terminate()
            }
        }
    }

    async fn handle_loop(
        &mut self,
        command_rcv: &mut broadcast::Receiver<CommandMessage>,
    ) -> TaskResult {
        let command_message = command_rcv.recv().await;
        self.handle_command_message(command_message).await
    }

    pub async fn start(mut self) -> anyhow::Result<()> {
        println!("Coordinator: Started");

        let mut command_rcv = self.command_snd.subscribe();
        let mut status: anyhow::Result<()> = Ok(());

        loop {
            match self.handle_loop(&mut command_rcv).await {
                Ok(TaskOk::Continue) => {}
                Ok(TaskOk::Terminate) => break,
                Err(TaskErr::RecoverableError(error)) => {
                    eprintln!("Coordinator: Error: {error}");
                }
                Err(TaskErr::UnrecoverableError(error)) => {
                    status = Err(error.into());
                    break;
                }
            }
        }

        let Coordinator {
            relay,
            peers,
            control,
            ..
        } = self;

        let mut rest = vec![relay];

        if let Some(i) = control {
            rest.push(i);
        }

        println!("Coordinator: Waiting for threads...");

        let handles = peers.into_values().chain(rest);

        for i in join_all(handles).await {
            if let Err(error) = i {
                status = Err(error.into());
            }
        }

        println!("Coordinator: Stopped");

        status
    }
}
