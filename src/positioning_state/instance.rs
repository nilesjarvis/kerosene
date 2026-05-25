use crate::account::AssetContext;
use crate::config;
use crate::hyperdash_api::{PerpDeltas, TickerPositions};

use super::{
    PositioningInfoChangeSortField, PositioningInfoChangeTimeframe, PositioningInfoId,
    PositioningInfoPage, PositioningInfoSide, PositioningInfoSortField,
    default_positioning_change_sort_direction, default_positioning_sort_direction,
};

// ---------------------------------------------------------------------------
// Positioning Information Instance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) struct PositioningInfoInstance {
    pub(crate) id: PositioningInfoId,
    pub(crate) page: PositioningInfoPage,
    pub(crate) symbol: String,
    pub(crate) search_query: String,
    pub(crate) symbol_picker_open: bool,
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
            symbol_picker_open: false,
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
