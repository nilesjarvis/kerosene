use super::super::{AddressBookEntry, WalletLabelsImportSummary};
use crate::app_state::TradingTerminal;
use crate::config::{self, AddressBookEntryConfig};

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Wallet Label Import / Export
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn wallet_label_export_entries_from_address_book(
        address_book: &HashMap<String, AddressBookEntry>,
    ) -> Vec<AddressBookEntryConfig> {
        Self::address_book_config_from_entries(address_book)
            .into_iter()
            .filter(|entry| {
                !entry.label.trim().is_empty() || entry.color.is_some() || !entry.tags.is_empty()
            })
            .collect()
    }

    pub(crate) fn wallet_labels_export_with_time(
        &self,
        exported_at_ms: u64,
    ) -> config::WalletLabelsExport {
        config::WalletLabelsExport {
            schema: config::WALLET_LABELS_EXPORT_SCHEMA.to_string(),
            exported_at_ms,
            labels: Self::wallet_label_export_entries_from_address_book(&self.address_book),
        }
    }

    pub(crate) fn merge_wallet_label_export(
        address_book: &mut HashMap<String, AddressBookEntry>,
        export: config::WalletLabelsExport,
    ) -> Result<WalletLabelsImportSummary, String> {
        if export.schema != config::WALLET_LABELS_EXPORT_SCHEMA {
            return Err(format!(
                "Unsupported wallet label schema: {}",
                export.schema
            ));
        }

        let mut summary = WalletLabelsImportSummary::default();
        for imported in export.labels {
            let Some(address) = Self::normalize_wallet_address(&imported.address) else {
                summary.skipped_invalid += 1;
                continue;
            };

            let label = imported.label.trim().to_string();
            let color = imported
                .color
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string);
            let tags: Vec<String> = imported
                .tags
                .into_iter()
                .map(|tag| tag.trim().to_string())
                .filter(|tag| !tag.is_empty())
                .fold(Vec::new(), |mut acc, tag| {
                    if !acc.contains(&tag) {
                        acc.push(tag);
                    }
                    acc
                });

            if label.is_empty() && color.is_none() && tags.is_empty() {
                summary.skipped_empty += 1;
                continue;
            }

            if let Some(existing) = address_book.get_mut(&address) {
                let mut changed = false;

                if existing.label.trim().is_empty() && !label.is_empty() {
                    existing.label = label;
                    changed = true;
                }

                if existing.color.is_none() && color.is_some() {
                    existing.color = color;
                    changed = true;
                }

                for tag in tags {
                    if !existing.tags.contains(&tag) {
                        existing.tags.push(tag);
                        changed = true;
                    }
                }

                if changed {
                    summary.updated += 1;
                } else {
                    summary.preserved += 1;
                }
            } else {
                address_book.insert(address, AddressBookEntry { label, color, tags });
                summary.added += 1;
            }
        }

        Ok(summary)
    }
}
