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

pub trait IcedBasicComponent: IcedComponent {
    fn update_basic(&mut self, message: Self::Message) -> Task<Self::TaskMessage>;
    fn view_basic(&self) -> Element<Self::Message>;
    fn subscription_basic(&self) -> Subscription<Self::Message>;
}

impl<'a, T> IcedBasicComponent for T
where
    T: IcedComponent<
        ExtraUpdateArgs<'a> = (),
        ExtraViewArgs<'a> = (),
        ExtraSubscriptionArgs<'a> = (),
        TaskMessage = Self::Message,
    >,
{
    #[inline]
    fn update_basic(&mut self, message: Self::Message) -> Task<Self::TaskMessage> {
        self.update(message, ())
    }

    #[inline]
    fn view_basic(&self) -> Element<Self::Message> {
        self.view(())
    }

    #[inline]
    fn subscription_basic(&self) -> Subscription<Self::Message> {
        self.subscription(())
    }
}
