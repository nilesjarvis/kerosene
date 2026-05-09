use crate::config;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AccountPickerOption {
    pub(crate) index: usize,
    pub(crate) label: String,
    pub(crate) address: String,
    pub(crate) can_trade: bool,
    pub(crate) is_ghost: bool,
}

impl std::fmt::Display for AccountPickerOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
