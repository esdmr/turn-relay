use std::mem::take;

use iced::{
    widget::{button, column, horizontal_space, row, text, text_input, vertical_space},
    Element, Task,
};
use tokio::sync::broadcast;

use crate::{
    gui::{
        relay::{connected, connecting, connection_failed},
        types::IcedComponent,
    },
    worker::CommandMessage,
};

#[derive(Debug, Clone)]
pub enum Message {
    UpdateServer(String),
    UpdateUsername(String),
    UpdatePassword(String),
    Connect,
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub server: String,
    username: String,
    password: String,
}

impl State {
    pub fn new(server: String) -> Self {
        Self {
            server,
            ..Default::default()
        }
    }
}

impl From<connecting::State> for State {
    fn from(value: connecting::State) -> Self {
        Self::new(value.server)
    }
}

impl From<connection_failed::State> for State {
    fn from(value: connection_failed::State) -> Self {
        Self::new(value.server)
    }
}

impl From<connected::State> for State {
    fn from(value: connected::State) -> Self {
        Self::new(value.server)
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
        _command_snd: Self::ExtraUpdateArgs<'_>,
    ) -> Task<Self::TaskMessage> {
        match message {
            Message::UpdateServer(i) => {
                self.server = i;
            }

            Message::UpdateUsername(i) => {
                self.username = i;
            }

            Message::UpdatePassword(i) => {
                self.password = i;
            }

            Message::Connect => {
                return Task::done(super::Message::ToConnecting {
                    server: self.server.trim().to_string(),
                    username: take(&mut self.username),
                    password: take(&mut self.password),
                });
            }
        }

        Task::none()
    }

    fn view<'a>(&'a self, _extra: Self::ExtraViewArgs<'_>) -> Element<'a, Self::Message> {
        column![
            row![
                text!("Server").width(96),
                horizontal_space().width(8),
                text_input("example.com:12345", &self.server).on_input(Message::UpdateServer),
            ],
            vertical_space().height(8),
            row![
                text!("Username").width(96),
                horizontal_space().width(8),
                text_input("12345:user", &self.username).on_input(Message::UpdateUsername),
            ],
            vertical_space().height(8),
            row![
                text!("Password").width(96),
                horizontal_space().width(8),
                text_input("abc123", &self.password)
                    .on_input(Message::UpdatePassword)
                    .on_submit(Message::Connect),
            ],
            vertical_space().height(24),
            button(text!("Connect")).on_press(Message::Connect),
        ]
        .into()
    }
}
