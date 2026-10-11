use crate::account::AssetContext;
use crate::config;
use crate::helpers::parse_positive_finite_number;
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
    pub(crate) entry_min_input: String,
    pub(crate) entry_max_input: String,
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
    pub(crate) fn from_config(config: &config::PositioningInfoConfig, symbol: String) -> Self {
        let mut instance = Self::new(config.id, symbol);
        instance.page = config.page;
        instance.side = config.side;
        instance.sort_field = config.sort_field;
        instance.sort_direction = config.sort_direction;
        instance.entry_min_input = config.entry_min.clone();
        instance.entry_max_input = config.entry_max.clone();
        instance.change_timeframe = config.change_timeframe;
        instance.change_sort_field = config.change_sort_field;
        instance.change_sort_direction = config.change_sort_direction;
        instance.normalize_removed_filters();
        instance
    }

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
            entry_min_input: String::new(),
            entry_max_input: String::new(),
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

    /// Keep a known mark through partial pushes without renewing its freshness.
    /// Switching to a mid price or an invalid context would change the valuation
    /// basis between otherwise identical position rows.
    pub(crate) fn update_asset_context(&mut self, ctx: AssetContext, now_ms: u64) {
        let has_mark = ctx
            .mark_px
            .as_deref()
            .and_then(parse_positive_finite_number)
            .is_some();
        if !has_mark {
            let had_mark = self
                .asset_ctx
                .as_ref()
                .and_then(|previous| previous.mark_px.as_deref())
                .and_then(parse_positive_finite_number)
                .is_some();
            let has_mid = ctx
                .mid_px
                .as_deref()
                .and_then(parse_positive_finite_number)
                .is_some();
            if had_mark || !has_mid {
                return;
            }
        }

        self.asset_ctx = Some(ctx);
        self.asset_ctx_updated_at_ms = Some(now_ms);
    }

    pub(crate) fn has_active_filters(&self) -> bool {
        self.side != PositioningInfoSide::default()
            || self.sort_field != PositioningInfoSortField::default()
            || self.sort_direction != default_positioning_sort_direction()
            || !self.entry_min_input.trim().is_empty()
            || !self.entry_max_input.trim().is_empty()
    }

    pub(crate) fn reset_filters(&mut self) {
        self.side = PositioningInfoSide::default();
        self.sort_field = PositioningInfoSortField::default();
        self.sort_direction = default_positioning_sort_direction();
        self.entry_min_input.clear();
        self.entry_max_input.clear();
    }

    pub(crate) fn normalize_removed_filters(&mut self) {
        if self.sort_field == PositioningInfoSortField::CopyScore {
            self.sort_field = PositioningInfoSortField::default();
            self.sort_direction = default_positioning_sort_direction();
        }
    }
}
