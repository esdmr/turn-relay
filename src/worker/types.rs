use std::{fmt::Debug, net::SocketAddr};

#[derive(Debug, Clone)]
pub enum ServiceMessage {
    RelayAllocated(SocketAddr),
    RelayDisconnected,
    RelayConnectionFailed(String),
    RelayRedirected(SocketAddr),
    RelayPeerGranted(SocketAddr),
    RelayPeerDenied(SocketAddr),
    PeerBound {
        peer_addr: SocketAddr,
        local_addr: SocketAddr,
    },
    PeerBindFailed(SocketAddr),
    PeerUnbound(SocketAddr),
}

#[derive(Debug, Clone)]
pub enum CommandMessage {
    ConnectRelay {
        server: String,
        username: String,
        password: String,
    },
    ConnectPeer {
        peer_addr: SocketAddr,
        local_addr: Option<SocketAddr>,
    },
    ChangeFwdAddr(SocketAddr),
    DisconnectAll,
    DisconnectPeer(SocketAddr),
    TerminateAll,
}

pub type DataMessage = (SocketAddr, Vec<u8>);
