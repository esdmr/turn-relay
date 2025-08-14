use crate::gui::types::IcedComponent;
use crate::gui::{peer, relay};
use crate::worker::{
    run, CommandMessage, ServiceMessage, COMMAND_CHANNEL_CAPACITY, SERVICE_CHANNEL_CAPACITY,
};

use iced::widget::{center, container};
use iced::window::{close, close_requests, Id};
use iced::{Element, Subscription, Task};
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum Message {
    OnCloseRequested(Id),
    Relay(relay::Message),
}

impl From<relay::Message> for Message {
    fn from(value: relay::Message) -> Self {
        Self::Relay(value)
    }
}

impl From<ServiceMessage> for Message {
    fn from(value: ServiceMessage) -> Self {
        use peer::Message as P;
        use relay::Message as R;
        use ServiceMessage as S;

        Self::Relay(match value {
            S::RelayAllocated(socket_addr) => R::OnAllocated(socket_addr),
            S::RelayDisconnected => R::OnDisconnected,
            S::RelayConnectionFailed(why) => R::OnConnectionFailed(why),
            S::RelayRedirected(socket_addr) => R::OnRedirect(socket_addr),
            S::RelayPeerGranted(socket_addr) => {
                R::ForPeerByAddr(socket_addr, P::OnPermissionGranted)
            }
            S::RelayPeerDenied(socket_addr) => R::ForPeerByAddr(socket_addr, P::OnPermissionDenied),
            S::PeerBound {
                peer_addr,
                local_addr,
            } => R::ForPeerByAddr(peer_addr, P::OnBound(local_addr)),
            S::PeerUnbound(socket_addr) => R::ForPeerByAddr(socket_addr, P::OnUnbound),
            S::PeerBindFailed(socket_addr) => R::ForPeerByAddr(socket_addr, P::OnBindFailed),
        })
    }
}

#[derive(Debug)]
pub struct State {
    command_snd: broadcast::Sender<CommandMessage>,
    is_relay_connected: bool,
    terminating_window_id: Option<Id>,
    relay: relay::State,
}

impl Default for State {
    fn default() -> Self {
        Self {
            command_snd: broadcast::Sender::<CommandMessage>::new(COMMAND_CHANNEL_CAPACITY),
            is_relay_connected: false,
            terminating_window_id: None,
            relay: relay::State::default(),
        }
    }
}

impl IcedComponent for State {
    type Message = Message;
    type TaskMessage = Message;
    type ExtraUpdateArgs<'a> = ();
    type ExtraViewArgs<'a> = ();
    type ExtraSubscriptionArgs<'a> = ();

    fn update(
        &mut self,
        message: Self::Message,
        _extra: Self::ExtraUpdateArgs<'_>,
    ) -> Task<Self::Message> {
        match &message {
            Message::Relay(relay::Message::OnAllocated(_)) => {
                self.is_relay_connected = true;
            }

            Message::Relay(relay::Message::OnDisconnected) => {
                self.is_relay_connected = false;

                if let Some(id) = self.terminating_window_id {
                    println!("Got relay disconnect event; Closing window");

                    return close(id);
                }
            }

            _ => {}
        }

        match message {
            Message::OnCloseRequested(id) => {
                if !self.is_relay_connected {
                    println!("Got close event for window {id}; Closing window");
                    return close(id);
                }

                self.terminating_window_id = Some(id);

                println!("Got close event for window {id}; Disconnecting the relay");

                self.command_snd.send(CommandMessage::TerminateAll).unwrap();

                Task::none()
            }

            Message::Relay(sub_message) => self
                .relay
                .update(sub_message, &self.command_snd)
                .map(Message::Relay),
        }
    }

    fn view<'a>(&'a self, _extra: Self::ExtraViewArgs<'_>) -> Element<'a, Self::Message> {
        center(container(Element::from(self.relay.view(())).map(Message::Relay)).max_width(512))
            .padding(8)
            .into()
    }

    fn subscription(&self, _extra: Self::ExtraSubscriptionArgs<'_>) -> Subscription<Self::Message> {
        let command_snd = self.command_snd.clone();

        Subscription::batch([
            Subscription::<ServiceMessage>::run_with_id(
                (),
                iced::stream::channel(
                    SERVICE_CHANNEL_CAPACITY,
                    move |service_snd: futures::channel::mpsc::Sender<ServiceMessage>| {
                        run(move || command_snd.subscribe(), service_snd)
                    },
                ),
            )
            .map(Message::from),
            close_requests().map(Message::OnCloseRequested),
        ])
    }
}
