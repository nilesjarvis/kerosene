use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use iced::Task;

const CHART_EDITOR_RESULT_ROW_HEIGHT: f32 = 28.0;
const CHART_EDITOR_RESULT_SCROLL_PADDING: f32 = 104.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChartEditorSelectionStep {
    Previous,
    Next,
}

impl TradingTerminal {
    pub(super) fn handle_hotkey_keyboard_event(&mut self, message: Message) -> Task<Message> {
        let Message::KeyboardEvent(
            iced::keyboard::Event::KeyPressed { key, modifiers, .. },
            status,
        ) = message
        else {
            return Task::none();
        };

        if let Some(action) = self.recording_hotkey_for.clone() {
            return self.apply_recorded_hotkey(action, key, modifiers);
        }

        if let Some(editor_task) = self.handle_chart_editor_keyboard(key.as_ref(), modifiers) {
            return editor_task;
        }

        if status != iced::event::Status::Ignored || self.hotkeys.is_empty() {
            return Task::none();
        }

        let Some(key_str) = Self::hotkey_key_string(&key) else {
            return Task::none();
        };

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

    fn apply_recorded_hotkey(
        &mut self,
        action: config::HotkeyAction,
        key: iced::keyboard::Key,
        modifiers: iced::keyboard::Modifiers,
    ) -> Task<Message> {
        if key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) {
            self.recording_hotkey_for = None;
            return Task::none();
        }

        let Some(key_str) = Self::hotkey_key_string(&key) else {
            return Task::none();
        };

        if Self::hotkey_key_is_modifier(&key_str) {
            return Task::none();
        }

        if !Self::hotkey_combo_is_assignable(&key_str, modifiers) {
            self.push_toast(
                "Use a function key or a key combination with Ctrl, Alt, Shift, or Win/Cmd"
                    .to_string(),
                true,
            );
            return Task::none();
        }

        let conflicting_action = self
            .hotkeys
            .iter()
            .find(|hotkey| {
                hotkey.action != action && Self::hotkey_matches(hotkey, &key_str, modifiers)
            })
            .map(|hotkey| hotkey.action.clone());

        if let Some(conflicting_action) = conflicting_action {
            let label = self.hotkey_action_label(&conflicting_action);
            self.push_toast(format!("Hotkey already assigned to {label}"), true);
            return Task::none();
        }

        self.recording_hotkey_for = None;
        if let Some(existing) = self
            .hotkeys
            .iter_mut()
            .find(|hotkey| hotkey.action == action)
        {
            existing.key = key_str;
            existing.shift = modifiers.shift();
            existing.ctrl = modifiers.control();
            existing.alt = modifiers.alt();
            existing.logo = modifiers.logo();
        } else {
            self.hotkeys.push(config::HotkeyConfig {
                action,
                key: key_str,
                shift: modifiers.shift(),
                ctrl: modifiers.control(),
                alt: modifiers.alt(),
                logo: modifiers.logo(),
            });
        }
        self.persist_config();

        Task::none()
    }

    fn handle_chart_editor_keyboard(
        &mut self,
        key: iced::keyboard::Key<&str>,
        modifiers: iced::keyboard::Modifiers,
    ) -> Option<Task<Message>> {
        let editor_id = self.active_chart_editor_id()?;
        if modifiers.control() || modifiers.alt() || modifiers.logo() {
            return None;
        }

        match key {
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                Some(self.move_chart_editor_selection(editor_id, ChartEditorSelectionStep::Next))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) if !modifiers.shift() => {
                Some(self.move_chart_editor_selection(editor_id, ChartEditorSelectionStep::Next))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => Some(
                self.move_chart_editor_selection(editor_id, ChartEditorSelectionStep::Previous),
            ),
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) if modifiers.shift() => {
                Some(
                    self.move_chart_editor_selection(editor_id, ChartEditorSelectionStep::Previous),
                )
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter) => {
                if self
                    .charts
                    .get(&editor_id)
                    .is_some_and(|instance| instance.editor_selected_index.is_some())
                {
                    return Some(self.update(Message::ChartEditorSubmit(editor_id)));
                }
                None
            }
            _ => None,
        }
    }

    fn move_chart_editor_selection(
        &mut self,
        editor_id: crate::chart_state::ChartId,
        step: ChartEditorSelectionStep,
    ) -> Task<Message> {
        let Some((query, current_index)) = self.charts.get(&editor_id).map(|instance| {
            (
                instance.editor_search_query.trim().to_string(),
                instance.editor_selected_index,
            )
        }) else {
            return Task::none();
        };

        let result_count = self.chart_editor_filtered_symbols(&query).len();
        let next_index = next_chart_editor_selection(current_index, result_count, step);
        if let Some(instance) = self.charts.get_mut(&editor_id) {
            instance.editor_selected_index = next_index;
        }

        let Some(index) = next_index else {
            return Task::none();
        };

        let offset_y = (index as f32 * CHART_EDITOR_RESULT_ROW_HEIGHT
            - CHART_EDITOR_RESULT_SCROLL_PADDING)
            .max(0.0);

        Task::batch([
            Self::scroll_chart_symbol_search_results_to(editor_id, offset_y),
            Self::focus_chart_symbol_search_input(editor_id),
        ])
    }
}

fn next_chart_editor_selection(
    current_index: Option<usize>,
    result_count: usize,
    step: ChartEditorSelectionStep,
) -> Option<usize> {
    if result_count == 0 {
        return None;
    }

    match (current_index.filter(|index| *index < result_count), step) {
        (Some(index), ChartEditorSelectionStep::Next) => {
            Some(index.saturating_add(1).min(result_count - 1))
        }
        (Some(index), ChartEditorSelectionStep::Previous) => Some(index.saturating_sub(1)),
        (None, ChartEditorSelectionStep::Next) => Some(0),
        (None, ChartEditorSelectionStep::Previous) => Some(result_count - 1),
    }
}

#[cfg(test)]
mod tests {
    use super::{ChartEditorSelectionStep, next_chart_editor_selection};

    #[test]
    fn chart_editor_keyboard_selection_starts_in_direction() {
        assert_eq!(
            next_chart_editor_selection(None, 4, ChartEditorSelectionStep::Next),
            Some(0)
        );
        assert_eq!(
            next_chart_editor_selection(None, 4, ChartEditorSelectionStep::Previous),
            Some(3)
        );
    }

    #[test]
    fn chart_editor_keyboard_selection_clamps_at_edges() {
        assert_eq!(
            next_chart_editor_selection(Some(2), 3, ChartEditorSelectionStep::Next),
            Some(2)
        );
        assert_eq!(
            next_chart_editor_selection(Some(0), 3, ChartEditorSelectionStep::Previous),
            Some(0)
        );
    }

    #[test]
    fn chart_editor_keyboard_selection_handles_empty_or_stale_index() {
        assert_eq!(
            next_chart_editor_selection(Some(2), 0, ChartEditorSelectionStep::Next),
            None
        );
        assert_eq!(
            next_chart_editor_selection(Some(99), 3, ChartEditorSelectionStep::Next),
            Some(0)
        );
    }
}
