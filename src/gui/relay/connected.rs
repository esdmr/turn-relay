use std::net::SocketAddr;

use iced::widget::{
    button, column, horizontal_space, row, scrollable, text, text_input, vertical_space,
};
use iced::{clipboard, Element, Length, Task};
use tokio::sync::broadcast;

use crate::gui::peer;
use crate::gui::types::IcedComponent;
use crate::macros::addr;
use crate::worker::CommandMessage;
use crate::LOCAL_IP;

#[derive(Debug, Clone)]
pub enum Message {
    CopyRelayAddr,
    Disconnect,
    UpdateFwdAddr(String),
    ChangeFwdAddr,
    AddPeer,
    ForPeerByIndex(usize, peer::Message),
    ForPeerByAddr(SocketAddr, peer::Message),
}

#[derive(Debug, Clone)]
pub struct State {
    pub server: String,
    relay_addr: SocketAddr,
    fwd_addr: String,
    peers: Vec<peer::State>,
}

impl State {
    pub const fn new(server: String, relay_addr: SocketAddr) -> Self {
        Self {
            server,
            relay_addr,
            fwd_addr: String::new(),
            peers: vec![],
        }
    }
}

impl IcedComponent for State {
    type Message = Message;
    type TaskMessage = super::Message;
    type ExtraUpdateArgs<'a> = &'a broadcast::Sender<CommandMessage>;
    type ExtraViewArgs<'a> = ();
    type ExtraSubscriptionArgs<'a> = ();

    fn update(
        &mut self,
        message: Self::Message,
        command_snd: Self::ExtraUpdateArgs<'_>,
    ) -> Task<Self::TaskMessage> {
        match message {
            Message::CopyRelayAddr => {
                return clipboard::write(format!("{}", self.relay_addr));
            }

            Message::Disconnect => {
                command_snd.send(CommandMessage::DisconnectAll).unwrap();
            }

            Message::UpdateFwdAddr(i) => {
                self.fwd_addr = i;
            }

            Message::ChangeFwdAddr => {
                let fwd_addr = self.fwd_addr.trim();

                let addr = match fwd_addr.parse() {
                    Ok(addr) => addr,
                    Err(e) => {
                        if let Ok(i) = fwd_addr.parse() {
                            addr!(LOCAL_IP:i)
                        } else {
                            eprintln!("Invalid forward address {fwd_addr}: {e}");
                            return Task::none();
                        }
                    }
                };

                command_snd
                    .send(CommandMessage::ChangeFwdAddr(addr))
                    .unwrap();
            }

            Message::AddPeer => {
                self.peers.push(peer::State::default());
            }

            Message::ForPeerByIndex(index, message) => {
                return self.peers[index]
                    .update(message, (command_snd, self.relay_addr))
                    .map(move |i| super::Message::ForPeerByIndex(index, i));
            }

            Message::ForPeerByAddr(peer_addr, message) => {
                if let Some((index, peer)) = self
                    .peers
                    .iter_mut()
                    .enumerate()
                    .find(|(_, i)| i.compare_peer(peer_addr))
                {
                    return peer
                        .update(message, (command_snd, self.relay_addr))
                        .map(move |i| super::Message::ForPeerByIndex(index, i));
                }

                eprintln!("non-existent peer {peer_addr} ignored: {message:?} @ {self:?}");
            }
        }

        Task::none()
    }

    fn view<'a>(&'a self, _extra: Self::ExtraViewArgs<'_>) -> Element<'a, Self::Message> {
        column![
            row![
                text!("Available at").width(96),
                horizontal_space().width(8),
                text_input("", format!("{}", self.relay_addr).as_ref()),
                horizontal_space().width(8),
                button(text!("Copy")).on_press(Message::CopyRelayAddr),
                horizontal_space().width(8),
                button(text!("Disconnect")).on_press(Message::Disconnect),
            ],
            vertical_space().height(8),
            row![
                text!("Forward to").width(96),
                horizontal_space().width(8),
                text_input("127.0.0.1:12345", &self.fwd_addr)
                    .on_input(Message::UpdateFwdAddr)
                    .on_submit(Message::ChangeFwdAddr),
                horizontal_space().width(8),
                button(text!("Apply")).on_press(Message::ChangeFwdAddr),
            ],
            vertical_space().height(24),
            row![
                text!("Peers").width(48),
                horizontal_space().width(8),
                button(text!("Add")).on_press_maybe(
                    self.peers
                        .iter()
                        .all(|i| !i.is_uncommitted())
                        .then_some(Message::AddPeer)
                ),
            ],
            vertical_space().height(8),
            scrollable(column(self.peers.iter().enumerate().map(
                |(index, peer)| {
                    column![
                        vertical_space().height(if index > 0 { 8 } else { 0 }),
                        Element::from(peer.view(index))
                            .map(move |i| Message::ForPeerByIndex(index, i)),
                    ]
                    .into()
                }
            )))
            .height(Length::Fill),
        ]
        .into()
    }
}
