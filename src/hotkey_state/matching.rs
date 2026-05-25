use crate::app_state::TradingTerminal;
use crate::config;
use crate::timeframe::{TIMEFRAME_OPTIONS, Timeframe};

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
        if key_str == "Space" {
            return modifiers.shift() || modifiers.control() || modifiers.alt() || modifiers.logo();
        }

        if [
            "Shift",
            "Control",
            "Alt",
            "Meta",
            "Super",
            "Logo",
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

    pub(crate) fn chart_timeframe_for_hotkey_key(key_str: &str) -> Option<Timeframe> {
        let index = key_str.parse::<usize>().ok()?.checked_sub(1)?;
        TIMEFRAME_OPTIONS.get(index).copied()
    }

    pub(crate) fn hotkey_key_is_modifier(key_str: &str) -> bool {
        ["Shift", "Control", "Alt", "Meta", "Super", "Logo"].contains(&key_str)
    }
}
