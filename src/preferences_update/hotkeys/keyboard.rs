use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use iced::Task;

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
        match key {
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) if !modifiers.shift() => {
                let should_arm = self.charts.get(&editor_id).is_some_and(|instance| {
                    let query = instance.editor_search_query.trim();
                    !query.is_empty() && !self.chart_editor_filtered_symbols(query).is_empty()
                });
                if should_arm && let Some(instance) = self.charts.get_mut(&editor_id) {
                    instance.editor_keyboard_selected = true;
                }
                Some(Task::none())
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter) => {
                if self
                    .charts
                    .get(&editor_id)
                    .is_some_and(|instance| instance.editor_keyboard_selected)
                {
                    return Some(self.update(Message::ChartEditorSubmit(editor_id)));
                }
                None
            }
            _ => None,
        }
    }
}
