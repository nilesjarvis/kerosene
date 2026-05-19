use crate::account::AssetContext;
use crate::config;
use crate::hyperdash_api::{PerpDeltas, TickerPositions};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Positioning Information State
// ---------------------------------------------------------------------------

pub(crate) type PositioningInfoId = u64;

pub(crate) const POSITIONING_INFO_LIMIT: u32 = 30;
pub(crate) const POSITIONING_INFO_OFFSET: u32 = 0;
pub(crate) const POSITIONING_CHANGE_ROW_LIMIT: usize = 500;

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

#[derive(Debug, Clone)]
pub(crate) struct PositioningInfoInstance {
    pub(crate) id: PositioningInfoId,
    pub(crate) page: PositioningInfoPage,
    pub(crate) symbol: String,
    pub(crate) search_query: String,
    pub(crate) side: PositioningInfoSide,
    pub(crate) sort_field: PositioningInfoSortField,
    pub(crate) sort_direction: config::SortDirection,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    pub(crate) data: Option<TickerPositions>,
    pub(crate) asset_ctx: Option<AssetContext>,
    pub(crate) asset_ctx_updated_at_ms: Option<u64>,
    pub(crate) change_timeframe: PositioningInfoChangeTimeframe,
    pub(crate) change_sort_field: PositioningInfoChangeSortField,
    pub(crate) change_sort_direction: config::SortDirection,
    pub(crate) change_loading: bool,
    pub(crate) change_error: Option<String>,
    pub(crate) change_data: Option<PerpDeltas>,
    pub(crate) change_last_fetch_ms: Option<u64>,
    pub(crate) change_pending_key: Option<String>,
    pub(crate) last_fetch_ms: Option<u64>,
    pub(crate) pending_key: Option<String>,
}

impl PositioningInfoInstance {
    pub(crate) fn new(id: PositioningInfoId, symbol: String) -> Self {
        Self {
            id,
            page: PositioningInfoPage::default(),
            symbol,
            search_query: String::new(),
            side: PositioningInfoSide::default(),
            sort_field: PositioningInfoSortField::default(),
            sort_direction: default_positioning_sort_direction(),
            loading: false,
            error: None,
            data: None,
            asset_ctx: None,
            asset_ctx_updated_at_ms: None,
            change_timeframe: PositioningInfoChangeTimeframe::default(),
            change_sort_field: PositioningInfoChangeSortField::default(),
            change_sort_direction: default_positioning_change_sort_direction(),
            change_loading: false,
            change_error: None,
            change_data: None,
            change_last_fetch_ms: None,
            change_pending_key: None,
            last_fetch_ms: None,
            pending_key: None,
        }
    }

    pub(crate) fn has_active_filters(&self) -> bool {
        self.side != PositioningInfoSide::default()
            || self.sort_field != PositioningInfoSortField::default()
            || self.sort_direction != default_positioning_sort_direction()
    }

    pub(crate) fn reset_filters(&mut self) {
        self.side = PositioningInfoSide::default();
        self.sort_field = PositioningInfoSortField::default();
        self.sort_direction = default_positioning_sort_direction();
    }

    pub(crate) fn normalize_removed_filters(&mut self) {
        if self.sort_field == PositioningInfoSortField::CopyScore {
            self.sort_field = PositioningInfoSortField::default();
            self.sort_direction = default_positioning_sort_direction();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positioning_info_filters_track_side_and_sort_changes() {
        let mut instance = PositioningInfoInstance::new(7, "HYPE".to_string());

        assert!(!instance.has_active_filters());

        instance.side = PositioningInfoSide::Long;
        assert!(instance.has_active_filters());

        instance.reset_filters();
        assert!(!instance.has_active_filters());

        instance.sort_field = PositioningInfoSortField::NotionalSize;
        assert!(instance.has_active_filters());

        instance.reset_filters();
        instance.sort_direction = config::SortDirection::Ascending;
        assert!(instance.has_active_filters());
    }

    #[test]
    fn positioning_info_removed_copy_sort_normalizes_to_default_sort() {
        let mut instance = PositioningInfoInstance::new(7, "HYPE".to_string());
        instance.side = PositioningInfoSide::Short;
        instance.sort_field = PositioningInfoSortField::CopyScore;
        instance.sort_direction = config::SortDirection::Ascending;

        instance.normalize_removed_filters();

        assert_eq!(instance.side, PositioningInfoSide::Short);
        assert_eq!(instance.sort_field, PositioningInfoSortField::UnrealizedPnl);
        assert_eq!(instance.sort_direction, config::SortDirection::Descending);
    }

    #[test]
    fn positioning_notional_and_size_sorts_use_hyperdash_notional_enum_name() {
        assert_eq!(
            PositioningInfoSortField::NotionalSize.api_field(),
            "notional"
        );
        assert_eq!(PositioningInfoSortField::Size.api_field(), "notional");
    }

    #[test]
    fn positioning_change_nav_label_uses_delta_symbol() {
        assert_eq!(PositioningInfoPage::Change.label(), "\u{0394} Change");
    }

    #[test]
    fn positioning_change_defaults_to_short_timeframe_and_largest_change() {
        let instance = PositioningInfoInstance::new(7, "HYPE".to_string());

        assert_eq!(
            instance.change_timeframe,
            PositioningInfoChangeTimeframe::FifteenMinutes
        );
        assert_eq!(
            instance.change_sort_field,
            PositioningInfoChangeSortField::Change
        );
        assert_eq!(
            instance.change_sort_direction,
            config::SortDirection::Descending
        );
    }
}
