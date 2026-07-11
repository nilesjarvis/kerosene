use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use super::super::image::{
    PnlCardImage, copy_pnl_card_to_clipboard, render_pnl_card_image, save_pnl_card_png,
};
use super::super::metrics::PnlCardMetrics;
use super::super::{PnlCardTarget, PnlCardWindowState};

use iced::{Task, window};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// PnL Card Export
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn copy_pnl_card_image(&mut self, window_id: window::Id) -> Task<Message> {
        let image = match self.pnl_card_export_image(window_id) {
            Ok(image) => image,
            Err(err) => {
                self.push_toast(err, true);
                return Task::none();
            }
        };

        Task::perform(
            async move { copy_pnl_card_to_clipboard(image).map_err(|err| err.to_string()) },
            |result| Message::PnlCardCopied(result.into()),
        )
    }

    pub(crate) fn save_pnl_card_image(&mut self, window_id: window::Id) -> Task<Message> {
        let image = match self.pnl_card_export_image(window_id) {
            Ok(image) => image,
            Err(err) => {
                self.push_toast(err, true);
                return Task::none();
            }
        };

        Task::perform(save_pnl_card_png(image), |result| {
            Message::PnlCardSaved(result.into())
        })
    }

    pub(crate) fn handle_pnl_card_copied(&mut self, result: Result<(), String>) -> Task<Message> {
        match result {
            Ok(()) => self.push_toast("PnL card copied to clipboard".to_string(), false),
            Err(err) => self.push_toast(
                format!(
                    "PnL card copy failed: {}",
                    redact_sensitive_response_text(&err)
                ),
                true,
            ),
        }
        Task::none()
    }

    pub(crate) fn handle_pnl_card_saved(
        &mut self,
        result: Result<Option<PathBuf>, String>,
    ) -> Task<Message> {
        match result {
            Ok(Some(path)) => {
                self.push_toast(format!("PnL card saved to {}", path.display()), false)
            }
            Ok(None) => {}
            Err(err) => self.push_toast(
                format!(
                    "PnL card save failed: {}",
                    redact_sensitive_response_text(&err)
                ),
                true,
            ),
        }
        Task::none()
    }

    pub(in crate::pnl_card) fn pnl_card_metrics_for_state(
        &self,
        state: &PnlCardWindowState,
    ) -> Result<PnlCardMetrics, String> {
        if !self.pnl_card_account_is_current(state) {
            return Err(self.stale_pnl_card_message(state));
        }

        match &state.target {
            PnlCardTarget::Position(coin) => self
                .position_pnl_card_metrics(coin)
                .ok_or_else(|| "Position is no longer open".to_string()),
            PnlCardTarget::Summary => self.summary_pnl_card_metrics(),
        }
    }

    fn pnl_card_account_is_current(&self, state: &PnlCardWindowState) -> bool {
        pnl_card_account_matches(self.connected_address.as_deref(), state)
    }

    fn stale_pnl_card_message(&self, state: &PnlCardWindowState) -> String {
        format!(
            "PnL card was opened for {}. Reopen it for the current account.",
            Self::short_address(&state.account_address)
        )
    }

    fn pnl_card_export_image(&self, window_id: window::Id) -> Result<PnlCardImage, String> {
        let state = self
            .pnl_card_windows
            .get(&window_id)
            .cloned()
            .ok_or_else(|| "PnL card not found".to_string())?;
        let metrics = self.pnl_card_metrics_for_state(&state)?;

        let theme = self.theme();
        let pnl_color = self.direction_color(&theme, metrics.upnl);
        render_pnl_card_image(
            &state,
            metrics,
            self.display_denomination_context(),
            pnl_color,
            &theme,
        )
    }
}

pub(in crate::pnl_card) fn pnl_card_account_matches(
    current_address: Option<&str>,
    state: &PnlCardWindowState,
) -> bool {
    current_address
        .and_then(TradingTerminal::normalize_wallet_address)
        .as_deref()
        .is_some_and(|address| address == state.account_address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KeroseneConfig;

    #[test]
    fn pnl_card_copy_error_redacts_toast_detail() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());

        let _ = terminal
            .handle_pnl_card_copied(Err("clipboard failed: auth_token=token-secret".to_string()));

        let toast = terminal.toasts.last().expect("toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("auth_token=<redacted>"));
        assert!(!toast.message.contains("token-secret"));
    }

    #[test]
    fn pnl_card_save_error_redacts_toast_detail() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());

        let _ = terminal
            .handle_pnl_card_saved(Err("save failed: client_secret=secret-value".to_string()));

        let toast = terminal.toasts.last().expect("toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("client_secret=<redacted>"));
        assert!(!toast.message.contains("secret-value"));
    }
}
