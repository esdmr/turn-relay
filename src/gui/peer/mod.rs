mod editing_local;
mod editing_peer;
mod failed;
mod ready;
mod types;
mod waiting;

use std::{mem::replace, net::SocketAddr};

use iced::{Element, Task};
use tokio::sync::broadcast;

use crate::{gui::types::IcedComponent, worker::CommandMessage};

#[derive(Debug, Clone)]
pub enum Message {
    ForEditingPeer(editing_peer::Message),
    ForEditingLocal(editing_local::Message),
    ForWaiting(waiting::Message),
    ForFailed(failed::Message),
    ForReady(ready::Message),
    ToWaiting {
        peer_addr: SocketAddr,
        pinned_addr: Option<SocketAddr>,
    },
    ToReady,
    OnPermissionGranted,
    OnPermissionDenied,
    OnBound(SocketAddr),
    OnUnbound,
    OnBindFailed,
}

impl From<editing_peer::Message> for Message {
    fn from(value: editing_peer::Message) -> Self {
        Self::ForEditingPeer(value)
    }
}

impl From<editing_local::Message> for Message {
    fn from(value: editing_local::Message) -> Self {
        Self::ForEditingLocal(value)
    }
}

impl From<waiting::Message> for Message {
    fn from(value: waiting::Message) -> Self {
        Self::ForWaiting(value)
    }
}

impl From<failed::Message> for Message {
    fn from(value: failed::Message) -> Self {
        Self::ForFailed(value)
    }
}

impl From<ready::Message> for Message {
    fn from(value: ready::Message) -> Self {
        Self::ForReady(value)
    }
}

#[derive(Debug, Clone)]
pub enum State {
    EditingPeer(editing_peer::State),
    EditingLocal(editing_local::State),
    Waiting(waiting::State),
    Failed(failed::State),
    Ready(ready::State),
    Intermediate,
}

impl Default for State {
    fn default() -> Self {
        Self::EditingPeer(editing_peer::State::default())
    }
}

impl State {
    pub fn compare_peer(&self, other_peer_addr: SocketAddr) -> bool {
        match self {
            Self::EditingPeer(_) => false,
            Self::EditingLocal(i) => i.compare_peer(other_peer_addr),
            Self::Waiting(i) => i.compare_peer(other_peer_addr),
            Self::Failed(i) => i.compare_peer(other_peer_addr),
            Self::Ready(i) => i.compare_peer(other_peer_addr),
            Self::Intermediate => {
                unreachable!("Peer UI state is in an intermediate state");
            }
        }
    }

    pub const fn is_uncommitted(&self) -> bool {
        matches!(self, Self::EditingPeer(_))
    }
}

impl IcedComponent for State {
    type Message = Message;
    type TaskMessage = Message;
    type ExtraUpdateArgs<'a> = (&'a broadcast::Sender<CommandMessage>, SocketAddr);
    type ExtraViewArgs<'a> = usize;
    type ExtraSubscriptionArgs<'a> = ();

    #[allow(clippy::too_many_lines)]
    fn update(
        &mut self,
        message: Self::Message,
        (command_snd, relay_addr): Self::ExtraUpdateArgs<'_>,
    ) -> Task<Self::TaskMessage> {
        match (replace(self, Self::Intermediate), message) {
            // State transitions
            (
                Self::EditingPeer(_) | Self::EditingLocal(_),
                Message::ToWaiting {
                    peer_addr,
                    pinned_addr,
                },
            ) => {
                *self = Self::Waiting(waiting::State::new(peer_addr, pinned_addr));
            }

            (
                Self::Waiting(waiting::State {
                    peer_addr,
                    local_addr,
                    ..
                }),
                Message::ToReady,
            ) => {
                *self = Self::Ready(ready::State::new(
                    peer_addr,
                    local_addr.bound_addr().unwrap(),
                    local_addr.is_pinned(),
                ));
            }

            (
                Self::Waiting(waiting::State {
                    peer_addr,
                    local_addr,
                    ..
                }),
                Message::OnPermissionDenied,
            ) => {
                *self = Self::Failed(failed::State::new_permission_denied(
                    peer_addr,
                    local_addr.pinned_addr(),
                ));
            }

            (
                Self::Ready(ready::State {
                    peer_addr,
                    local_addr,
                    pinned,
                }),
                Message::OnPermissionDenied,
            ) => {
                *self = Self::Failed(failed::State::new_permission_denied(
                    peer_addr,
                    pinned.then_some(local_addr),
                ));
            }

            (
                Self::Waiting(waiting::State {
                    peer_addr,
                    local_addr,
                    ..
                }),
                Message::OnBindFailed,
            ) => {
                *self = Self::Failed(failed::State::new_bind_failed(
                    peer_addr,
                    local_addr.pinned_addr(),
                ));
            }

            (
                Self::Ready(ready::State {
                    peer_addr,
                    local_addr,
                    pinned,
                }),
                Message::OnBindFailed,
            ) => {
                *self = Self::Failed(failed::State::new_bind_failed(
                    peer_addr,
                    pinned.then_some(local_addr),
                ));
            }

            (
                Self::Waiting(waiting::State {
                    peer_addr,
                    local_addr,
                    ..
                }),
                Message::OnUnbound,
            ) => {
                *self = Self::EditingLocal(editing_local::State::new(
                    peer_addr,
                    local_addr
                        .pinned_addr()
                        .map_or_else(String::new, |i| format!("{i}")),
                ));
            }

            (
                Self::Failed(failed::State {
                    peer_addr,
                    pinned_addr,
                    ..
                }),
                Message::OnUnbound,
            ) => {
                *self = Self::EditingLocal(editing_local::State::new(
                    peer_addr,
                    pinned_addr.map_or_else(String::new, |i| format!("{i}")),
                ));
            }

            (
                Self::Ready(ready::State {
                    peer_addr,
                    local_addr,
                    pinned,
                }),
                Message::OnUnbound,
            ) => {
                *self = Self::EditingLocal(editing_local::State::new(
                    peer_addr,
                    if pinned {
                        format!("{local_addr}")
                    } else {
                        String::new()
                    },
                ));
            }

            // External events
            (Self::Waiting(mut i), Message::OnPermissionGranted) => {
                let task = i.update(
                    waiting::Message::OnPermissionGranted,
                    (command_snd, relay_addr),
                );
                *self = Self::Waiting(i);
                return task;
            }

            (Self::Waiting(mut i), Message::OnBound(j)) => {
                let task = i.update(waiting::Message::OnBound(j), (command_snd, relay_addr));
                *self = Self::Waiting(i);
                return task;
            }

            (Self::Failed(mut i), Message::OnPermissionDenied) => {
                let task = i.update(
                    failed::Message::OnPermissionDenied,
                    (command_snd, relay_addr),
                );
                *self = Self::Failed(i);
                return task;
            }

            (Self::Failed(mut i), Message::OnBindFailed) => {
                let task = i.update(failed::Message::OnBindFailed, (command_snd, relay_addr));
                *self = Self::Failed(i);
                return task;
            }

            // Inner-state messages
            (Self::EditingPeer(mut i), Message::ForEditingPeer(message)) => {
                let task = i.update(message, (command_snd, relay_addr));
                *self = Self::EditingPeer(i);
                return task;
            }

            (Self::EditingLocal(mut i), Message::ForEditingLocal(message)) => {
                let task = i.update(message, (command_snd, relay_addr));
                *self = Self::EditingLocal(i);
                return task;
            }

            (Self::Waiting(mut i), Message::ForWaiting(message)) => {
                let task = i.update(message, (command_snd, relay_addr));
                *self = Self::Waiting(i);
                return task;
            }

            (Self::Failed(mut i), Message::ForFailed(message)) => {
                let task = i.update(message, (command_snd, relay_addr));
                *self = Self::Failed(i);
                return task;
            }

            (Self::Ready(mut i), Message::ForReady(message)) => {
                let task = i.update(message, (command_snd, relay_addr));
                *self = Self::Ready(i);
                return task;
            }

            // Invalid states or messages
            (Self::Intermediate, _) => {
                unreachable!("Peer UI state is in an intermediate state");
            }

            (state, message) => {
                eprintln!("Ignoring message: {message:?} @ {state:?}");
                *self = state;
            }
        }

        Task::none()
    }

    fn view<'a>(&'a self, index: Self::ExtraViewArgs<'_>) -> Element<'a, Self::Message> {
        match self {
            Self::EditingPeer(i) => i.view(index).map(Message::ForEditingPeer),
            Self::EditingLocal(i) => i.view(index).map(Message::ForEditingLocal),
            Self::Waiting(i) => i.view(index).map(Message::ForWaiting),
            Self::Failed(i) => i.view(index).map(Message::ForFailed),
            Self::Ready(i) => i.view(index).map(Message::ForReady),
            Self::Intermediate => {
                unreachable!("Peer UI state is in an intermediate state");
            }
        }
    }
}
