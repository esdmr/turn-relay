mod app;
mod peer;
mod relay;

use crate::gui::app::App;

use iced::application;

pub fn run_gui() -> iced::Result {
    application("TURN Relay", App::update, App::view)
        .subscription(App::subscribe)
        .run()
}
