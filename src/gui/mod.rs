mod app;
mod macros;
mod peer;
mod relay;
mod types;

use iced::{application, window, Settings, Size};

use crate::gui::types::IcedBasicComponent;

pub fn run_gui() -> iced::Result {
    application(
        "TURN Relay",
        app::State::update_basic,
        app::State::view_basic,
    )
    .subscription(app::State::subscription_basic)
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
