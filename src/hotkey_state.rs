use crate::app_state::TradingTerminal;

use crate::config;

impl TradingTerminal {
    pub(crate) fn hotkey_key_string(key: &iced::keyboard::Key) -> Option<String> {
        match key.as_ref() {
            iced::keyboard::Key::Character(c) => Some(c.to_uppercase()),
            iced::keyboard::Key::Named(n) => Some(format!("{:?}", n)),
            _ => None,
        }
    }

    pub(crate) fn hotkey_combo_is_assignable(
        key_str: &str,
        modifiers: iced::keyboard::Modifiers,
    ) -> bool {
        if [
            "Shift",
            "Control",
            "Alt",
            "Meta",
            "Super",
            "Logo",
            "Space",
            "Tab",
            "Enter",
            "Escape",
            "Backspace",
            "Delete",
            "ArrowLeft",
            "ArrowRight",
            "ArrowUp",
            "ArrowDown",
            "Home",
            "End",
            "PageUp",
            "PageDown",
        ]
        .contains(&key_str)
        {
            return false;
        }

        modifiers.shift()
            || modifiers.control()
            || modifiers.alt()
            || modifiers.logo()
            || key_str.len() > 1
    }

    pub(crate) fn hotkey_matches(
        hotkey: &config::HotkeyConfig,
        key_str: &str,
        modifiers: iced::keyboard::Modifiers,
    ) -> bool {
        hotkey.key == key_str
            && hotkey.shift == modifiers.shift()
            && hotkey.ctrl == modifiers.control()
            && hotkey.alt == modifiers.alt()
            && hotkey.logo == modifiers.logo()
            && Self::hotkey_combo_is_assignable(key_str, modifiers)
    }

    pub(crate) fn hotkey_display(hotkey: &config::HotkeyConfig) -> String {
        let mut parts = Vec::new();
        if hotkey.ctrl {
            parts.push("Ctrl".to_string());
        }
        if hotkey.alt {
            parts.push("Alt".to_string());
        }
        if hotkey.shift {
            parts.push("Shift".to_string());
        }
        if hotkey.logo {
            parts.push("Win/Cmd".to_string());
        }
        parts.push(hotkey.key.clone());
        parts.join(" + ")
    }

    pub(crate) fn hotkey_action_label(&self, action: &config::HotkeyAction) -> String {
        match action {
            config::HotkeyAction::AddCandlestickChart => "Add Candlestick Chart".to_string(),
            config::HotkeyAction::OpenWalletTracker => "Open Wallet Tracker".to_string(),
            config::HotkeyAction::OpenQuickSymbolSearch => "Quick Symbol Search".to_string(),
            config::HotkeyAction::OpenSettingsWindow => "Open Settings".to_string(),
            config::HotkeyAction::SwitchAccount { secret_id } => self
                .accounts
                .iter()
                .find(|profile| profile.secret_id == secret_id.as_str())
                .map(|profile| format!("Switch to {}", profile.name))
                .unwrap_or_else(|| "Switch to missing account".to_string()),
        }
    }

    pub(crate) fn hotkey_key_is_modifier(key_str: &str) -> bool {
        ["Shift", "Control", "Alt", "Meta", "Super", "Logo"].contains(&key_str)
    }

    pub(crate) fn available_hotkey_actions(&self) -> Vec<(config::HotkeyAction, String)> {
        let mut actions = vec![
            (
                config::HotkeyAction::AddCandlestickChart,
                "Add Candlestick Chart".to_string(),
            ),
            (
                config::HotkeyAction::OpenWalletTracker,
                "Open Wallet Tracker".to_string(),
            ),
            (
                config::HotkeyAction::OpenQuickSymbolSearch,
                "Quick Symbol Search".to_string(),
            ),
            (
                config::HotkeyAction::OpenSettingsWindow,
                "Open Settings".to_string(),
            ),
        ];

        actions.extend(
            self.accounts
                .iter()
                .filter(|profile| !self.ghost_account_secret_ids.contains(&profile.secret_id))
                .map(|profile| {
                    (
                        config::HotkeyAction::SwitchAccount {
                            secret_id: profile.secret_id.clone(),
                        },
                        format!("Switch to {}", profile.name),
                    )
                }),
        );

        actions
    }
}
