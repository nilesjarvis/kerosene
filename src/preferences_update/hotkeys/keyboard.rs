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
        let workspace = self.workspace_for_window(window_id).or_else(|| {
            self.main_window_id
                .is_none()
                .then_some(crate::canvas_state::WorkspaceId::Main)
        });
        if let Some(workspace) = workspace {
            self.last_focused_workspace = workspace;
            self.add_widget_workspace = workspace;
        }
        let is_workspace_window = workspace.is_some();

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

        if !is_workspace_window {
            if let Some(chart_id) = self.detached_window_open_chart_editor_id(window_id) {
                if let Some(editor_task) =
                    self.handle_chart_editor_keyboard_for_chart(chart_id, key.as_ref(), modifiers)
                {
                    return editor_task;
                }
                return Task::none();
            }
            if self.detached_window_has_open_spaghetti_editor(window_id) {
                // Spaghetti editor handles keyboard at the widget level
                return Task::none();
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

    fn detached_window_open_chart_editor_id(
        &self,
        window_id: iced::window::Id,
    ) -> Option<crate::chart_state::ChartId> {
        self.detached_chart_windows
            .get(&window_id)
            .map(|state| state.chart_id)
            .filter(|chart_id| {
                self.charts
                    .get(chart_id)
                    .is_some_and(|instance| instance.editor_open || instance.secondary_editor_open)
            })
    }

    fn detached_window_has_open_spaghetti_editor(&self, window_id: iced::window::Id) -> bool {
        self.detached_spaghetti_windows
            .get(&window_id)
            .and_then(|state| self.spaghetti_charts.get(&state.chart_id))
            .is_some_and(|instance| instance.editor_open)
    }
}

#[cfg(test)]
mod tests;
