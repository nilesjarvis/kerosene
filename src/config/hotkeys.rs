use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HotkeyAction {
    AddCandlestickChart,
    ChartTimeframePrefix,
    OpenTradingJournal,
    OpenWalletTracker,
    OpenQuickSymbolSearch,
    OpenSettingsWindow,
    SwitchAccount { secret_id: String },
    SwitchLayout { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HotkeyConfig {
    pub action: HotkeyAction,
    pub key: String,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub logo: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct HotkeyPrefixConfig {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub logo: bool,
}
