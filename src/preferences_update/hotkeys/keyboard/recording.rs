use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;

use iced::Task;

// ---------------------------------------------------------------------------
// Hotkey Recording
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn apply_recorded_hotkey(
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

        if action == config::HotkeyAction::ChartTimeframePrefix {
            return self.apply_recorded_chart_timeframe_prefix(&key_str, modifiers);
        }

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

        if self
            .chart_timeframe_hotkey_prefix
            .as_ref()
            .is_some_and(|prefix| {
                Self::hotkey_prefix_matches(prefix, modifiers)
                    && Self::chart_timeframe_for_hotkey_key(&key_str).is_some()
            })
        {
            self.push_toast(
                "Hotkey already reserved for Chart Timeframes".to_string(),
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

    pub(super) fn apply_recorded_chart_timeframe_prefix_from_modifiers(
        &mut self,
        modifiers: iced::keyboard::Modifiers,
    ) -> Task<Message> {
        self.apply_chart_timeframe_prefix(Self::hotkey_prefix_from_modifiers(modifiers))
    }

    fn apply_recorded_chart_timeframe_prefix(
        &mut self,
        key_str: &str,
        modifiers: iced::keyboard::Modifiers,
    ) -> Task<Message> {
        let Some(prefix) = Self::hotkey_prefix_from_recorded_key(key_str, modifiers) else {
            self.push_toast(
                "Hold Ctrl, Alt, Shift, or Win/Cmd to set the timeframe shortcut prefix"
                    .to_string(),
                true,
            );
            return Task::none();
        };

        self.apply_chart_timeframe_prefix(prefix)
    }

    fn apply_chart_timeframe_prefix(
        &mut self,
        prefix: config::HotkeyPrefixConfig,
    ) -> Task<Message> {
        let Some(prefix) = Self::normalize_chart_timeframe_hotkey_prefix(prefix) else {
            return Task::none();
        };

        if let Some(conflicting_action) = self
            .hotkeys
            .iter()
            .find(|hotkey| {
                Self::hotkey_has_prefix(hotkey, &prefix)
                    && Self::chart_timeframe_for_hotkey_key(&hotkey.key).is_some()
            })
            .map(|hotkey| hotkey.action.clone())
        {
            let label = self.hotkey_action_label(&conflicting_action);
            self.push_toast(format!("Timeframe prefix conflicts with {label}"), true);
            return Task::none();
        }

        self.recording_hotkey_for = None;
        self.chart_timeframe_hotkey_prefix = Some(prefix);
        self.persist_config();

        Task::none()
    }

    fn hotkey_prefix_from_recorded_key(
        key_str: &str,
        modifiers: iced::keyboard::Modifiers,
    ) -> Option<config::HotkeyPrefixConfig> {
        let mut prefix = Self::hotkey_prefix_from_modifiers(modifiers);

        match key_str {
            "Shift" => prefix.shift = true,
            "Control" => prefix.ctrl = true,
            "Alt" => prefix.alt = true,
            "Meta" | "Super" | "Logo" => prefix.logo = true,
            _ => {}
        }

        Self::normalize_chart_timeframe_hotkey_prefix(prefix)
    }

    pub(super) fn handle_chart_timeframe_hotkey(
        &mut self,
        key_str: &str,
        modifiers: iced::keyboard::Modifiers,
    ) -> Option<Task<Message>> {
        let prefix = self.chart_timeframe_hotkey_prefix.as_ref()?;
        if !Self::hotkey_prefix_matches(prefix, modifiers) {
            return None;
        }

        let timeframe = Self::chart_timeframe_for_hotkey_key(key_str)?;
        let Some(chart_id) = self.active_candlestick_chart_id() else {
            self.push_toast(
                "No candlestick chart available for timeframe hotkey".to_string(),
                true,
            );
            return Some(Task::none());
        };

        self.primary_chart_id = Some(chart_id);
        Some(self.update(Message::ChartSwitchTimeframe(chart_id, timeframe)))
    }
}
