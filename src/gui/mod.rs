mod app;
mod peer;
mod relay;

use crate::gui::app::App;

use iced::{application, window, Settings, Size};

pub fn run_gui() -> iced::Result {
    application("TURN Relay", App::update, App::view)
        .subscription(App::subscribe)
        .settings(Settings {
            id: Some("turn_relay".to_string()),
            ..Default::default()
        })
        .window(window::Settings {
            exit_on_close_request: false,
            min_size: Some(Size::new(456., 456.)),
            ..Default::default()
        })
        .run()
}
