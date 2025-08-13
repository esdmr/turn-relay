mod connected;
mod connecting;
mod connection_failed;
mod disconnected;

use std::{mem::replace, net::SocketAddr};

use iced::{Element, Task};
use tokio::sync::broadcast;

use crate::{
    gui::{peer, types::IcedComponent},
    worker::CommandMessage,
};

#[derive(Debug, Clone)]
pub enum Message {
    ForDisconnected(disconnected::Message),
    ForConnecting(connecting::Message),
    ForConnectionFailed(connection_failed::Message),
    ForConnected(connected::Message),
    ForPeerByAddr(SocketAddr, peer::Message),
    ToDisconnected,
    ToConnecting,
    OnAllocated(SocketAddr),
    OnDisconnected,
    OnConnectionFailed(String),
    OnRedirect(SocketAddr),
}

impl From<disconnected::Message> for Message {
    fn from(value: disconnected::Message) -> Self {
        Self::ForDisconnected(value)
    }
}

impl From<connecting::Message> for Message {
    fn from(value: connecting::Message) -> Self {
        Self::ForConnecting(value)
    }
}

impl From<connection_failed::Message> for Message {
    fn from(value: connection_failed::Message) -> Self {
        Self::ForConnectionFailed(value)
    }
}

impl From<connected::Message> for Message {
    fn from(value: connected::Message) -> Self {
        Self::ForConnected(value)
    }
}

#[derive(Debug, Clone)]
pub enum State {
    Disconnected(disconnected::State),
    Connecting(connecting::State),
    ConnectionFailed(connection_failed::State),
    Connected(connected::State),
    Intermediate,
}

impl Default for State {
    fn default() -> Self {
        Self::Disconnected(disconnected::State::default())
    }
}

impl IcedComponent for State {
    type Message = Message;
    type TaskMessage = Message;
    type ExtraUpdateArgs<'a> = &'a broadcast::Sender<CommandMessage>;
    type ExtraViewArgs<'a> = ();
    type ExtraSubscriptionArgs<'a> = ();

    fn update(
        &mut self,
        message: Self::Message,
        command_snd: Self::ExtraUpdateArgs<'_>,
    ) -> Task<Self::TaskMessage> {
        match (replace(self, Self::Intermediate), message) {
            // State transitions
            (Self::Disconnected(disconnected::State { server, .. }), Message::ToConnecting) => {
                *self = Self::Connecting(connecting::State::new(server));
            }

            (
                Self::Connecting(connecting::State { server, .. })
                | Self::ConnectionFailed(connection_failed::State { server, .. }),
                Message::ToDisconnected,
            )
            | (
                Self::Connecting(connecting::State { server, .. })
                | Self::Connected(connected::State { server, .. }),
                Message::OnDisconnected,
            ) => {
                *self = Self::Disconnected(disconnected::State::new(server));
            }

            (
                Self::Connecting(connecting::State { server, .. }),
                Message::OnAllocated(relay_addr),
            ) => {
                *self = Self::Connected(connected::State::new(server, relay_addr));
            }

            (
                Self::Connecting(connecting::State { server, .. })
                | Self::Connected(connected::State { server, .. }),
                Message::OnConnectionFailed(why),
            ) => {
                *self = Self::ConnectionFailed(connection_failed::State::new(server, why));
            }

            (Self::Connecting(_), Message::OnRedirect(server)) => {
                *self = Self::Disconnected(disconnected::State::new(format!("{server}")));
            }

            // Inner-state messages
            (Self::Disconnected(mut state), Message::ForDisconnected(message)) => {
                let task = state.update(message, command_snd);
                *self = Self::Disconnected(state);
                return task;
            }

            (Self::Connecting(mut state), Message::ForConnecting(message)) => {
                let task = state.update(message, command_snd);
                *self = Self::Connecting(state);
                return task;
            }

            #[allow(unreachable_patterns)]
            (Self::ConnectionFailed(mut state), Message::ForConnectionFailed(message)) => {
                let task = state.update(message, command_snd);
                *self = Self::ConnectionFailed(state);
                return task;
            }

            (Self::Connected(mut state), Message::ForConnected(message)) => {
                let task = state.update(message, command_snd);
                *self = Self::Connected(state);
                return task;
            }

            (Self::Connected(mut state), Message::ForPeerByAddr(j, message)) => {
                let task = state.update(connected::Message::ForPeerByAddr(j, message), command_snd);
                *self = Self::Connected(state);
                return task;
            }

            // Invalid states or messages
            (Self::Intermediate, _) => {
                unreachable!("Relay UI state is in an intermediate state");
            }

            (state, message) => {
                eprintln!("Ignoring message: {message:?} @ {state:?}");
                *self = state;
            }
        }

        Task::none()
    }

    fn view<'a>(&'a self, _extra: Self::ExtraViewArgs<'_>) -> Element<'a, Self::Message> {
        match self {
            Self::Disconnected(i) => i.view(()).map(Message::ForDisconnected),
            Self::Connecting(i) => i.view(()).map(Message::ForConnecting),
            Self::ConnectionFailed(i) => i.view(()).map(Message::ForConnectionFailed),
            Self::Connected(i) => i.view(()).map(Message::ForConnected),

            Self::Intermediate => {
                unreachable!("Relay UI state is in an intermediate state");
            }
        }
    }
}
