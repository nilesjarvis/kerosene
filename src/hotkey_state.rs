use crate::app_state::TradingTerminal;

use crate::config;
use crate::timeframe::{TIMEFRAME_OPTIONS, Timeframe};

pub(crate) struct HotkeyActionGroup {
    pub(crate) title: &'static str,
    pub(crate) actions: Vec<(config::HotkeyAction, String)>,
}

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

    pub(crate) fn hotkey_prefix_has_modifier(prefix: &config::HotkeyPrefixConfig) -> bool {
        prefix.shift || prefix.ctrl || prefix.alt || prefix.logo
    }

    pub(crate) fn normalize_chart_timeframe_hotkey_prefix(
        mut prefix: config::HotkeyPrefixConfig,
    ) -> Option<config::HotkeyPrefixConfig> {
        if prefix.ctrl || prefix.alt || prefix.logo {
            prefix.shift = false;
        }

        Self::hotkey_prefix_has_modifier(&prefix).then_some(prefix)
    }

    pub(crate) fn hotkey_prefix_from_modifiers(
        modifiers: iced::keyboard::Modifiers,
    ) -> config::HotkeyPrefixConfig {
        config::HotkeyPrefixConfig {
            shift: modifiers.shift(),
            ctrl: modifiers.control(),
            alt: modifiers.alt(),
            logo: modifiers.logo(),
        }
    }

    pub(crate) fn hotkey_prefix_matches(
        prefix: &config::HotkeyPrefixConfig,
        modifiers: iced::keyboard::Modifiers,
    ) -> bool {
        Self::normalize_chart_timeframe_hotkey_prefix(*prefix).is_some_and(|prefix| {
            Self::normalize_chart_timeframe_hotkey_prefix(Self::hotkey_prefix_from_modifiers(
                modifiers,
            ))
            .is_some_and(|event_prefix| prefix == event_prefix)
        })
    }

    pub(crate) fn hotkey_has_prefix(
        hotkey: &config::HotkeyConfig,
        prefix: &config::HotkeyPrefixConfig,
    ) -> bool {
        let hotkey_prefix = config::HotkeyPrefixConfig {
            shift: hotkey.shift,
            ctrl: hotkey.ctrl,
            alt: hotkey.alt,
            logo: hotkey.logo,
        };

        Self::normalize_chart_timeframe_hotkey_prefix(*prefix).is_some_and(|prefix| {
            Self::normalize_chart_timeframe_hotkey_prefix(hotkey_prefix)
                .is_some_and(|hotkey_prefix| prefix == hotkey_prefix)
        })
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

    pub(crate) fn hotkey_prefix_display(prefix: &config::HotkeyPrefixConfig) -> String {
        let mut parts = Vec::new();
        if prefix.ctrl {
            parts.push("Ctrl".to_string());
        }
        if prefix.alt {
            parts.push("Alt".to_string());
        }
        if prefix.shift {
            parts.push("Shift".to_string());
        }
        if prefix.logo {
            parts.push("Win/Cmd".to_string());
        }
        parts.push(format!("1..{}", TIMEFRAME_OPTIONS.len()));
        parts.join(" + ")
    }

    pub(crate) fn chart_timeframe_for_hotkey_key(key_str: &str) -> Option<Timeframe> {
        let index = key_str.parse::<usize>().ok()?.checked_sub(1)?;
        TIMEFRAME_OPTIONS.get(index).copied()
    }

    pub(crate) fn hotkey_action_label(&self, action: &config::HotkeyAction) -> String {
        match action {
            config::HotkeyAction::AddCandlestickChart => "Add Candlestick Chart".to_string(),
            config::HotkeyAction::ChartTimeframePrefix => "Chart Timeframes".to_string(),
            config::HotkeyAction::OpenTradingJournal => "Open Trading Journal".to_string(),
            config::HotkeyAction::OpenWalletTracker => "Open Wallet Tracker".to_string(),
            config::HotkeyAction::OpenQuickSymbolSearch => "Quick Symbol Search".to_string(),
            config::HotkeyAction::OpenSettingsWindow => "Open Settings".to_string(),
            config::HotkeyAction::SwitchLayout { name } => format!("Switch to Layout: {name}"),
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

    pub(crate) fn available_hotkey_action_groups(&self) -> Vec<HotkeyActionGroup> {
        let mut groups = vec![HotkeyActionGroup {
            title: "General",
            actions: vec![
                (
                    config::HotkeyAction::AddCandlestickChart,
                    "Add Candlestick Chart".to_string(),
                ),
                (
                    config::HotkeyAction::ChartTimeframePrefix,
                    format!("Chart Timeframes 1..{}", TIMEFRAME_OPTIONS.len()),
                ),
                (
                    config::HotkeyAction::OpenTradingJournal,
                    "Open Trading Journal".to_string(),
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
            ],
        }];

        if !self.saved_layouts.is_empty() {
            groups.push(HotkeyActionGroup {
                title: "Layouts",
                actions: self
                    .saved_layouts
                    .iter()
                    .map(|layout| {
                        (
                            config::HotkeyAction::SwitchLayout {
                                name: layout.name.clone(),
                            },
                            layout.name.clone(),
                        )
                    })
                    .collect(),
            });
        }

        let account_actions: Vec<_> = self
            .accounts
            .iter()
            .filter(|profile| !self.ghost_account_secret_ids.contains(&profile.secret_id))
            .map(|profile| {
                (
                    config::HotkeyAction::SwitchAccount {
                        secret_id: profile.secret_id.clone(),
                    },
                    profile.name.clone(),
                )
            })
            .collect();
        if !account_actions.is_empty() {
            groups.push(HotkeyActionGroup {
                title: "Accounts",
                actions: account_actions,
            });
        }

        groups
    }
}
