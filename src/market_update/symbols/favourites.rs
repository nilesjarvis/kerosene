use crate::api::{ExchangeSymbolsPayload, spot_symbol_for_indexed_key};
use crate::app_state::TradingTerminal;
use std::collections::HashSet;

impl TradingTerminal {
    /// Only a live, successfully loaded market family can prove a favourite
    /// was delisted. Search/risk filters and missing price contexts cannot.
    pub(super) fn prune_delisted_favourites(&mut self, payload: &ExchangeSymbolsPayload) -> bool {
        if payload.loaded_from_cache {
            return false;
        }

        let listed_keys: HashSet<&str> = payload
            .symbols
            .iter()
            .map(|symbol| symbol.key.as_str())
            .collect();
        let keep = |key: &String| {
            let metadata_failed = if key.starts_with('#') {
                payload.outcome_meta_failed
            } else if key.starts_with('@') || key.contains('/') {
                payload.spot_meta_failed
            } else {
                payload.perp_meta_failed
            };
            metadata_failed
                || listed_keys.contains(key.as_str())
                // Keep legacy indexed aliases for API-named spot pairs.
                || spot_symbol_for_indexed_key(&payload.symbols, key).is_some()
        };

        let previous_len = self.favourite_symbols.len();
        self.favourite_symbols.retain(keep);
        let favourites_changed = self.favourite_symbols.len() != previous_len;
        let mut layouts_changed = false;
        for layout in &mut self.saved_layouts {
            let previous_len = layout.favourite_symbols.len();
            layout.favourite_symbols.retain(keep);
            layouts_changed |= layout.favourite_symbols.len() != previous_len;
        }

        if favourites_changed {
            self.ticker_tape_ctxs
                .retain(|key, _| self.favourite_symbols.contains(key));
        }
        if favourites_changed || layouts_changed {
            self.persist_config();
        }
        favourites_changed
    }
}
