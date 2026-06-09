use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use iced::Task;

mod chart_editor;
mod recording;

#[cfg(test)]
use chart_editor::{ChartEditorSelectionStep, next_chart_editor_selection};

impl TradingTerminal {
    pub(super) fn handle_hotkey_keyboard_event(&mut self, message: Message) -> Task<Message> {
        let Message::KeyboardEvent(window_id, event, status) = message else {
            return Task::none();
        };
        let is_main_window = self
            .main_window_id
            .is_none_or(|main_id| main_id == window_id);

        if let iced::keyboard::Event::ModifiersChanged(modifiers) = event {
            if self.recording_hotkey_for == Some(config::HotkeyAction::ChartTimeframePrefix) {
                return self.apply_recorded_chart_timeframe_prefix_from_modifiers(modifiers);
            }
            return Task::none();
        }

        let iced::keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
            return Task::none();
        };

        if let Some(action) = self.recording_hotkey_for.clone() {
            return self.apply_recorded_hotkey(action, key, modifiers);
        }

        if !is_main_window {
            if self.detached_window_has_open_chart_editor(window_id)
                && let Some(editor_task) =
                    self.handle_chart_editor_keyboard(key.as_ref(), modifiers)
            {
                return editor_task;
            }
            return Task::none();
        }

        if self.alfred.open {
            if let Some(key_str) = Self::hotkey_key_string(&key)
                && self.hotkeys.iter().any(|hotkey| {
                    hotkey.action == config::HotkeyAction::OpenAlfred
                        && Self::hotkey_matches(hotkey, &key_str, modifiers)
                })
            {
                return self.update(Message::ToggleAlfred);
            }
            return self.handle_alfred_keyboard(key.as_ref(), modifiers, status);
        }

        if let Some(editor_task) = self.handle_chart_editor_keyboard(key.as_ref(), modifiers) {
            return editor_task;
        }

        if status != iced::event::Status::Ignored {
            return Task::none();
        }

        let Some(key_str) = Self::hotkey_key_string(&key) else {
            return Task::none();
        };

        if let Some(timeframe_task) = self.handle_chart_timeframe_hotkey(&key_str, modifiers) {
            return timeframe_task;
        }

        if self.hotkeys.is_empty() {
            return Task::none();
        }

        let mut matched_action = None;
        for hotkey in &self.hotkeys {
            if Self::hotkey_matches(hotkey, &key_str, modifiers) {
                matched_action = Some(hotkey.action.clone());
                break;
            }
        }

        if let Some(action) = matched_action {
            return self.update(Message::ExecuteHotkey(action));
        }

        Task::none()
    }

    fn detached_window_has_open_chart_editor(&self, window_id: iced::window::Id) -> bool {
        self.detached_chart_windows
            .get(&window_id)
            .and_then(|state| self.charts.get(&state.chart_id))
            .is_some_and(|instance| instance.editor_open)
    }
}

#[cfg(test)]
mod tests;
