use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::{Subscription, window};

mod hydromancer;
mod market;
mod telegram;
mod timers;
mod user_data;

impl TradingTerminal {
    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let mut subs = Vec::new();
        self.push_market_subscriptions(&mut subs);
        self.push_user_data_subscriptions(&mut subs);
        self.push_hydromancer_subscriptions(&mut subs);
        self.push_telegram_subscriptions(&mut subs);
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
            window::Event::Focused => Message::WindowFocused(id),
            window::Event::FileHovered(_) => Message::AgentPnlCardHoverChanged(id, true),
            window::Event::FileDropped(path) => {
                Message::AgentPnlCardDropped(id, crate::agent_pnl_card::AgentPnlCardPath::new(path))
            }
            window::Event::FilesHoveredLeft => Message::AgentPnlCardHoverChanged(id, false),
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
    fn focused_window_events_track_the_hotkey_workspace() {
        let id = window::Id::unique();

        assert!(matches!(
            TradingTerminal::window_event_message(id, window::Event::Focused),
            Message::WindowFocused(message_id) if message_id == id
        ));
    }

    #[test]
    fn dropped_files_are_forwarded_to_the_assistant_attachment_route() {
        let id = window::Id::unique();

        assert!(matches!(
            TradingTerminal::window_event_message(
                id,
                window::Event::FileDropped(std::path::PathBuf::from("card.png")),
            ),
            Message::AgentPnlCardDropped(message_id, _) if message_id == id
        ));
        assert!(matches!(
            TradingTerminal::window_event_message(id, window::Event::FilesHoveredLeft),
            Message::AgentPnlCardHoverChanged(message_id, false) if message_id == id
        ));
    }
}
