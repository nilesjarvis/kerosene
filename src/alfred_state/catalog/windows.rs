use crate::alfred_state::{AlfredCommand, AlfredCommandId, AlfredCommandKind};
use crate::app_state::TradingTerminal;
use crate::message::Message;

use super::availability::open_tag;

// ---------------------------------------------------------------------------
// Window Commands
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn alfred_window_commands(&self) -> Vec<AlfredCommand> {
        let mut commands = vec![
            AlfredCommand::new(
                AlfredCommandId::CreateCanvas,
                "New Canvas",
                "Open a customizable workspace window",
                "Window",
                AlfredCommandKind::OpenWindow,
                Some(Message::CreateCanvas),
                &["canvas", "workspace", "monitor", "window", "new", "open"],
            ),
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
                AlfredCommandId::OpenWalletClustersWindow,
                "Wallet Clusters Window",
                "Open wallet clusters window",
                open_tag(self.wallet_clusters.window_id.is_some(), "Window"),
                AlfredCommandKind::OpenWindow,
                Some(Message::OpenWalletClustersWindow),
                &[
                    "wallet", "clusters", "cluster", "split", "orders", "window", "open",
                ],
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
        ];

        commands.extend(self.canvases.iter().map(|(id, canvas)| {
            let open = canvas.window_id.is_some();
            AlfredCommand::new(
                AlfredCommandId::OpenCanvas(*id),
                "Canvas",
                "Open Canvas workspace window",
                open_tag(open, "Closed"),
                AlfredCommandKind::OpenWindow,
                Some(Message::OpenCanvas(*id)),
                &["canvas", "workspace", "monitor", "window", "open", "reopen"],
            )
            .with_dynamic_text(
                canvas.label.clone(),
                format!("Open {} workspace window", canvas.label),
                open_tag(open, "Closed").to_string(),
            )
        }));

        commands
    }
}
