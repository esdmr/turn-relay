use std::{net::SocketAddr, ops::IndexMut, usize};

use serde_derive::Serialize;

use crate::{control::peer::PeerState, DEFAULT_FWD_SOCKET};

#[derive(Debug, Clone, Serialize, Default)]
#[serde(tag = "type")]
pub enum RelayState {
    #[default]
    Disconnected,
    Connecting,
    ConnectionFailed {
        why: String,
    },
    Connected {
        relay_addr: SocketAddr,
        fwd_addr: SocketAddr,
        peers: Vec<PeerState>,
    },
}

impl RelayState {
    pub fn connect(&mut self) {
        *self = RelayState::Connecting;
    }

    pub fn allocate(&mut self, relay_addr: SocketAddr) {
        *self = RelayState::Connected {
            relay_addr,
            fwd_addr: DEFAULT_FWD_SOCKET,
            peers: Vec::new(),
        };
    }

    pub fn failure(&mut self, why: String) {
        *self = RelayState::ConnectionFailed { why };
    }

    pub fn disconnect(&mut self) {
        *self = RelayState::Disconnected;
    }

    pub fn relay_addr(&mut self) -> Option<SocketAddr> {
        if let RelayState::Connected { relay_addr, .. } = self {
            Some(*relay_addr)
        } else {
            None
        }
    }

    pub fn fwd_addr(&mut self) -> Option<SocketAddr> {
        if let RelayState::Connected { fwd_addr, .. } = self {
            Some(*fwd_addr)
        } else {
            None
        }
    }

    pub fn peers(&self) -> Option<&Vec<PeerState>> {
        if let RelayState::Connected { peers, .. } = self {
            Some(peers)
        } else {
            None
        }
    }

    pub fn peer(&self, peer_addr: SocketAddr) -> Option<&PeerState> {
        if let RelayState::Connected { peers, .. } = self {
            let mut index = usize::MAX;

            for (i, p) in peers.iter().enumerate() {
                let (PeerState::Failed { peer_addr: a, .. }
                | PeerState::Waiting { peer_addr: a, .. }
                | PeerState::Ready { peer_addr: a, .. }) = p;

                if peer_addr == *a {
                    index = i;
                    break;
                }
            }

            if index < usize::MAX {
                Some(&peers[index])
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn peer_mut(&mut self, peer_addr: SocketAddr) -> Option<&mut PeerState> {
        if let RelayState::Connected { peers, .. } = self {
            let mut index = usize::MAX;

            for (i, p) in peers.iter().enumerate() {
                let (PeerState::Failed { peer_addr: a, .. }
                | PeerState::Waiting { peer_addr: a, .. }
                | PeerState::Ready { peer_addr: a, .. }) = p;

                if peer_addr == *a {
                    index = i;
                    break;
                }
            }

            if index < usize::MAX {
                Some(&mut peers[index])
            } else {
                peers.push(PeerState::Waiting {
                    peer_addr,
                    local_addr: None,
                    authorized: false,
                });

                peers.last_mut()
            }
        } else {
            None
        }
    }

    pub fn unbind_peer(&mut self, peer_addr: SocketAddr) {
        if let RelayState::Connected { peers, .. } = self {
            let mut index = usize::MAX;

            for (i, p) in peers.iter().enumerate() {
                let (PeerState::Failed { peer_addr: a, .. }
                | PeerState::Waiting { peer_addr: a, .. }
                | PeerState::Ready { peer_addr: a, .. }) = p;

                if peer_addr == *a {
                    index = i;
                    break;
                }
            }

            if index < usize::MAX {
                peers.remove(index);
            }
        }
    }
}
