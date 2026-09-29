use super::super::AddressBookEntry;
use crate::app_state::TradingTerminal;

use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Labeled Address Tracker Sync
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn labeled_wallet_addresses_from_address_book(
        address_book: &HashMap<String, AddressBookEntry>,
    ) -> Vec<String> {
        labeled_addresses_from_entries(address_book.iter())
    }

    pub(crate) fn add_labeled_addresses_to_wallet_tracker(
        tracked_addresses: &mut Vec<String>,
        address_book: &HashMap<String, AddressBookEntry>,
    ) -> Vec<String> {
        let mut added = Vec::new();
        for address in Self::labeled_wallet_addresses_from_address_book(address_book) {
            if !tracked_addresses.contains(&address) {
                tracked_addresses.push(address.clone());
                added.push(address);
            }
        }
        added
    }

    pub(crate) fn sync_labeled_addresses_to_wallet_tracker(&mut self) -> Vec<String> {
        let added = Self::add_labeled_addresses_to_wallet_tracker(
            &mut self.wallet_tracker.tracked_addresses,
            &self.address_book,
        );
        for address in &added {
            self.wallet_tracker.rows.entry(address.clone()).or_default();
        }
        added
    }

    pub(crate) fn labeled_wallet_addresses(&self) -> Vec<String> {
        labeled_addresses_from_entries(
            self.address_book
                .iter()
                .chain(self.wallet_tracker.remote_database.entries.iter()),
        )
    }

    pub(crate) fn tracked_trade_subscription_addresses(&self) -> Vec<String> {
        self.labeled_wallet_addresses()
            .into_iter()
            .filter(|address| !self.wallet_tracker.muted_addresses.contains(address))
            .collect()
    }
}

fn labeled_addresses_from_entries<'a>(
    entries: impl Iterator<Item = (&'a String, &'a AddressBookEntry)>,
) -> Vec<String> {
    let mut addresses: Vec<_> = entries
        .filter(|(_, entry)| !entry.label.trim().is_empty())
        .map(|(address, _)| address.clone())
        .collect();
    addresses.sort();
    addresses.dedup();
    addresses
}
