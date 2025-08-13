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
    why: String,
}

impl State {
    pub const fn new(server: String, why: String) -> Self {
        Self { server, why }
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
            Message::Disconnect => {}
        }

        Task::none()
    }

    fn view<'a>(&'a self, _extra: Self::ExtraViewArgs<'_>) -> Element<'a, Self::Message> {
        column![
            text!("Connection to {} failed.", self.server),
            vertical_space().height(8),
            text!("{}", self.why),
            vertical_space().height(24),
            button(text!("Back")).on_press(Message::Disconnect),
        ]
        .into()
    }
}
