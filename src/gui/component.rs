use std::fmt::Debug;

use iced::{Element, Subscription, Task};

pub trait IcedComponent {
    type Message: Send + Debug + 'static;
    type TaskMessage: Send + Debug + 'static;
    type ExtraUpdateArgs<'a>;
    type ExtraViewArgs<'a>;
    type ExtraSubscriptionArgs<'a>;

    fn update(
        &mut self,
        message: Self::Message,
        extra: Self::ExtraUpdateArgs<'_>,
    ) -> Task<Self::TaskMessage>;

    fn view<'a>(&'a self, extra: Self::ExtraViewArgs<'_>) -> Element<'a, Self::Message>;

    fn subscription(&self, _extra: Self::ExtraSubscriptionArgs<'_>) -> Subscription<Self::Message> {
        Subscription::none()
    }
}
