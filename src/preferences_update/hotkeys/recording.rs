use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn start_recording_hotkey(&mut self, message: Message) -> Task<Message> {
        if let Message::StartRecordingHotkey(action) = message {
            self.recording_hotkey_for = Some(action);
        }

        Task::none()
    }
}
