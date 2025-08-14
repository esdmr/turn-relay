mod editing_local;
mod editing_peer;
mod failed;
mod ready;
mod types;
mod waiting;

use std::net::SocketAddr;

use tokio::sync::broadcast;

use crate::{gui::macros::router_component, worker::CommandMessage};

router_component! {
    message enum Message {
        OnBindFailed,
        OnBound(SocketAddr),
        OnPermissionDenied,
        OnPermissionGranted,
        OnUnbound,
        ToEditingLocal,
        ToReady,
        ToWaiting {
            peer_addr: SocketAddr,
            pinned_addr: Option<SocketAddr>,
        },
    }

    state enum State {
        EditingPeer(editing_peer::State),
        EditingLocal(editing_local::State),
        Waiting(waiting::State),
        Failed(failed::State),
        Ready(ready::State),
    }

    impl Default for EditingPeer;

    type ExtraUpdateArgs<'a> = (&'a broadcast::Sender<CommandMessage>, SocketAddr);
    type ExtraViewArgs<'a> = usize;
    type ExtraSubscriptionArgs<'a> = ();

    fn update(..) {
        // OnBindFailed
        given OnBindFailed ignore EditingPeer;
        given OnBindFailed ignore EditingLocal;

        given OnBindFailed {}
            turn Waiting(i)
            into Failed(failed::State::new_bind_failed(i.peer_addr, i.local_addr.pinned_addr()));

        given OnBindFailed {}
            pass Failed(failed::Message::OnBindFailed);

        given OnBindFailed {}
            turn Ready(i)
            into Failed(failed::State::new_bind_failed(i.peer_addr, i.pinned.then_some(i.local_addr)));

        // OnBound
        given OnBound ignore EditingPeer;
        given OnBound ignore EditingLocal;

        given OnBound(i)
            pass Waiting(waiting::Message::OnBound(i));

        given OnBound ignore Failed;

        given OnBound(i)
            pass Ready(ready::Message::OnBound(i));

        // OnPermissionDenied
        given OnPermissionDenied ignore EditingPeer;
        given OnPermissionDenied ignore EditingLocal;

        given OnPermissionDenied {}
            turn Waiting(i)
            into Failed(failed::State::new_permission_denied(i.peer_addr, i.local_addr.pinned_addr()));

        given OnPermissionDenied {}
            pass Failed(failed::Message::OnPermissionDenied);

        given OnPermissionDenied {}
            turn Ready(i)
            into Failed(failed::State::new_bind_failed(i.peer_addr, i.pinned.then_some(i.local_addr)));

        // OnPermissionGranted
        given OnPermissionGranted ignore EditingPeer;
        given OnPermissionGranted ignore EditingLocal;

        given OnPermissionGranted {}
            pass Waiting(waiting::Message::OnPermissionGranted);

        given OnPermissionGranted ignore Failed;
        given OnPermissionGranted ignore Ready;

        // OnUnbound
        given OnUnbound ignore EditingPeer;
        given OnUnbound ignore EditingLocal;
        given OnUnbound turn Waiting into EditingLocal;
        given OnUnbound turn Failed into EditingLocal;
        given OnUnbound turn Ready into EditingLocal;

        // ToEditingLocal
        given ToEditingLocal ignore EditingPeer;
        given ToEditingLocal ignore EditingLocal;
        given ToEditingLocal turn Waiting into EditingLocal;
        given ToEditingLocal turn Failed into EditingLocal;
        given ToEditingLocal turn Ready into EditingLocal;

        // ToReady
        given ToReady ignore EditingPeer;
        given ToReady ignore EditingLocal;
        given ToReady turn Waiting into Ready;
        given ToReady ignore Failed;
        given ToReady ignore Ready;

        // ToWaiting
        given ToWaiting { peer_addr, pinned_addr, }
            turn EditingPeer(_) | EditingLocal(_)
            into Waiting(waiting::State::new(peer_addr, pinned_addr))
            then((command_snd, _)) {
                command_snd
                    .send(CommandMessage::ConnectPeer {
                        peer_addr,
                        local_addr: pinned_addr,
                    })
                    .unwrap();
            };

        given ToWaiting ignore Waiting;
        given ToWaiting ignore Failed;
        given ToWaiting ignore Ready;
    }
}

impl State {
    pub fn compare_peer(&self, other_peer_addr: SocketAddr) -> bool {
        match self {
            Self::Intermediate => {
                unreachable!("Fatal: UI state is in an intermediate state");
            }
            Self::EditingPeer(_) => false,
            Self::EditingLocal(editing_local::State { peer_addr, .. })
            | Self::Waiting(waiting::State { peer_addr, .. })
            | Self::Failed(failed::State { peer_addr, .. })
            | Self::Ready(ready::State { peer_addr, .. }) => *peer_addr == other_peer_addr,
        }
    }

    pub const fn is_uncommitted(&self) -> bool {
        matches!(self, Self::EditingPeer(..))
    }
}
