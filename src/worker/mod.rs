mod peer;
mod relay;
mod types;

use std::{collections::HashMap, net::SocketAddr};

use futures::channel::mpsc;
use futures::future::join_all;
use tokio::sync::broadcast;

use crate::{worker::{peer::PeerWorker, relay::RelayWorker, types::DataMessage}, DEFAULT_FWD_SOCKET_ADDR};
pub use crate::worker::types::{CommandMessage, ServiceMessage};

pub const DATA_CHANNEL_CAPACITY: usize = u8::MAX as usize;
pub const SERVICE_CHANNEL_CAPACITY: usize = u8::MAX as usize;
pub const COMMAND_CHANNEL_CAPACITY: usize = u8::MAX as usize;

pub async fn handle_coordinator<F: FnMut() -> broadcast::Receiver<CommandMessage>>(
    mut subscribe_command: F,
    service_snd: mpsc::Sender<ServiceMessage>,
) -> anyhow::Result<()> {
    let (upstream_snd, upstream_rcv) = mpsc::channel::<DataMessage>(DATA_CHANNEL_CAPACITY);
    let (downstream_snd, _) = broadcast::channel::<DataMessage>(DATA_CHANNEL_CAPACITY);

    let relay = tokio::spawn(RelayWorker::new(
        upstream_rcv,
        downstream_snd.clone(),
        subscribe_command(),
        service_snd.clone(),
    ).start());

    let mut peers: HashMap<String, tokio::task::JoinHandle<()>> =
        HashMap::new();
    let mut command_rcv = subscribe_command();
    let mut fwd_addr: SocketAddr = DEFAULT_FWD_SOCKET_ADDR;

    loop {
        use CommandMessage::*;

        match command_rcv.recv().await? {
            ConnectRelay { .. } => {}
            ConnectPeer {
                peer_addr,
                local_addr,
            } => {
                peers.insert(
                    peer_addr.to_string(),
                    tokio::spawn(PeerWorker::new(
                        peer_addr,
                        local_addr,
                        fwd_addr,
                        upstream_snd.clone(),
                        downstream_snd.subscribe(),
                        subscribe_command(),
                        service_snd.clone(),
                    ).start()),
                );
            }
            ChangeFwdAddr(fwd_addr_) => {
                fwd_addr = fwd_addr_;
            }
            DisconnectAll => {
                join_all(peers.into_values())
                    .await
                    .into_iter()
                    .collect::<Result<Vec<()>, _>>()?;

                peers = HashMap::new();
            }
            DisconnectPeer(peer_addr) => {
                if let Some(peer) = peers.remove(&peer_addr.to_string()) {
                    peer.await?;
                }
            }
            TerminateAll => {
                join_all(peers.into_values())
                    .await
                    .into_iter()
                    .collect::<Result<Vec<()>, _>>()?;

                relay.await?;

                return Ok(());
            }
        }
    }
}
