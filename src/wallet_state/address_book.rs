use crate::app_state::TradingTerminal;
use std::fmt;

mod display;
mod labels;

pub(crate) fn normalize_wallet_address_value(input: &str) -> Option<String> {
    let address = input.trim().to_lowercase();
    let hex = address.strip_prefix("0x")?;
    (hex.len() == 40 && hex.chars().all(|c| c.is_ascii_hexdigit())).then_some(address)
}

#[derive(Clone, Default)]
pub(crate) struct AddressBookEntry {
    pub(crate) label: String,
    pub(crate) color: Option<String>,
    pub(crate) tags: Vec<String>,
}

impl fmt::Debug for AddressBookEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AddressBookEntry")
            .field(
                "label",
                &(!self.label.trim().is_empty()).then_some("<redacted>"),
            )
            .field("color", &self.color.as_ref().map(|_| "<redacted>"))
            .field("tags", &format_args!("<{} redacted>", self.tags.len()))
            .finish()
    }
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
            .field("primary", &"<redacted>")
            .field("secondary", &"<redacted>")
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
    fn address_book_entry_debug_is_structural_and_preserves_values() {
        const LABEL: &str = "private-live-wallet-label-sentinel";
        const COLOR: &str = "#ff00ff";
        const TAG: &str = "private-live-wallet-tag-sentinel";
        let entry = AddressBookEntry {
            label: LABEL.to_string(),
            color: Some(COLOR.to_string()),
            tags: vec![TAG.to_string(), TEST_ADDRESS.to_string()],
        };

        let rendered = format!("{entry:?}");

        assert!(rendered.contains("label: Some(\"<redacted>\")"));
        assert!(rendered.contains("color: Some(\"<redacted>\")"));
        assert!(rendered.contains("tags: <2 redacted>"));
        for sensitive in [TEST_ADDRESS, LABEL, COLOR, TAG] {
            assert!(
                !rendered.contains(sensitive),
                "{sensitive} leaked in {rendered}"
            );
        }
        assert_eq!(entry.label, LABEL);
        assert_eq!(entry.color.as_deref(), Some(COLOR));
        assert_eq!(entry.tags, vec![TAG.to_string(), TEST_ADDRESS.to_string()]);
    }

    #[test]
    fn wallet_display_debug_redacts_full_and_short_addresses_without_changing_values() {
        let display =
            TradingTerminal::wallet_display_from_address_book(&HashMap::new(), TEST_ADDRESS);
        let primary = display.primary.clone();
        let secondary = display.secondary.clone();

        let rendered = format!("{display:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains(TEST_ADDRESS));
        assert!(!rendered.contains("0xabc0...0000"));
        assert!(rendered.contains("has_label: false"));
        assert_eq!(display.primary, primary);
        assert_eq!(display.secondary, secondary);
        assert!(!display.has_label);
    }

    #[test]
    fn labeled_wallet_display_debug_redacts_label_and_preserves_render_values() {
        const LABEL: &str = "private-wallet-display-label-sentinel";
        let mut address_book = HashMap::new();
        address_book.insert(
            TEST_ADDRESS.to_string(),
            AddressBookEntry {
                label: LABEL.to_string(),
                ..Default::default()
            },
        );
        let display =
            TradingTerminal::wallet_display_from_address_book(&address_book, TEST_ADDRESS);

        let rendered = format!("{display:?}");

        assert!(rendered.contains("primary: \"<redacted>\""), "{rendered}");
        assert!(rendered.contains("secondary: \"<redacted>\""), "{rendered}");
        assert!(rendered.contains("has_label: true"), "{rendered}");
        assert!(!rendered.contains(LABEL), "{rendered}");
        assert!(!rendered.contains("0xabc0...0000"), "{rendered}");
        assert_eq!(display.primary, LABEL);
        assert_eq!(display.secondary, "0xabc0...0000");
        assert!(display.has_label);
    }
}
