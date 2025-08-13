use std::net::SocketAddr;

use iced::{
    widget::{button, horizontal_space, row, text, text_input},
    Element, Length, Task,
};
use tokio::sync::broadcast;

use crate::{
    gui::{peer::types::SocketState, types::IcedComponent},
    worker::CommandMessage,
};

#[derive(Debug, Clone)]
pub enum Message {
    Delete,
    OnPermissionGranted,
    OnBound(SocketAddr),
}

#[derive(Debug, Clone)]
pub struct State {
    pub peer_addr: SocketAddr,
    pub local_addr: SocketState,
    pub authorized: bool,
}

impl State {
    pub fn compare_peer(&self, other_peer_addr: SocketAddr) -> bool {
        self.peer_addr == other_peer_addr
    }

    pub fn new(peer_addr: SocketAddr, pinned_addr: Option<SocketAddr>) -> Self {
        Self {
            peer_addr,
            local_addr: SocketState::default().with_pin(pinned_addr),
            authorized: false,
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

            Message::OnPermissionGranted => {
                self.authorized = true;

                if self.local_addr.is_bound() {
                    return Task::done(super::Message::ToReady);
                }
            }

            Message::OnBound(i) => {
                self.local_addr.bind(Some(i));

                if self.authorized {
                    return Task::done(super::Message::ToReady);
                }
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
            if !self.authorized {
                text!("Authorizing...")
            } else if self.local_addr.is_unbound() {
                text!("Binding...")
            } else {
                text!("Waiting...")
            }
            .width(Length::Fill),
            horizontal_space().width(8),
            button(text!("X")).on_press(Message::Delete),
        ]
        .into()
    }
}
