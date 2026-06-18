use crate::app_state::TradingTerminal;
use crate::config;
use crate::timeframe::TIMEFRAME_HOTKEY_OPTIONS;

impl TradingTerminal {
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
        parts.push(format!("1..{}", TIMEFRAME_HOTKEY_OPTIONS.len()));
        parts.join(" + ")
    }

    pub(crate) fn hotkey_action_label(&self, action: &config::HotkeyAction) -> String {
        match action {
            config::HotkeyAction::AddCandlestickChart => "Add Candlestick Chart".to_string(),
            config::HotkeyAction::ChartTimeframePrefix => "Chart Timeframes".to_string(),
            config::HotkeyAction::OpenAlfred => "alfred".to_string(),
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
}
