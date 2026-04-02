mod component;
mod peer;
mod relay;
mod router;
mod socket_state;
mod stream;

use crate::gui::component::IcedComponent;
use crate::gui::stream::BroadcastStream;
use crate::worker::{CommandMessage, ServiceMessage};

use iced::widget::{center, container};
use iced::window::{self, close, close_requests, Id};
use iced::{application, Element, Settings, Size, Subscription, Task};
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

impl State {
    fn new(command_snd: broadcast::Sender<CommandMessage>) -> Self {
        Self {
            command_snd,
            is_relay_connected: false,
            terminating_window_id: None,
            relay: relay::State::default(),
        }
    }

    fn update_root(&mut self, message: Message) -> Task<Message> {
        self.update(message, ())
    }

    fn view_root<'a>(&'a self) -> Element<'a, Message> {
        self.view(())
    }

    fn subscription_root(
        service_rcv_stream: BroadcastStream<ServiceMessage>,
    ) -> impl Fn(&State) -> Subscription<Message> {
        move |i: &State| i.subscription(service_rcv_stream.clone())
    }

    pub fn run(
        service_rcv: broadcast::Receiver<ServiceMessage>,
        command_snd: broadcast::Sender<CommandMessage>,
    ) -> iced::Result {
        let service_rcv_stream = BroadcastStream::new(service_rcv);

        application("TURN Relay", Self::update_root, Self::view_root)
            .subscription(Self::subscription_root(service_rcv_stream))
            .settings(Settings {
                id: Some("turn_relay".to_string()),
                ..Default::default()
            })
            .window(window::Settings {
                exit_on_close_request: false,
                min_size: Some(Size::new(456., 456.)),
                ..Default::default()
            })
            .run_with(move || (Self::new(command_snd), Task::none()))
    }
}

impl IcedComponent for State {
    type Message = Message;
    type TaskMessage = Message;
    type ExtraUpdateArgs<'a> = ();
    type ExtraViewArgs<'a> = ();
    type ExtraSubscriptionArgs<'a> = BroadcastStream<ServiceMessage>;

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

    fn subscription(&self, extra: Self::ExtraSubscriptionArgs<'_>) -> Subscription<Self::Message> {
        println!("GUI: DEBUG: subscription fn was called");
        Subscription::batch([
            Subscription::run_with_id((), extra).map(Message::from),
            close_requests().map(Message::OnCloseRequested),
        ])
    }
}
