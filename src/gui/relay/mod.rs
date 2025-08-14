mod connected;
mod connecting;
mod connection_failed;
mod disconnected;

use std::net::SocketAddr;

use tokio::sync::broadcast;

use crate::{
    gui::{macros::router_component, peer},
    worker::CommandMessage,
};

router_component! {
    message enum Message {
        ForPeerByAddr(SocketAddr, peer::Message),
        ForPeerByIndex(usize, peer::Message),
        OnAllocated(SocketAddr),
        OnConnectionFailed(String),
        OnDisconnected,
        OnRedirect(SocketAddr),
        ToConnecting {
            server: String,
            username: String,
            password: String,
        },
        ToDisconnected,
    }

    state enum State {
        Disconnected(disconnected::State),
        Connecting(connecting::State),
        ConnectionFailed(connection_failed::State),
        Connected(connected::State),
    }

    impl Default for Disconnected;

    type ExtraUpdateArgs<'a> = &'a broadcast::Sender<CommandMessage>;
    type ExtraViewArgs<'a> = ();
    type ExtraSubscriptionArgs<'a> = ();

    fn update(..) {
        // ForPeerByAddr
        given ForPeerByAddr ignore Disconnected;
        given ForPeerByAddr ignore Connecting;
        given ForPeerByAddr ignore ConnectionFailed;

        given ForPeerByAddr(addr, message)
            pass Connected(connected::Message::ForPeerByAddr(addr, message));

        // ForPeerByIndex
        given ForPeerByIndex ignore Disconnected;
        given ForPeerByIndex ignore Connecting;
        given ForPeerByIndex ignore ConnectionFailed;

        given ForPeerByIndex(index, message)
            pass Connected(connected::Message::ForPeerByIndex(index, message));

        // OnAllocated
        given OnAllocated ignore Disconnected;

        given OnAllocated(relay_addr)
            turn Connecting(i)
            into Connected(connected::State::new(i.server, relay_addr));

        given OnAllocated ignore ConnectionFailed;
        given OnAllocated ignore Connected;

        // OnConnectionFailed
        given OnConnectionFailed ignore Disconnected;

        given OnConnectionFailed(why)
            turn Connecting(connecting::State {server, ..})
            | Connected(connected::State {server, ..})
            into ConnectionFailed(connection_failed::State::new(server, why));

        given OnConnectionFailed ignore ConnectionFailed;

        // OnDisconnected
        given OnDisconnected ignore Disconnected;
        given OnDisconnected turn Connecting into Disconnected;
        given OnDisconnected ignore ConnectionFailed;
        given OnDisconnected turn Connected into Disconnected;

        // OnRedirect
        given OnRedirect ignore Disconnected;

        given OnRedirect(server)
            turn Connecting(_)
            into Disconnected(disconnected::State::new(format!("{server}")));

        given OnRedirect ignore ConnectionFailed;
        given OnRedirect ignore Connected;

        // ToConnecting
        given ToConnecting { server, username, password}
            turn Disconnected(_)
            into Connecting(connecting::State::new(server.clone()))
            then (command_snd) {
                command_snd
                    .send(CommandMessage::ConnectRelay {
                        server,
                        username,
                        password,
                    })
                    .unwrap();
            };

        given ToConnecting ignore Connecting;
        given ToConnecting ignore ConnectionFailed;
        given ToConnecting ignore Connected;

        // ToDisconnected
        given ToDisconnected ignore Disconnected;
        given ToDisconnected turn Connecting into Disconnected;
        given ToDisconnected turn ConnectionFailed into Disconnected;
        given ToDisconnected turn Connected into Disconnected;
    }
}
