use super::super::AddressBookEntry;
use crate::app_state::TradingTerminal;
use crate::config::{AddressBookEntryConfig, KeroseneConfig};

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Address Book Config Conversion
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn build_address_book(cfg: &KeroseneConfig) -> HashMap<String, AddressBookEntry> {
        let mut address_book = HashMap::new();

        for entry in &cfg.address_book {
            if let Some(address) = Self::normalize_wallet_address(&entry.address) {
                address_book.insert(
                    address,
                    AddressBookEntry {
                        label: entry.label.trim().to_string(),
                        color: entry.color.clone(),
                        tags: entry.tags.clone(),
                    },
                );
            }
        }

        for wallet in &cfg.wallet_tracker.wallets {
            if let Some(address) = Self::normalize_wallet_address(&wallet.address) {
                let label = wallet.label.trim();
                if !label.is_empty() {
                    address_book
                        .entry(address)
                        .or_insert_with(|| AddressBookEntry {
                            label: label.to_string(),
                            ..Default::default()
                        });
                }
            }
        }

        address_book
    }

    pub(crate) fn address_book_config(&self) -> Vec<AddressBookEntryConfig> {
        Self::address_book_config_from_entries(&self.address_book)
    }

    pub(crate) fn address_book_config_from_entries(
        address_book: &HashMap<String, AddressBookEntry>,
    ) -> Vec<AddressBookEntryConfig> {
        let mut entries: Vec<_> = address_book
            .iter()
            .map(|(address, entry)| AddressBookEntryConfig {
                address: address.clone(),
                label: entry.label.clone(),
                color: entry.color.clone(),
                tags: entry.tags.clone(),
            })
            .collect();
        entries.sort_by(|a, b| a.address.cmp(&b.address));
        entries
    }
}
