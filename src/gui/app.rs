use crate::gui::peer::PeerEntryMessage;
use crate::gui::relay::{RelayMessage, RelayState};
use crate::worker::{
    run_worker, CommandMessage, ServiceMessage, COMMAND_CHANNEL_CAPACITY, SERVICE_CHANNEL_CAPACITY,
};

use iced::{Element, Subscription};
use tokio::sync::broadcast;

#[derive(Debug)]
pub struct App {
    command_snd: broadcast::Sender<CommandMessage>,
    state: RelayState,
}

impl Default for App {
    fn default() -> Self {
        App {
            command_snd: broadcast::Sender::<CommandMessage>::new(COMMAND_CHANNEL_CAPACITY),
            state: RelayState::default(),
        }
    }
}

impl App {
    pub fn update(&mut self, message: RelayMessage) {
        self.state.update(message, &self.command_snd);
    }

    pub fn view(&self) -> Element<RelayMessage> {
        Element::from(self.state.view())
    }

    pub fn subscribe(&self) -> Subscription<RelayMessage> {
        let command_snd = self.command_snd.clone();

        Subscription::<ServiceMessage>::run_with_id(
            (),
            iced::stream::channel(SERVICE_CHANNEL_CAPACITY, move |service_snd| {
                run_worker(move || command_snd.subscribe(), service_snd)
            }),
        )
        .map(|i| {
            use PeerEntryMessage as Peer;
            use RelayMessage::*;
            use ServiceMessage::*;

            match i {
                RelayAllocated(socket_addr) => OnAllocate(socket_addr),
                RelayDisconnected => OnDisconnect,
                RelayConnectionFailed(why) => OnConnectionFailed(why),
                RelayRedirected(socket_addr) => OnRedirect(socket_addr),
                RelayPeerGranted(socket_addr) => OnPeer(socket_addr, Peer::OnPermissionGranted),
                RelayPeerDenied(socket_addr) => OnPeer(socket_addr, Peer::OnPermissionDenied),
                PeerBound {
                    peer_addr,
                    local_addr,
                } => OnPeer(peer_addr, Peer::OnBound(local_addr)),
                PeerUnbound(socket_addr) => OnPeer(socket_addr, Peer::OnUnbound),
                PeerBindFailed(socket_addr) => OnPeer(socket_addr, Peer::OnBindFailed),
            }
        })
    }
}
