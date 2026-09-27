use super::{AddressBookEntry, WalletDisplay};
use crate::app_state::TradingTerminal;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Wallet Display Helpers
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn wallet_label_from_address_book<'a>(
        address_book: &'a HashMap<String, AddressBookEntry>,
        address: &str,
    ) -> Option<&'a str> {
        let address = Self::normalize_wallet_address(address)?;
        address_book
            .get(&address)
            .map(|entry| entry.label.trim())
            .filter(|label| !label.is_empty())
    }

    pub(crate) fn wallet_display_from_address_book(
        address_book: &HashMap<String, AddressBookEntry>,
        address: &str,
    ) -> WalletDisplay {
        let normalized =
            Self::normalize_wallet_address(address).unwrap_or_else(|| address.to_string());
        let short = Self::short_address(&normalized);
        if let Some(label) = Self::wallet_label_from_address_book(address_book, &normalized) {
            WalletDisplay {
                primary: label.to_string(),
                secondary: short,
                has_label: true,
            }
        } else {
            WalletDisplay {
                primary: short.clone(),
                secondary: normalized,
                has_label: false,
            }
        }
    }

    pub(crate) fn wallet_label(&self, address: &str) -> Option<&str> {
        Self::wallet_label_from_address_book(&self.wallet_tracker.remote_database.entries, address)
            .or_else(|| Self::wallet_label_from_address_book(&self.address_book, address))
    }

    pub(crate) fn wallet_is_remote(&self, address: &str) -> bool {
        Self::normalize_wallet_address(address).is_some_and(|address| {
            self.wallet_tracker
                .remote_database
                .entries
                .contains_key(&address)
        })
    }

    pub(crate) fn wallet_display(&self, address: &str) -> WalletDisplay {
        if Self::wallet_label_from_address_book(
            &self.wallet_tracker.remote_database.entries,
            address,
        )
        .is_some()
        {
            Self::wallet_display_from_address_book(
                &self.wallet_tracker.remote_database.entries,
                address,
            )
        } else {
            Self::wallet_display_from_address_book(&self.address_book, address)
        }
    }

    pub(crate) fn wallet_detail_symbol(dex: &str, coin: &str) -> String {
        if dex.is_empty() || coin.contains(':') {
            coin.to_string()
        } else {
            format!("{dex}:{coin}")
        }
    }
}
