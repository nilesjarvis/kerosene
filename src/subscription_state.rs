use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::{Subscription, window};

mod hydromancer;
mod market;
mod timers;
mod user_data;

impl TradingTerminal {
    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let mut subs = Vec::new();
        self.push_market_subscriptions(&mut subs);
        self.push_user_data_subscriptions(&mut subs);
        self.push_hydromancer_subscriptions(&mut subs);
        self.push_timer_subscriptions(&mut subs);
        Self::push_window_subscriptions(&mut subs);
        self.push_post_window_timer_subscriptions(&mut subs);
        Subscription::batch(subs)
    }

    fn push_window_subscriptions(subs: &mut Vec<Subscription<Message>>) {
        subs.push(window::close_events().map(Message::WindowClosed));
        subs.push(window::events().map(|(id, event)| match event {
            iced::window::Event::Resized(size) => Message::WindowResized(id, size),
            iced::window::Event::Moved(point) => Message::WindowMoved(id, point),
            _ => Message::Tick,
        }));
    }
}
