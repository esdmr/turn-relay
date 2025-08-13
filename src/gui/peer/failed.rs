use std::net::SocketAddr;

use iced::{
    widget::{button, horizontal_space, row, text, text_input},
    Element, Length, Task,
};
use tokio::sync::broadcast;

use crate::{gui::types::IcedComponent, worker::CommandMessage};

#[derive(Debug, Clone)]
pub enum Message {
    Delete,
    OnPermissionDenied,
    OnBindFailed,
}

#[derive(Debug, Clone)]
pub struct State {
    pub peer_addr: SocketAddr,
    pub pinned_addr: Option<SocketAddr>,
    permission_denied: bool,
    bind_failed: bool,
}

impl State {
    pub fn compare_peer(&self, other_peer_addr: SocketAddr) -> bool {
        self.peer_addr == other_peer_addr
    }

    pub const fn new_permission_denied(
        peer_addr: SocketAddr,
        pinned_addr: Option<SocketAddr>,
    ) -> Self {
        Self {
            peer_addr,
            pinned_addr,
            permission_denied: true,
            bind_failed: false,
        }
    }

    pub const fn new_bind_failed(peer_addr: SocketAddr, pinned_addr: Option<SocketAddr>) -> Self {
        Self {
            peer_addr,
            pinned_addr,
            permission_denied: false,
            bind_failed: true,
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
        (command_snd, _relay_addr): Self::ExtraUpdateArgs<'_>,
    ) -> Task<Self::TaskMessage> {
        match message {
            Message::Delete => {
                command_snd
                    .send(CommandMessage::DisconnectPeer(self.peer_addr))
                    .unwrap();
            }

            Message::OnPermissionDenied => {
                self.permission_denied = true;
            }

            Message::OnBindFailed => {
                self.bind_failed = true;
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
            if self.permission_denied {
                text!("Authorization Failed")
            } else if self.bind_failed {
                text!("Binding Failed")
            } else {
                text!("Failed")
            }
            .width(Length::Fill),
            horizontal_space().width(8),
            button(text!("X")).on_press(Message::Delete),
        ]
        .into()
    }
}
