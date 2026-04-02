use std::net::SocketAddr;

use serde_derive::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum PeerState {
    Waiting {
        peer_addr: SocketAddr,
        local_addr: Option<SocketAddr>,
        authorized: bool,
    },
    Failed {
        peer_addr: SocketAddr,
        permission_denied: bool,
        bind_failed: bool,
    },
    Ready {
        peer_addr: SocketAddr,
        local_addr: SocketAddr,
    },
}

impl PeerState {
    pub fn grant(&mut self) {
        if let Self::Waiting {
            peer_addr,
            local_addr,
            authorized,
        } = self
        {
            if let Some(i) = local_addr {
                *self = Self::Ready {
                    peer_addr: *peer_addr,
                    local_addr: *i,
                };
            } else {
                *authorized = true;
            }
        }
    }

    pub fn deny(&mut self) {
        if let Self::Failed {
            peer_addr,
            bind_failed,
            ..
        } = self
        {
            *self = Self::Failed {
                peer_addr: *peer_addr,
                permission_denied: true,
                bind_failed: *bind_failed,
            };
        } else if let Self::Ready { peer_addr, .. } | Self::Waiting { peer_addr, .. } = self {
            *self = Self::Failed {
                peer_addr: *peer_addr,
                permission_denied: true,
                bind_failed: false,
            };
        }
    }

    pub fn bind(&mut self, local_addr: SocketAddr) {
        if let Self::Waiting {
            peer_addr,
            local_addr: l,
            authorized,
        } = self
        {
            if *authorized {
                *self = Self::Ready {
                    peer_addr: *peer_addr,
                    local_addr,
                };
            } else {
                *l = Some(local_addr);
            }
        }
    }

    pub fn bind_failed(&mut self) {
        if let Self::Failed {
            peer_addr,
            bind_failed,
            ..
        } = self
        {
            *self = Self::Failed {
                peer_addr: *peer_addr,
                permission_denied: true,
                bind_failed: *bind_failed,
            };
        } else if let Self::Ready { peer_addr, .. } | Self::Waiting { peer_addr, .. } = self {
            *self = Self::Failed {
                peer_addr: *peer_addr,
                permission_denied: true,
                bind_failed: false,
            };
        }
    }

    pub fn local_addr(&self) -> Option<SocketAddr> {
        if let Self::Ready { local_addr, .. } = self {
            Some(*local_addr)
        } else {
            None
        }
    }
}
