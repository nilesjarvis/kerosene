use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Subscription;

// ---------------------------------------------------------------------------
// Keyboard And Input Subscriptions
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_keyboard_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        if self.recording_hotkey_for.is_some()
            || !self.hotkeys.is_empty()
            || self.charts.values().any(|inst| inst.editor_open)
        {
            subs.push(iced::event::listen_with(|event, status, _window| {
                if let iced::Event::Keyboard(keyboard_event) = event {
                    Some(Message::KeyboardEvent(keyboard_event, status))
                } else {
                    None
                }
            }));
        }

        if self.charts.values().any(|inst| {
            inst.quick_order.is_some() || inst.editor_open || inst.chart.active_tool.is_some()
        }) {
            subs.push(iced::event::listen_with(|event, _status, _id| {
                if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                    ..
                }) = event
                {
                    Some(Message::EscapePressed)
                } else {
                    None
                }
            }));
        }
    }
}
