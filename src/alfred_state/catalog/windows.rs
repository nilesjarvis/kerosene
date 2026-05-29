use crate::alfred_state::{AlfredCommand, AlfredCommandId, AlfredCommandKind};
use crate::app_state::TradingTerminal;
use crate::message::Message;

use super::availability::open_tag;

// ---------------------------------------------------------------------------
// Window Commands
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn alfred_window_commands(&self) -> Vec<AlfredCommand> {
        vec![
            AlfredCommand::new(
                AlfredCommandId::OpenTradingJournal,
                "Trading Journal",
                "Open journal window",
                open_tag(self.journal.window_id.is_some(), "Window"),
                AlfredCommandKind::OpenWindow,
                Some(Message::AddTradingJournal),
                &["journal", "notes", "trades", "window", "open"],
            ),
            AlfredCommand::new(
                AlfredCommandId::OpenWalletTrackerWindow,
                "Wallet Tracker Window",
                "Open wallet tracker window",
                open_tag(self.wallet_tracker.window_id.is_some(), "Window"),
                AlfredCommandKind::OpenWindow,
                Some(Message::OpenWalletTrackerWindow),
                &["wallet", "tracker", "addresses", "window", "open"],
            ),
            AlfredCommand::new(
                AlfredCommandId::OpenScreenerWindow,
                "Screener",
                "Open screener window",
                open_tag(self.screener.window_id.is_some(), "Window"),
                AlfredCommandKind::OpenWindow,
                Some(Message::OpenScreenerWindow),
                &["screener", "ticker", "prices", "funding", "window", "open"],
            ),
            AlfredCommand::new(
                AlfredCommandId::OpenSettingsWindow,
                "Settings",
                "Open settings window",
                open_tag(self.settings_window_id.is_some(), "Window"),
                AlfredCommandKind::OpenWindow,
                Some(Message::OpenSettingsWindow),
                &["preferences", "config", "hotkeys", "window", "open"],
            ),
        ]
    }
}
