use super::super::AddressBookEntry;
use crate::app_state::TradingTerminal;

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Labeled Address Tracker Sync
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn labeled_wallet_addresses_from_address_book(
        address_book: &HashMap<String, AddressBookEntry>,
    ) -> Vec<String> {
        let mut addresses: Vec<_> = address_book
            .iter()
            .filter(|(_, entry)| !entry.label.trim().is_empty())
            .map(|(address, _)| address.clone())
            .collect();
        addresses.sort();
        addresses.dedup();
        addresses
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

    pub(crate) fn tracked_trade_subscription_addresses_from_address_book(
        address_book: &HashMap<String, AddressBookEntry>,
        muted_addresses: &[String],
    ) -> Vec<String> {
        Self::labeled_wallet_addresses_from_address_book(address_book)
            .into_iter()
            .filter(|address| !muted_addresses.contains(address))
            .collect()
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
        Self::labeled_wallet_addresses_from_address_book(&self.address_book)
    }

    pub(crate) fn tracked_trade_subscription_addresses(&self) -> Vec<String> {
        Self::tracked_trade_subscription_addresses_from_address_book(
            &self.address_book,
            &self.wallet_tracker.muted_addresses,
        )
    }
}
