use std::net::SocketAddr;

use iced::{
    widget::{button, horizontal_space, row, text, text_input},
    Element, Task,
};
use tokio::sync::broadcast;

use crate::{gui::types::IcedComponent, macros::addr, worker::CommandMessage, LOCAL_IP};

#[derive(Debug, Clone)]
pub enum Message {
    UpdatePeer(String),
    UpdateLocal(String),
    Setup,
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub peer_addr: String,
    pub local_addr: String,
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
        (command_snd, relay_addr): Self::ExtraUpdateArgs<'_>,
    ) -> Task<Self::TaskMessage> {
        match message {
            Message::UpdatePeer(i) => {
                self.peer_addr = i;
            }

            Message::UpdateLocal(i) => {
                self.local_addr = i;
            }

            Message::Setup => {
                let peer_addr = self.peer_addr.trim();

                let peer_addr = match peer_addr.parse() {
                    Ok(i) => i,
                    Err(e) => {
                        if let Ok(i) = peer_addr.parse() {
                            addr!((relay_addr.ip()):i)
                        } else {
                            eprintln!("Invalid peer address {peer_addr}: {e}");
                            return Task::none();
                        }
                    }
                };

                let local_addr = self.local_addr.trim();

                let local_addr = match (!local_addr.is_empty()).then(|| local_addr.parse()) {
                    Some(Ok(i)) => Some(i),
                    Some(Err(e)) => {
                        if let Ok(i) = local_addr.parse() {
                            Some(addr!(LOCAL_IP:i))
                        } else {
                            eprintln!("Invalid local address {peer_addr}: {e}");
                            return Task::none();
                        }
                    }
                    None => None,
                };

                command_snd
                    .send(CommandMessage::ConnectPeer {
                        peer_addr,
                        local_addr,
                    })
                    .unwrap();

                return Task::done(super::Message::ToWaiting {
                    peer_addr,
                    pinned_addr: local_addr,
                });
            }
        }

        Task::none()
    }

    fn view<'a>(&'a self, _index: Self::ExtraViewArgs<'_>) -> Element<'a, Self::Message> {
        row![
            horizontal_space().width(48 + 8),
            text_input("123.45.67.89:12345", &self.peer_addr)
                .on_input(Message::UpdatePeer)
                .on_submit(Message::Setup),
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
