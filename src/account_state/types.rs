use crate::config;

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
            .field("label", &"<redacted>")
            .field("address", &"<redacted>")
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ActiveAccountSource {
    #[default]
    Hyperliquid,
    Schwab,
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
    fn account_picker_option_debug_redacts_address_shaped_identity() {
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
        assert_eq!(option.label, label_address);
        assert_eq!(option.address, account_address);
    }

    #[test]
    fn account_picker_option_debug_redacts_ordinary_label_but_display_preserves_it() {
        const LABEL: &str = "private-account-picker-label-sentinel";
        let account_address = "0xdef0000000000000000000000000000000000000";
        let option = AccountPickerOption {
            index: 0,
            label: LABEL.to_string(),
            address: account_address.to_string(),
            can_trade: false,
            is_ghost: true,
        };

        let rendered = format!("{option:?}");

        assert!(!rendered.contains(LABEL));
        assert!(!rendered.contains(account_address));
        assert!(rendered.contains("label: \"<redacted>\""));
        assert!(rendered.contains("is_ghost: true"));
        assert_eq!(option.label, LABEL);
        assert_eq!(option.address, account_address);
        assert_eq!(option.to_string(), LABEL);
    }
}
