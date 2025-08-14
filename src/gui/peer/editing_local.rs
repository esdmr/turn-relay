use std::net::SocketAddr;

use iced::{
    widget::{button, horizontal_space, row, text, text_input},
    Element, Task,
};
use tokio::sync::broadcast;

use crate::{
    gui::{
        peer::{failed, ready, waiting},
        types::IcedComponent,
    },
    macros::addr,
    worker::CommandMessage,
    LOCAL_IP,
};

#[derive(Debug, Clone)]
pub enum Message {
    UpdateLocal(String),
    Setup,
}

#[derive(Debug, Clone)]
pub struct State {
    pub peer_addr: SocketAddr,
    local_addr: String,
}

impl From<waiting::State> for State {
    fn from(value: waiting::State) -> Self {
        Self {
            peer_addr: value.peer_addr,
            local_addr: value
                .local_addr
                .pinned_addr()
                .map_or_else(String::new, |i| format!("{i}")),
        }
    }
}

impl From<failed::State> for State {
    fn from(value: failed::State) -> Self {
        Self {
            peer_addr: value.peer_addr,
            local_addr: value
                .pinned_addr
                .map_or_else(String::new, |i| format!("{i}")),
        }
    }
}

impl From<ready::State> for State {
    fn from(value: ready::State) -> Self {
        Self {
            peer_addr: value.peer_addr,
            local_addr: value
                .pinned
                .then_some(value.local_addr)
                .map_or_else(String::new, |i| format!("{i}")),
        }
    }
}

impl IcedComponent for State {
    type Message = Message;
    type TaskMessage = super::Message;
    type ExtraUpdateArgs<'a> = (&'a broadcast::Sender<CommandMessage>, SocketAddr);
    type ExtraViewArgs<'a> = usize;
    type ExtraSubscriptionArgs<'a> = ();

    fn update(
        &mut self,
        message: Self::Message,
        _extra: Self::ExtraUpdateArgs<'_>,
    ) -> Task<Self::TaskMessage> {
        match message {
            Message::UpdateLocal(i) => {
                self.local_addr = i;
            }

            Message::Setup => {
                let local_addr = self.local_addr.trim();

                let local_addr =
                    match (!local_addr.is_empty()).then(|| local_addr.parse::<SocketAddr>()) {
                        Some(Ok(i)) => Some(i),
                        Some(Err(e)) => {
                            if let Ok(i) = local_addr.parse() {
                                Some(addr!(LOCAL_IP:i))
                            } else {
                                eprintln!("Invalid local address {local_addr}: {e}");
                                return Task::none();
                            }
                        }
                        None => None,
                    };

                return Task::done(super::Message::ToWaiting {
                    peer_addr: self.peer_addr,
                    pinned_addr: local_addr,
                });
            }
        }

        Task::none()
    }

    fn view<'a>(&'a self, index: Self::ExtraViewArgs<'_>) -> Element<'a, Self::Message> {
        row![
            text!("{})", index + 1).width(48),
            horizontal_space().width(8),
            text_input("", format!("{}", self.peer_addr).as_ref()),
            horizontal_space().width(8),
            text_input("127.0.0.1:12345", &self.local_addr)
                .on_input(Message::UpdateLocal)
                .on_submit(Message::Setup),
            horizontal_space().width(8),
            button(text!("+")).on_press(Message::Setup),
        ]
        .into()
    }
}
