use serde::{Deserialize, Serialize};
use std::fmt;

#[cfg(test)]
mod tests;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HotkeyAction {
    AddCandlestickChart,
    ChartTimeframePrefix,
    OpenAlfred,
    OpenTradingJournal,
    OpenWalletTracker,
    OpenQuickSymbolSearch,
    OpenSettingsWindow,
    SwitchAccount { secret_id: String },
    SwitchLayout { name: String },
}

impl fmt::Debug for HotkeyAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AddCandlestickChart => f.write_str("AddCandlestickChart"),
            Self::ChartTimeframePrefix => f.write_str("ChartTimeframePrefix"),
            Self::OpenAlfred => f.write_str("OpenAlfred"),
            Self::OpenTradingJournal => f.write_str("OpenTradingJournal"),
            Self::OpenWalletTracker => f.write_str("OpenWalletTracker"),
            Self::OpenQuickSymbolSearch => f.write_str("OpenQuickSymbolSearch"),
            Self::OpenSettingsWindow => f.write_str("OpenSettingsWindow"),
            Self::SwitchAccount { .. } => f
                .debug_struct("SwitchAccount")
                .field("secret_id", &"<redacted>")
                .finish(),
            Self::SwitchLayout { name } => {
                f.debug_struct("SwitchLayout").field("name", name).finish()
            }
        }
    }
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
