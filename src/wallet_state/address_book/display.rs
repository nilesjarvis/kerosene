use super::WalletDisplay;
use crate::app_state::TradingTerminal;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Wallet Display Helpers
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn wallet_label(&self, address: &str) -> Option<&str> {
        let address = Self::normalize_wallet_address(address)?;
        self.wallet_label_for_normalized_address(&address)
    }

    /// Looks up an already normalized address, preferring a nonblank remote label.
    fn wallet_label_for_normalized_address(&self, address: &str) -> Option<&str> {
        [
            &self.wallet_tracker.remote_database.entries,
            &self.address_book,
        ]
        .into_iter()
        .filter_map(|book| book.get(address))
        .map(|entry| entry.label.trim())
        .find(|label| !label.is_empty())
    }

    pub(crate) fn wallet_display(&self, address: &str) -> WalletDisplay {
        let normalized = Self::normalize_wallet_address(address);
        let label = normalized
            .as_deref()
            .and_then(|address| self.wallet_label_for_normalized_address(address));
        let normalized = normalized.unwrap_or_else(|| address.to_string());
        let short = Self::short_address(&normalized);
        if let Some(label) = label {
            WalletDisplay {
                primary: label.to_string(),
                secondary: short,
                has_label: true,
            }
        } else {
            WalletDisplay {
                primary: short,
                secondary: normalized,
                has_label: false,
            }
        }
    }

    pub(crate) fn wallet_is_remote(&self, address: &str) -> bool {
        Self::normalize_wallet_address(address).is_some_and(|address| {
            self.wallet_tracker
                .remote_database
                .entries
                .contains_key(&address)
        })
    }

    pub(crate) fn wallet_detail_symbol(dex: &str, coin: &str) -> String {
        if dex.is_empty() || coin.contains(':') {
            coin.to_string()
        } else {
            format!("{dex}:{coin}")
        }
    }
}
