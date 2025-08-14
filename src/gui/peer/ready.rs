use std::net::SocketAddr;

use iced::{
    widget::{button, horizontal_space, row, text, text_input},
    Element, Task,
};
use tokio::sync::broadcast;

use crate::{gui::{peer::waiting, types::IcedComponent}, worker::CommandMessage};

#[derive(Debug, Clone)]
pub enum Message {
    Delete,
    OnBound(SocketAddr),
}

#[derive(Debug, Clone)]
pub struct State {
    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,
    pub pinned: bool,
}

#[allow(clippy::fallible_impl_from)]
impl From<waiting::State> for State {
    fn from(value: waiting::State) -> Self {
        Self {
            peer_addr: value.peer_addr,
            local_addr: value.local_addr.bound_addr().unwrap(),
            pinned: value.local_addr.is_pinned(),
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

            Message::OnBound(i) => {
                self.pinned &= self.local_addr == i;
                self.local_addr = i;
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
            text_input("", format!("{}", self.local_addr).as_ref()),
            horizontal_space().width(8),
            button(text!("X")).on_press(Message::Delete),
        ]
        .into()
    }
}
