use crate::config;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Positioning Information Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) enum PositioningInfoPage {
    #[default]
    Positions,
    Change,
}

impl PositioningInfoPage {
    pub(crate) const ALL: [Self; 2] = [Self::Positions, Self::Change];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Positions => "Positions",
            Self::Change => "\u{0394} Change",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) enum PositioningInfoSide {
    #[default]
    All,
    Long,
    Short,
}

impl PositioningInfoSide {
    pub(crate) const ALL: [Self; 3] = [Self::All, Self::Long, Self::Short];

    pub(crate) fn api_value(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Long => "long",
            Self::Short => "short",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Long => "Long",
            Self::Short => "Short",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) enum PositioningInfoSortField {
    #[default]
    UnrealizedPnl,
    NotionalSize,
    AccountValue,
    CopyScore,
    Size,
}

impl PositioningInfoSortField {
    pub(crate) fn api_field(self) -> &'static str {
        match self {
            Self::UnrealizedPnl => "unrealizedPnl",
            Self::NotionalSize => "notional",
            Self::AccountValue => "accountValue",
            Self::CopyScore => "copyScore",
            Self::Size => "notional",
        }
    }

    pub(crate) fn default_direction(self) -> config::SortDirection {
        let _ = self;
        config::SortDirection::Descending
    }
}

pub(crate) fn default_positioning_sort_direction() -> config::SortDirection {
    PositioningInfoSortField::default().default_direction()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) enum PositioningInfoChangeTimeframe {
    #[default]
    FifteenMinutes,
    OneHour,
    FourHours,
}

impl PositioningInfoChangeTimeframe {
    pub(crate) const ALL: [Self; 3] = [Self::FifteenMinutes, Self::OneHour, Self::FourHours];

    pub(crate) fn api_value(self) -> &'static str {
        match self {
            Self::FifteenMinutes => "FIFTEEN_MINUTES",
            Self::OneHour => "ONE_HOUR",
            Self::FourHours => "FOUR_HOURS",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::FifteenMinutes => "15m",
            Self::OneHour => "1h",
            Self::FourHours => "4h",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) enum PositioningInfoChangeSortField {
    Trader,
    Previous,
    Current,
    #[default]
    Change,
    CurrentUsd,
    ChangeUsd,
}

impl PositioningInfoChangeSortField {
    pub(crate) fn default_direction(self) -> config::SortDirection {
        match self {
            Self::Trader => config::SortDirection::Ascending,
            Self::Previous | Self::Current | Self::Change | Self::CurrentUsd | Self::ChangeUsd => {
                config::SortDirection::Descending
            }
        }
    }
}

pub(crate) fn default_positioning_change_sort_direction() -> config::SortDirection {
    PositioningInfoChangeSortField::default().default_direction()
}
