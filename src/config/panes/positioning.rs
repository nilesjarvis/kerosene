use crate::positioning_state::{
    PositioningInfoChangeSortField, PositioningInfoChangeTimeframe, PositioningInfoPage,
    PositioningInfoSide, PositioningInfoSortField,
};
use serde::{Deserialize, Serialize};

use super::super::{SortDirection, default_symbol};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PositioningInfoConfig {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub page: PositioningInfoPage,
    #[serde(default = "default_symbol")]
    pub symbol: String,
    #[serde(default)]
    pub side: PositioningInfoSide,
    #[serde(default)]
    pub sort_field: PositioningInfoSortField,
    #[serde(default = "default_positioning_info_sort_direction")]
    pub sort_direction: SortDirection,
    #[serde(default)]
    pub change_timeframe: PositioningInfoChangeTimeframe,
    #[serde(default)]
    pub change_sort_field: PositioningInfoChangeSortField,
    #[serde(default = "default_positioning_info_change_sort_direction")]
    pub change_sort_direction: SortDirection,
}

fn default_positioning_info_sort_direction() -> SortDirection {
    PositioningInfoSortField::default().default_direction()
}

fn default_positioning_info_change_sort_direction() -> SortDirection {
    PositioningInfoChangeSortField::default().default_direction()
}
