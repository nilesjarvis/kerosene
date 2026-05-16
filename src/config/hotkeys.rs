use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HotkeyAction {
    AddCandlestickChart,
    OpenTradingJournal,
    OpenWalletTracker,
    OpenQuickSymbolSearch,
    OpenSettingsWindow,
    SwitchAccount { secret_id: String },
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
