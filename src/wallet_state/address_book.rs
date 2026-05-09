use crate::app_state::TradingTerminal;

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

#[derive(Debug, Clone)]
pub(crate) struct WalletDisplay {
    pub(crate) primary: String,
    pub(crate) secondary: String,
    pub(crate) has_label: bool,
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
