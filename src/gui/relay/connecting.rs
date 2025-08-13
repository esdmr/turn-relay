use iced::widget::{button, column, text, vertical_space};
use iced::{Element, Task};
use tokio::sync::broadcast;

use crate::gui::types::IcedComponent;
use crate::worker::CommandMessage;

#[derive(Debug, Clone)]
pub enum Message {
    Disconnect,
}

#[derive(Debug, Clone)]
pub struct State {
    pub server: String,
    force_disconnect: bool,
}

impl State {
    pub const fn new(server: String) -> Self {
        Self {
            server,
            force_disconnect: false,
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
            Message::Disconnect => {
                if self.force_disconnect {
                    return Task::done(super::Message::ToDisconnected);
                }

                command_snd.send(CommandMessage::DisconnectAll).unwrap();

                self.force_disconnect = true;
            }
        }

        Task::none()
    }

    fn view<'a>(&'a self, _extra: Self::ExtraViewArgs<'_>) -> Element<'a, Self::Message> {
        column![
            text!("Connecting..."),
            vertical_space().height(24),
            button(text!("Cancel")).on_press(Message::Disconnect),
        ]
        .into()
    }
}
