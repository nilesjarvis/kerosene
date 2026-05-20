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

    pub(super) fn clear_configured_hotkey(&mut self, message: Message) -> Task<Message> {
        let Message::ClearHotkey(action) = message else {
            return Task::none();
        };

        if self.recording_hotkey_for.as_ref() == Some(&action) {
            self.recording_hotkey_for = None;
        }

        if action == crate::config::HotkeyAction::ChartTimeframePrefix {
            if self.chart_timeframe_hotkey_prefix.take().is_some() {
                self.persist_config();
            }
            return Task::none();
        }

        let before = self.hotkeys.len();
        self.hotkeys.retain(|hotkey| hotkey.action != action);
        if self.hotkeys.len() != before {
            self.persist_config();
        }

        Task::none()
    }
}
