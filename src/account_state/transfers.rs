use crate::account::transfers::{TransferEntry, TransferProvider, TransferSnapshot};
use std::collections::HashMap;

pub(crate) const TRANSFER_PAGE_SIZE: usize = 50;

#[derive(Default)]
pub(crate) struct TransferHistoryState {
    pub(crate) address: Option<String>,
    pub(crate) generation: u64,
    pub(crate) native: TransferSourceState,
    pub(crate) unit: TransferSourceState,
    pub(crate) entries: Vec<TransferEntry>,
    pub(crate) page: usize,
    pub(crate) expanded: Option<String>,
}

#[derive(Default)]
pub(crate) struct TransferSourceState {
    pub(crate) loading: bool,
    pub(crate) loaded: bool,
    pub(crate) error: Option<String>,
    pub(crate) warning: Option<String>,
    pub(crate) next_start: Option<u64>,
}

impl TransferHistoryState {
    pub(crate) fn clear(&mut self) {
        *self = Self {
            generation: self.generation.wrapping_add(1),
            ..Self::default()
        };
    }

    pub(crate) fn loading(&self) -> bool {
        self.native.loading || self.unit.loading
    }

    pub(crate) fn begin(&mut self, address: &str) -> Option<u64> {
        if self.address.as_deref() != Some(address) {
            self.clear();
            self.address = Some(address.to_string());
        }
        if self.loading() {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.native.loading = true;
        self.unit.loading = true;
        Some(self.generation)
    }

    pub(crate) fn apply(
        &mut self,
        address: &str,
        generation: u64,
        provider: TransferProvider,
        result: Result<TransferSnapshot, String>,
    ) {
        if self.address.as_deref() != Some(address) || self.generation != generation {
            return;
        }
        let source = match provider {
            TransferProvider::Hyperliquid => &mut self.native,
            TransferProvider::Unit => &mut self.unit,
        };
        if !source.loading {
            return;
        }
        source.loading = false;
        match result {
            Ok(snapshot) => {
                source.loaded = true;
                source.error = None;
                source.warning = snapshot.warning;
                source.next_start = snapshot.next_start;
                // Unit returns a full snapshot (including pending operations).
                // Native ledger reads are incremental; merge their boundary.
                if provider == TransferProvider::Unit {
                    self.entries.retain(|entry| entry.provider != provider);
                }
                let mut entries: HashMap<_, _> = self
                    .entries
                    .drain(..)
                    .map(|entry| (entry.id.clone(), entry))
                    .collect();
                for entry in snapshot.entries {
                    entries.insert(entry.id.clone(), entry);
                }
                self.entries = entries.into_values().collect();
                self.entries
                    .sort_by(|a, b| b.time.cmp(&a.time).then_with(|| a.id.cmp(&b.id)));
                self.page = self
                    .page
                    .min(self.entries.len().saturating_sub(1) / TRANSFER_PAGE_SIZE);
            }
            Err(error) => source.error = Some(error),
        }
    }
}

#[cfg(test)]
mod tests;
