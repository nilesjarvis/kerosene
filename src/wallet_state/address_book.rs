use crate::app_state::TradingTerminal;
use crate::helpers::redact_wallet_address_debug_value;
use std::fmt;

mod display;
mod labels;

pub(crate) fn normalize_wallet_address_value(input: &str) -> Option<String> {
    let address = input.trim().to_lowercase();
    let hex = address.strip_prefix("0x")?;
    (hex.len() == 40 && hex.chars().all(|c| c.is_ascii_hexdigit())).then_some(address)
}

#[derive(Debug, Clone, Default)]
pub(crate) struct AddressBookEntry {
    pub(crate) label: String,
    pub(crate) color: Option<String>,
    pub(crate) tags: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct WalletDisplay {
    pub(crate) primary: String,
    pub(crate) secondary: String,
    pub(crate) has_label: bool,
}

impl fmt::Debug for WalletDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletDisplay")
            .field("primary", &redact_wallet_address_debug_value(&self.primary))
            .field(
                "secondary",
                &redact_wallet_address_debug_value(&self.secondary),
            )
            .field("has_label", &self.has_label)
            .finish()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WalletLabelsImportSummary {
    pub(crate) added: usize,
    pub(crate) updated: usize,
    pub(crate) preserved: usize,
    pub(crate) skipped_invalid: usize,
    pub(crate) skipped_empty: usize,
}

impl WalletLabelsImportSummary {
    pub(crate) fn changed(&self) -> usize {
        self.added + self.updated
    }

    pub(crate) fn skipped(&self) -> usize {
        self.skipped_invalid + self.skipped_empty
    }

    pub(crate) fn toast_text(&self) -> String {
        format!(
            "Wallet labels imported: {} added, {} updated, {} preserved, {} skipped",
            self.added,
            self.updated,
            self.preserved,
            self.skipped()
        )
    }
}

impl TradingTerminal {
    pub(crate) fn normalize_wallet_address(input: &str) -> Option<String> {
        normalize_wallet_address_value(input)
    }

    pub(crate) fn short_address(address: &str) -> String {
        let char_count = address.chars().count();
        if char_count > 10 {
            let prefix: String = address.chars().take(6).collect();
            let suffix: String = address.chars().skip(char_count - 4).collect();
            format!("{prefix}...{suffix}")
        } else {
            address.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const TEST_ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

    #[test]
    fn wallet_display_debug_redacts_full_addresses() {
        let display =
            TradingTerminal::wallet_display_from_address_book(&HashMap::new(), TEST_ADDRESS);

        let rendered = format!("{display:?}");

        assert!(rendered.contains("0xabc0...0000"));
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains(TEST_ADDRESS));
    }
}
