mod execution;
mod keyboard;
mod recording;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_hotkey_preferences(&mut self, message: Message) -> Task<Message> {
        match message {
            message @ Message::StartRecordingHotkey(_) => self.start_recording_hotkey(message),
            message @ Message::ClearHotkey(_) => self.clear_configured_hotkey(message),
            message @ Message::KeyboardEvent(_, _, _) => self.handle_hotkey_keyboard_event(message),
            message @ Message::ExecuteHotkey(_) => self.execute_configured_hotkey(message),
            _ => Task::none(),
        }
    }
}
