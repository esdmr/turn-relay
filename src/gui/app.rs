use crate::gui::peer::PeerEntryMessage;
use crate::gui::relay::{RelayMessage, RelayState};
use crate::worker::{
    run_worker, CommandMessage, ServiceMessage, COMMAND_CHANNEL_CAPACITY, SERVICE_CHANNEL_CAPACITY,
};

use iced::window::{close, close_requests, Id};
use iced::{Element, Subscription, Task};
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum AppMessage {
    OnCloseRequested(Id),
    Relay(RelayMessage),
}

#[derive(Debug)]
pub struct App {
    command_snd: broadcast::Sender<CommandMessage>,
    is_relay_connected: bool,
    terminating_window_id: Option<Id>,
    state: RelayState,
}

impl Default for App {
    fn default() -> Self {
        Self {
            command_snd: broadcast::Sender::<CommandMessage>::new(COMMAND_CHANNEL_CAPACITY),
            is_relay_connected: false,
            terminating_window_id: None,
            state: RelayState::default(),
        }
    }
}

impl App {
    pub fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match &message {
            AppMessage::Relay(RelayMessage::OnAllocate(_)) => {
                self.is_relay_connected = true;
            }

            AppMessage::Relay(RelayMessage::OnDisconnect) => {
                self.is_relay_connected = false;

                if let Some(id) = self.terminating_window_id {
                    println!("Got relay disconnect event; Closing window");

                    return close(id);
                }
            }

            _ => {}
        }

        match message {
            AppMessage::OnCloseRequested(id) => {
                if !self.is_relay_connected {
                    println!("Got close event for window {id}; Closing window");
                    return close(id);
                }

                self.terminating_window_id = Some(id);

                println!("Got close event for window {id}; Disconnecting the relay");

                self.command_snd
                    .send(CommandMessage::TerminateAll)
                    .unwrap();

                Task::none()
            }

            AppMessage::Relay(sub_message) => {
                self.state.update(sub_message, &self.command_snd).map(AppMessage::Relay)
            }
        }
    }

    pub fn view(&self) -> Element<AppMessage> {
        Element::from(self.state.view()).map(AppMessage::Relay)
    }

    pub fn subscribe(&self) -> Subscription<AppMessage> {
        let command_snd = self.command_snd.clone();

        Subscription::batch([
            Subscription::<ServiceMessage>::run_with_id(
                (),
                iced::stream::channel(SERVICE_CHANNEL_CAPACITY, move |service_snd: futures::channel::mpsc::Sender<ServiceMessage>| {
                    run_worker(move || command_snd.subscribe(), service_snd)
                }),
            )
            .map(|i| {
                use PeerEntryMessage as Peer;
                use RelayMessage::{OnAllocate, OnConnectionFailed, OnDisconnect, OnPeer, OnRedirect};
                use ServiceMessage::{
                    PeerBindFailed, PeerBound, PeerUnbound, RelayAllocated, RelayConnectionFailed,
                    RelayDisconnected, RelayPeerDenied, RelayPeerGranted, RelayRedirected,
                };

                AppMessage::Relay(match i {
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
                })
            }),
            close_requests().map(AppMessage::OnCloseRequested),
        ])
    }
}
