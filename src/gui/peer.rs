use std::{fmt::Display, net::SocketAddr, ops::Not};

use iced::widget::{button, row, text, text_input, Row};
use tokio::sync::broadcast;

use crate::{macros::take_value, worker::CommandMessage, DEFAULT_SOCKET_ADDR};

#[derive(Debug, Clone)]
pub enum PeerEntryMessage {
    Delete,
    Setup,
    UpdatePeer(String),
    UpdateLocal(String),
    OnPermissionGranted,
    OnPermissionDenied,
    OnBound(SocketAddr),
    OnUnbound,
    OnBindFailed,
}

#[derive(Debug, Clone)]
pub enum PeerLocalState {
    Unbound(Option<SocketAddr>),
    Bound(SocketAddr, bool),
}

impl Default for PeerLocalState {
    fn default() -> Self {
        Self::Unbound(None)
    }
}

impl Display for PeerLocalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unbound(_) => write!(f, "Unbound"),
            Self::Bound(addr, _) => write!(f, "{}", addr),
        }
    }
}

impl PeerLocalState {
    fn assign(&mut self, local_addr: SocketAddr) {
        match &self {
            Self::Unbound(None) | Self::Bound(_, false) => {
                *self = Self::Bound(local_addr, false);
            }
            Self::Unbound(Some(addr)) | Self::Bound(addr, true) => {
                *self = Self::Bound(local_addr, *addr == local_addr);
            }
        }
    }

    fn with_assign(mut self, local_addr: SocketAddr) -> Self {
        self.assign(local_addr);
        self
    }
}

#[derive(Debug, Clone)]
pub enum PeerEntryState {
    EditingPeer {
        peer_addr: String,
        local_addr: String,
    },
    EditingLocal {
        peer_addr: SocketAddr,
        local_addr: String,
    },
    Waiting {
        peer_addr: SocketAddr,
        local_addr: PeerLocalState,
        authorized: bool,
    },
    Failed {
        peer_addr: SocketAddr,
        local_addr: PeerLocalState,
        authorized: bool,
    },
    Ready {
        peer_addr: SocketAddr,
        local_addr: PeerLocalState,
    },
}

impl Default for PeerEntryState {
    fn default() -> Self {
        Self::EditingPeer {
            peer_addr: String::default(),
            local_addr: String::default(),
        }
    }
}

impl PeerEntryState {
    pub fn compare_peer(&self, other_peer_addr: &SocketAddr) -> bool {
        match self {
            Self::EditingLocal { peer_addr, .. }
            | Self::Waiting { peer_addr, .. }
            | Self::Failed { peer_addr, .. }
            | Self::Ready { peer_addr, .. } => *peer_addr == *other_peer_addr,
            Self::EditingPeer { .. } => false,
        }
    }

    pub fn is_uncommitted(&self) -> bool {
        matches!(self, Self::EditingPeer { .. })
    }

    pub fn update(
        &mut self,
        message: PeerEntryMessage,
        command_snd: &broadcast::Sender<CommandMessage>,
    ) {
        use PeerEntryMessage::*;

        match (&self, message) {
            (
                Self::Waiting { peer_addr, .. }
                | Self::Failed { peer_addr, .. }
                | Self::Ready { peer_addr, .. },
                Delete,
            ) => {
                command_snd
                    .send(CommandMessage::DisconnectPeer(*peer_addr))
                    .unwrap();

                *self = Self::EditingLocal {
                    peer_addr: *peer_addr,
                    local_addr: String::default(),
                }
            }
            (_, Delete) => {
                unreachable!("invalid message: {:?} @ {:?}", Delete, self)
            }
            (
                Self::EditingPeer {
                    peer_addr,
                    local_addr,
                },
                Setup,
            ) => {
                let peer_addr = match peer_addr.parse::<SocketAddr>() {
                    Ok(i) => i,
                    Err(e) => {
                        eprintln!("Invalid peer address {}: {}", peer_addr, e);
                        return;
                    },
                };

                let local_addr = match local_addr
                    .trim()
                    .is_empty()
                    .not()
                    .then(|| local_addr.parse::<SocketAddr>())
                {
                    Some(Ok(i)) => Some(i),
                    Some(Err(e)) => {
                        if let Ok(i) = local_addr.parse() {
                            Some(SocketAddr::new(DEFAULT_SOCKET_ADDR.ip(), i))
                        } else {
                            eprintln!("Invalid local address {}: {}", peer_addr, e);
                            return;
                        }
                    },
                    None => None,
                };

                command_snd
                    .send(CommandMessage::ConnectPeer {
                        peer_addr,
                        local_addr,
                    })
                    .unwrap();

                *self = Self::Waiting {
                    peer_addr: peer_addr,
                    local_addr: PeerLocalState::Unbound(local_addr),
                    authorized: false,
                };
            }
            (
                Self::EditingLocal {
                    peer_addr,
                    local_addr,
                },
                Setup,
            ) => {
                let local_addr = match local_addr
                    .trim()
                    .is_empty()
                    .not()
                    .then(|| local_addr.parse::<SocketAddr>())
                {
                    Some(Ok(i)) => Some(i),
                    Some(Err(e)) => {
                        if let Ok(i) = local_addr.parse() {
                            Some(SocketAddr::new(DEFAULT_SOCKET_ADDR.ip(), i))
                        } else {
                            eprintln!("Invalid local address {}: {}", local_addr, e);
                            return;
                        }
                    },
                    None => None,
                };

                command_snd
                    .send(CommandMessage::ConnectPeer {
                        peer_addr: *peer_addr,
                        local_addr,
                    })
                    .unwrap();

                *self = Self::Waiting {
                    peer_addr: *peer_addr,
                    local_addr: PeerLocalState::Unbound(local_addr),
                    authorized: false,
                };
            }
            (_, Setup) => {
                unreachable!("invalid message: {:?} @ {:?}", Setup, self)
            }
            (Self::EditingPeer { .. }, UpdatePeer(value)) => {
                *self = Self::EditingPeer {
                    peer_addr: value,
                    local_addr: take_value!(Self::EditingPeer => self.local_addr),
                };
            }
            (_, UpdatePeer(value)) => {
                unreachable!("invalid message: {:?} @ {:?}", UpdatePeer(value), self)
            }
            (Self::EditingPeer { .. }, UpdateLocal(value)) => {
                *self = Self::EditingPeer {
                    peer_addr: take_value!(Self::EditingPeer => self.peer_addr),
                    local_addr: value,
                };
            }
            (Self::EditingLocal { peer_addr, .. }, UpdateLocal(value)) => {
                *self = Self::EditingLocal {
                    peer_addr: *peer_addr,
                    local_addr: value,
                };
            }
            (_, UpdateLocal(value)) => {
                unreachable!("invalid message: {:?} @ {:?}", UpdateLocal(value), self)
            }
            (
                Self::Waiting {
                    peer_addr,
                    local_addr: PeerLocalState::Bound(local_addr, pinned),
                    authorized: false,
                },
                OnPermissionGranted,
            ) => {
                *self = Self::Ready {
                    peer_addr: *peer_addr,
                    local_addr: PeerLocalState::Bound(*local_addr, *pinned),
                };
            }
            (
                Self::Waiting {
                    peer_addr,
                    local_addr: PeerLocalState::Unbound(pinned_addr),
                    authorized: false,
                },
                OnPermissionGranted,
            ) => {
                *self = Self::Waiting {
                    peer_addr: *peer_addr,
                    local_addr: PeerLocalState::Unbound(*pinned_addr),
                    authorized: true,
                };
            }
            (_, OnPermissionGranted) => {
                eprintln!("message ignored: {:?} @ {:?}", OnPermissionGranted, self);
            }

            (Self::Waiting { .. }, OnPermissionDenied) => {
                *self = Self::Failed {
                    peer_addr: take_value!(Self::Waiting => self.peer_addr: SocketAddr),
                    local_addr: take_value!(Self::Waiting => self.local_addr),
                    authorized: false,
                }
            }
            (Self::Ready { .. }, OnPermissionDenied) => {
                *self = Self::Failed {
                    peer_addr: take_value!(Self::Ready => self.peer_addr: SocketAddr),
                    local_addr: take_value!(Self::Ready => self.local_addr),
                    authorized: false,
                }
            }
            (_, OnPermissionDenied) => {
                eprintln!("message ignored: {:?} @ {:?}", OnPermissionDenied, self);
            }

            (
                Self::Waiting {
                    peer_addr,
                    local_addr: PeerLocalState::Unbound(pinned_addr),
                    authorized: true,
                },
                OnBound(local_addr),
            ) => {
                *self = Self::Ready {
                    peer_addr: *peer_addr,
                    local_addr: PeerLocalState::Unbound(*pinned_addr).with_assign(local_addr),
                }
            }
            (
                Self::Waiting {
                    peer_addr,
                    local_addr: PeerLocalState::Unbound(pinned_addr),
                    authorized: false,
                },
                OnBound(local_addr),
            ) => {
                *self = Self::Waiting {
                    peer_addr: *peer_addr,
                    local_addr: PeerLocalState::Unbound(*pinned_addr).with_assign(local_addr),
                    authorized: false,
                };
            }
            (_, OnBound(local_addr)) => {
                eprintln!("message ignored: {:?} @ {:?}", OnBound(local_addr), self);
            }

            (Self::Waiting { .. } | Self::Ready { .. }, OnBindFailed) => {
                *self = Self::Failed {
                    peer_addr: take_value!(Self::Waiting | Self::Ready => self.peer_addr: SocketAddr),
                    local_addr: take_value!(Self::Waiting | Self::Ready => self.local_addr),
                    authorized: true,
                };
            }
            (_, OnBindFailed) => {
                eprintln!("message ignored: {:?} @ {:?}", OnBindFailed, self);
            }

            (
                Self::Waiting {
                    peer_addr,
                    local_addr,
                    ..
                }
                | Self::Failed {
                    peer_addr,
                    local_addr,
                    ..
                }
                | Self::Ready {
                    peer_addr,
                    local_addr,
                },
                OnUnbound,
            ) => {
                *self = Self::EditingLocal {
                    peer_addr: *peer_addr,
                    local_addr: if let PeerLocalState::Bound(addr, true)
                    | PeerLocalState::Unbound(Some(addr)) = local_addr
                    {
                        format!("{}", addr)
                    } else {
                        String::default()
                    },
                }
            }
            (_, OnUnbound) => {
                eprintln!("message ignored: {:?} @ {:?}", OnBindFailed, self);
            }
        }
    }

    pub fn view(&self, index: usize) -> Row<PeerEntryMessage> {
        use PeerEntryMessage::*;

        match self {
            Self::EditingPeer {
                peer_addr,
                local_addr,
            } => row![
                text!("{: <1$} ", "", index.to_string().len()),
                text_input("123.45.67.89:12345", peer_addr).on_input(|value| UpdatePeer(value)),
                text_input("127.0.0.1:12345", local_addr).on_input(|value| UpdateLocal(value)),
                button(text!("+")).on_press(Setup),
            ],
            Self::EditingLocal {
                peer_addr,
                local_addr,
            } => row![
                text!("{})", index),
                text!("{}", peer_addr),
                text_input("127.0.0.1:12345", local_addr).on_input(|value| UpdateLocal(value)),
                button(text!("+")).on_press(Setup),
            ],
            Self::Waiting {
                peer_addr,
                local_addr,
                authorized,
            } => row![
                text!("{})", index),
                text!("{}", peer_addr),
                match (local_addr, authorized) {
                    (PeerLocalState::Bound(_, _), false) => text!("Authorizing..."),
                    (PeerLocalState::Unbound(_), true) => text!("Binding..."),
                    _ => text!("Waiting..."),
                },
                button(text!("X")).on_press(Delete),
            ],
            Self::Failed {
                peer_addr,
                local_addr: _,
                authorized,
            } => row![
                text!("{})", index),
                text!("{}", peer_addr),
                if *authorized {
                    text!("Binding Failed")
                } else {
                    text!("Authorization Failed")
                },
                button(text!("X")).on_press(Delete),
            ],
            Self::Ready {
                peer_addr,
                local_addr,
            } => row![
                text!("{})", index),
                text!("{}", peer_addr),
                text!("{}", local_addr),
                button(text!("X")).on_press(Delete),
            ],
        }
    }
}
