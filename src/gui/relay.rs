use std::net::SocketAddr;

use iced::{
    clipboard,
    widget::{button, column, row, text, text_input, Column},
    Element, Task,
};
use tokio::sync::broadcast;

use crate::{
    gui::peer::{PeerEntryMessage, PeerEntryState},
    macros::{addr, get_mut, take_value},
    worker::CommandMessage,
    LOCAL_IP,
};

#[derive(Debug, Clone)]
pub enum RelayMessage {
    UpdateServer(String),
    UpdateUsername(String),
    UpdatePassword(String),
    Connect,
    CopyRelayAddr,
    UpdateFwdAddr(String),
    ChangeFwdAddr,
    AddPeer,
    Disconnect,
    OnAllocate(SocketAddr),
    OnDisconnect,
    OnConnectionFailed(String),
    OnRedirect(SocketAddr),
    Peer(usize, PeerEntryMessage),
    OnPeer(SocketAddr, PeerEntryMessage),
}

#[derive(Debug, Clone)]
pub enum RelayState {
    Disconnected {
        server: String,
        username: String,
        password: String,
    },
    Connecting {
        server: String,
    },
    ConnectionFailed {
        server: String,
        why: String,
    },
    Connected {
        server: String,
        relay_addr: SocketAddr,
        fwd_addr: String,
        peers: Vec<PeerEntryState>,
    },
}

impl Default for RelayState {
    fn default() -> Self {
        Self::Disconnected {
            server: String::new(),
            username: String::new(),
            password: String::new(),
        }
    }
}

impl RelayState {
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    pub fn update(
        &mut self,
        message: RelayMessage,
        command_snd: &broadcast::Sender<CommandMessage>,
    ) -> Task<RelayMessage> {
        use RelayMessage::{
            AddPeer, ChangeFwdAddr, Connect, CopyRelayAddr, Disconnect, OnAllocate,
            OnConnectionFailed, OnDisconnect, OnPeer, OnRedirect, Peer, UpdateFwdAddr,
            UpdatePassword, UpdateServer, UpdateUsername,
        };

        match (&self, message) {
            (Self::Disconnected { .. }, UpdateServer(value)) => {
                *self = Self::Disconnected {
                    server: value,
                    username: take_value!(Self::Disconnected => self.username),
                    password: take_value!(Self::Disconnected => self.password),
                };
            }
            (_, UpdateServer(value)) => {
                unreachable!("invalid message: {:?} @ {:?}", UpdateServer(value), self)
            }

            (Self::Disconnected { .. }, UpdateUsername(value)) => {
                *self = Self::Disconnected {
                    server: take_value!(Self::Disconnected => self.server),
                    username: value,
                    password: take_value!(Self::Disconnected => self.password),
                };
            }
            (_, UpdateUsername(value)) => {
                unreachable!("invalid message: {:?} @ {:?}", UpdateUsername(value), self)
            }

            (Self::Disconnected { .. }, UpdatePassword(value)) => {
                *self = Self::Disconnected {
                    server: take_value!(Self::Disconnected => self.server),
                    username: take_value!(Self::Disconnected => self.username),
                    password: value,
                };
            }
            (_, UpdatePassword(value)) => {
                unreachable!("invalid message: {:?} @ {:?}", UpdatePassword(value), self)
            }

            (Self::Disconnected { server, .. }, Connect) => {
                command_snd
                    .send(CommandMessage::ConnectRelay {
                        server: server.clone(),
                        username: take_value!(Self::Disconnected => self.username),
                        password: take_value!(Self::Disconnected => self.password),
                    })
                    .unwrap();

                *self = Self::Connecting {
                    server: take_value!(Self::Disconnected => self.server),
                }
            }
            (_, Connect) => {
                unreachable!("invalid message: {:?} @ {:?}", Connect, self)
            }

            (Self::Connected { relay_addr, .. }, CopyRelayAddr) => {
                return clipboard::write(format!("{relay_addr}"));
            }
            (_, CopyRelayAddr) => {
                unreachable!("invalid message: {:?} @ {:?}", CopyRelayAddr, self)
            }

            (Self::Connected { .. }, UpdateFwdAddr(value)) => {
                *self = Self::Connected {
                    server: take_value!(Self::Connected => self.server),
                    relay_addr: take_value!(Self::Connected => self.relay_addr: SocketAddr),
                    fwd_addr: value,
                    peers: take_value!(Self::Connected => self.peers),
                }
            }
            (_, UpdateFwdAddr(value)) => {
                unreachable!("invalid message: {:?} @ {:?}", UpdateFwdAddr(value), self)
            }

            (Self::Connected { fwd_addr, .. }, ChangeFwdAddr) => {
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
            (_, ChangeFwdAddr) => {
                unreachable!("invalid message: {:?} @ {:?}", ChangeFwdAddr, self)
            }

            (Self::Connected { .. }, AddPeer) => {
                get_mut!(Self::Connected => self.peers).push(PeerEntryState::EditingPeer {
                    peer_addr: String::default(),
                    local_addr: String::default(),
                });
            }
            (_, AddPeer) => {
                unreachable!("invalid message: {:?} @ {:?}", AddPeer, self)
            }

            (Self::ConnectionFailed { .. }, Disconnect) => {
                *self = Self::Disconnected {
                    server: take_value!(Self::ConnectionFailed => self.server),
                    username: String::default(),
                    password: String::default(),
                };
            }
            (Self::Connected { .. } | Self::Connecting { .. }, Disconnect) => {
                command_snd.send(CommandMessage::DisconnectAll).unwrap();
            }
            (_, Disconnect) => {
                unreachable!("invalid message: {:?} @ {:?}", Disconnect, self)
            }

            (Self::Connecting { .. } | Self::ConnectionFailed { .. }, OnAllocate(relay_addr)) => {
                *self = Self::Connected {
                    server: take_value!(Self::Connecting | Self::ConnectionFailed => self.server),
                    relay_addr,
                    fwd_addr: String::default(),
                    peers: vec![],
                };
            }
            (_, OnAllocate(relay_addr)) => {
                eprintln!("message ignored: {:?} @ {:?}", OnAllocate(relay_addr), self);
            }

            (
                Self::Connected { .. } | Self::Connecting { .. } | Self::ConnectionFailed { .. },
                OnDisconnect,
            ) => {
                *self = Self::Disconnected {
                    server: take_value!(Self::Connected | Self::Connecting | Self::ConnectionFailed => self.server),
                    username: String::default(),
                    password: String::default(),
                };
            }
            (_, OnDisconnect) => {
                eprintln!("message ignored: {OnDisconnect:?} @ {self:?}");
            }

            (
                Self::Connected { .. } | Self::Connecting { .. } | Self::ConnectionFailed { .. },
                OnConnectionFailed(why),
            ) => {
                *self = Self::ConnectionFailed {
                    server: take_value!(Self::Connected | Self::Connecting | Self::ConnectionFailed => self.server),
                    why,
                };
            }
            (_, OnConnectionFailed(why)) => {
                eprintln!(
                    "message ignored: {:?} @ {:?}",
                    OnConnectionFailed(why),
                    self
                );
            }

            (Self::Disconnected { .. }, OnRedirect(server)) => {
                *self = Self::Disconnected {
                    server: format!("{server}"),
                    username: take_value!(Self::Disconnected => self.username),
                    password: take_value!(Self::Disconnected => self.password),
                };
            }
            (Self::Connecting { .. }, OnRedirect(server)) => {
                *self = Self::Connecting {
                    server: format!("{server}"),
                };
            }
            (Self::ConnectionFailed { .. }, OnRedirect(server)) => {
                *self = Self::ConnectionFailed {
                    server: format!("{server}"),
                    why: take_value!(Self::ConnectionFailed => self.why),
                };
            }
            (Self::Connected { .. }, OnRedirect(server)) => {
                *self = Self::Connected {
                    server: format!("{server}"),
                    relay_addr: take_value!(Self::Connected => self.relay_addr: SocketAddr),
                    fwd_addr: take_value!(Self::Connected => self.fwd_addr),
                    peers: take_value!(Self::Connected => self.peers),
                };
            }

            (Self::Connected { .. }, Peer(index, sub_message)) => {
                return get_mut!(Self::Connected => self.peers)[index]
                    .update(sub_message, command_snd)
                    .map(move |i| RelayMessage::Peer(index, i));
            }
            (_, Peer(index, sub_message)) => {
                unreachable!(
                    "invalid message: {:?} @ {:?}",
                    Peer(index, sub_message),
                    self
                )
            }

            (Self::Connected { .. }, OnPeer(peer_addr, sub_message)) => {
                if let Some((index, peer)) = get_mut!(Self::Connected => self.peers)
                    .iter_mut()
                    .enumerate()
                    .find(|(_, i)| i.compare_peer(&peer_addr))
                {
                    return peer
                        .update(sub_message, command_snd)
                        .map(move |i| RelayMessage::Peer(index, i));
                } else {
                    eprintln!(
                        "non-existent peer ignored: {:?} @ {:?}",
                        OnPeer(peer_addr, sub_message),
                        self
                    );
                }
            }
            (_, OnPeer(peer_addr, sub_message)) => {
                eprintln!(
                    "message ignored: {:?} @ {:?}",
                    OnPeer(peer_addr, sub_message),
                    self
                );
            }
        }

        Task::none()
    }

    pub fn view(&self) -> Column<RelayMessage> {
        use RelayMessage::{
            AddPeer, ChangeFwdAddr, Connect, CopyRelayAddr, Disconnect, Peer, UpdateFwdAddr,
            UpdatePassword, UpdateServer, UpdateUsername,
        };

        match self {
            Self::Disconnected {
                server,
                username,
                password,
            } => column![
                row![
                    text!("Server: "),
                    text_input("example.com:12345", server).on_input(UpdateServer),
                ],
                row![
                    text!("Username: "),
                    text_input("12345:user", username).on_input(UpdateUsername),
                ],
                row![
                    text!("Password: "),
                    text_input("abc123", password).on_input(UpdatePassword),
                ],
                button(text!("Connect")).on_press(Connect),
            ],
            Self::Connecting { server: _ } => {
                column![
                    text!("Connecting..."),
                    button(text!("Cancel")).on_press(Disconnect),
                ]
            }
            Self::ConnectionFailed { server, why } => {
                column![
                    text!("Connection to {} failed.", server),
                    text!("{}", why),
                    button(text!("Back")).on_press(Disconnect),
                ]
            }
            Self::Connected {
                server: _,
                relay_addr,
                fwd_addr,
                peers,
            } => column![
                row![
                    text!("Available at {}.", relay_addr),
                    button(text!("Copy")).on_press(CopyRelayAddr),
                    button(text!("Disconnect")).on_press(Disconnect),
                ],
                row![
                    text!("Forward to "),
                    text_input("127.0.0.1:12345", fwd_addr).on_input(UpdateFwdAddr),
                    button(text!("Apply")).on_press(ChangeFwdAddr),
                ],
                row![
                    text!("Peers"),
                    button(text!("Add")).on_press_maybe(
                        peers.iter().all(|i| !i.is_uncommitted()).then_some(AddPeer)
                    ),
                ],
                column(peers.iter().enumerate().map(|(index, peer)| {
                    Element::from(peer.view(index)).map(move |i| Peer(index, i))
                })),
            ],
        }
    }
}
