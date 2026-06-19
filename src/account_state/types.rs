use crate::{config, helpers::redact_wallet_address_debug_value};

use std::fmt;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct AccountPickerOption {
    pub(crate) index: usize,
    pub(crate) label: String,
    pub(crate) address: String,
    pub(crate) can_trade: bool,
    pub(crate) is_ghost: bool,
}

impl fmt::Debug for AccountPickerOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccountPickerOption")
            .field("index", &self.index)
            .field(
                "label",
                &redact_wallet_address_debug_value(self.label.trim()),
            )
            .field(
                "address",
                &redact_wallet_address_debug_value(self.address.trim()),
            )
            .field("can_trade", &self.can_trade)
            .field("is_ghost", &self.is_ghost)
            .finish()
    }
}

impl fmt::Display for AccountPickerOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BottomTab {
    Positions,
    OpenOrders,
    Balances,
    TradeHistory,
    FundingHistory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PositionsSortColumn {
    Symbol,
    Side,
    Size,
    Entry,
    Liquidation,
    Mark,
    Value,
    UnrealizedPnl,
    Funding,
    TotalPnl,
    Leverage,
}

impl PositionsSortColumn {
    pub(crate) fn default_direction(self) -> config::SortDirection {
        match self {
            Self::Symbol | Self::Side => config::SortDirection::Ascending,
            Self::Size
            | Self::Entry
            | Self::Liquidation
            | Self::Mark
            | Self::Value
            | Self::UnrealizedPnl
            | Self::Funding
            | Self::TotalPnl
            | Self::Leverage => config::SortDirection::Descending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AccountPickerOption;

    #[test]
    fn account_picker_option_debug_redacts_wallet_addresses() {
        let label_address = "0xabc0000000000000000000000000000000000000";
        let account_address = "0xdef0000000000000000000000000000000000000";
        let option = AccountPickerOption {
            index: 2,
            label: label_address.to_string(),
            address: account_address.to_string(),
            can_trade: true,
            is_ghost: false,
        };

        let rendered = format!("{option:?}");

        assert!(!rendered.contains(label_address));
        assert!(!rendered.contains(account_address));
        assert!(rendered.contains("<redacted>"));
        assert!(rendered.contains("can_trade: true"));
    }

    #[test]
    fn account_picker_option_debug_keeps_non_address_label() {
        let account_address = "0xdef0000000000000000000000000000000000000";
        let option = AccountPickerOption {
            index: 0,
            label: "Main account".to_string(),
            address: account_address.to_string(),
            can_trade: false,
            is_ghost: true,
        };

        let rendered = format!("{option:?}");

        assert!(rendered.contains("Main account"));
        assert!(!rendered.contains(account_address));
        assert!(rendered.contains("is_ghost: true"));
    }
}
