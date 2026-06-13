use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::{Subscription, window};

mod hydromancer;
mod market;
mod telegram;
mod timers;
mod user_data;
mod x;

impl TradingTerminal {
    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let mut subs = Vec::new();
        self.push_market_subscriptions(&mut subs);
        self.push_user_data_subscriptions(&mut subs);
        self.push_hydromancer_subscriptions(&mut subs);
        self.push_telegram_subscriptions(&mut subs);
        self.push_x_feed_subscriptions(&mut subs);
        self.push_timer_subscriptions(&mut subs);
        Self::push_window_subscriptions(&mut subs);
        self.push_post_window_timer_subscriptions(&mut subs);
        Subscription::batch(subs)
    }

    fn push_window_subscriptions(subs: &mut Vec<Subscription<Message>>) {
        subs.push(window::close_events().map(Message::WindowClosed));
        subs.push(window::events().map(|(id, event)| Self::window_event_message(id, event)));
    }

    fn window_event_message(id: window::Id, event: window::Event) -> Message {
        match event {
            window::Event::Resized(size) => Message::WindowResized(id, size),
            window::Event::Moved(point) => Message::WindowMoved(id, point),
            _ => Message::NoOp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_resize_and_move_events_map_to_window_messages() {
        let id = window::Id::unique();
        let size = iced::Size::new(800.0, 600.0);
        let point = iced::Point::new(12.0, 34.0);

        assert!(matches!(
            TradingTerminal::window_event_message(id, window::Event::Resized(size)),
            Message::WindowResized(message_id, message_size)
                if message_id == id && message_size == size
        ));
        assert!(matches!(
            TradingTerminal::window_event_message(id, window::Event::Moved(point)),
            Message::WindowMoved(message_id, message_point)
                if message_id == id && message_point == point
        ));
    }

    #[test]
    fn ignored_window_events_do_not_emit_calendar_tick() {
        let id = window::Id::unique();

        assert!(matches!(
            TradingTerminal::window_event_message(id, window::Event::Focused),
            Message::NoOp
        ));
    }
}
