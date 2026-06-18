use super::{ChartId, ChartInstance};
use crate::account::AssetContext;
use crate::chart::CandlestickChart;
use crate::config;
use crate::market_state::MARKET_ASSET_CONTEXT_MAX_AGE_MS;
use crate::timeframe::Timeframe;

// ---------------------------------------------------------------------------
// Chart Instance Construction
// ---------------------------------------------------------------------------

impl ChartInstance {
    pub(crate) fn new(id: ChartId, symbol: String, interval: Timeframe) -> Self {
        let display = symbol.split(':').nth(1).unwrap_or(&symbol).to_string();
        let mut chart = CandlestickChart::new(id);
        chart.set_symbol_key(symbol.clone());
        chart.set_timeframe(interval);
        chart.set_symbol_label(display.clone());
        Self {
            id,
            symbol,
            symbol_display: display,
            secondary_symbol: None,
            secondary_symbol_display: None,
            interval,
            chart,
            asset_ctx: None,
            asset_ctx_updated_at_ms: None,
            asset_ctx_from_rest: false,
            asset_ctx_rest_in_flight: false,
            editor_open: false,
            header_collapsed: false,
            drawing_toolbar_collapsed: false,
            editor_search_query: String::new(),
            editor_selected_index: None,
            secondary_editor_open: false,
            secondary_editor_search_query: String::new(),
            secondary_editor_selected_index: None,
            quick_order: None,
            last_quick_order_is_limit: true,
            last_quick_order_symbol: String::new(),
            last_quick_order_quantity: String::new(),
            last_quick_order_quantity_is_usd: false,
            last_quick_order_percentage: 0.0,
            annotations: Vec::new(),
            next_annotation_id: 0,
            selected_annotation: None,
            show_liquidations: false,
            liquidation_data: None,
            liquidation_fetching: false,
            liquidation_status: None,
            liquidation_pending_key: None,
            show_heatmap: false,
            heatmap_data: None,
            heatmap_last_fetch: None,
            heatmap_viewport: None,
            heatmap_status: None,
            heatmap_fetching: false,
            candle_fetch_request: None,
            candle_fetch_error: None,
            secondary_candle_fetch_request: None,
            secondary_candle_fetch_error: None,
            last_price_flash: None,
            show_earnings_markers: false,
            earnings_events: None,
            earnings_fetching: false,
            earnings_status: None,
            earnings_pending_ticker: None,
            funding_fetch_request: None,
            funding_last_attempt_ms: None,
            macro_candles_request_id: 0,
            macro_indicators: config::MacroIndicatorsConfig::default(),
            macro_menu_open: false,
            open_interest_as_notional: false,
            asset_volume_as_notional: true,
            outcome_volume_as_notional: false,
        }
    }

    pub(crate) fn set_symbol_identity(&mut self, symbol: String, display: String) -> bool {
        let changed = self.symbol != symbol || self.symbol_display != display;
        self.symbol = symbol.clone();
        self.symbol_display = display.clone();
        self.chart.set_symbol_key(symbol);
        self.chart.set_symbol_label(display);
        changed
    }

    pub(crate) fn set_secondary_symbol_identity(
        &mut self,
        symbol: String,
        display: String,
    ) -> bool {
        let changed = self.secondary_symbol.as_deref() != Some(symbol.as_str())
            || self.secondary_symbol_display.as_deref() != Some(display.as_str());
        self.secondary_symbol = Some(symbol.clone());
        self.secondary_symbol_display = Some(display.clone());
        self.chart.set_secondary_series_identity(symbol, display);
        changed
    }

    pub(crate) fn clear_secondary_symbol(&mut self) {
        self.secondary_symbol = None;
        self.secondary_symbol_display = None;
        self.secondary_editor_open = false;
        self.secondary_editor_search_query.clear();
        self.secondary_editor_selected_index = None;
        self.secondary_candle_fetch_request = None;
        self.secondary_candle_fetch_error = None;
        self.chart.clear_secondary_series();
    }

    pub(crate) fn clone_for_detached_window(&self, id: ChartId) -> Self {
        Self {
            id,
            symbol: self.symbol.clone(),
            symbol_display: self.symbol_display.clone(),
            secondary_symbol: self.secondary_symbol.clone(),
            secondary_symbol_display: self.secondary_symbol_display.clone(),
            interval: self.interval,
            chart: self.chart.clone_for_chart_id(id),
            asset_ctx: self.asset_ctx.clone(),
            asset_ctx_updated_at_ms: self.asset_ctx_updated_at_ms,
            asset_ctx_from_rest: self.asset_ctx_from_rest,
            asset_ctx_rest_in_flight: false,
            editor_open: false,
            header_collapsed: self.header_collapsed,
            drawing_toolbar_collapsed: self.drawing_toolbar_collapsed,
            editor_search_query: String::new(),
            editor_selected_index: None,
            secondary_editor_open: false,
            secondary_editor_search_query: String::new(),
            secondary_editor_selected_index: None,
            quick_order: None,
            last_quick_order_is_limit: self.last_quick_order_is_limit,
            last_quick_order_symbol: self.last_quick_order_symbol.clone(),
            last_quick_order_quantity: self.last_quick_order_quantity.clone(),
            last_quick_order_quantity_is_usd: self.last_quick_order_quantity_is_usd,
            last_quick_order_percentage: self.last_quick_order_percentage,
            annotations: self.annotations.clone(),
            next_annotation_id: self.next_annotation_id,
            selected_annotation: self.selected_annotation,
            show_liquidations: self.show_liquidations,
            liquidation_data: self.liquidation_data.clone(),
            liquidation_fetching: false,
            liquidation_status: self.liquidation_status.clone(),
            liquidation_pending_key: None,
            show_heatmap: self.show_heatmap,
            heatmap_data: self.heatmap_data.clone(),
            heatmap_last_fetch: self.heatmap_last_fetch.clone(),
            heatmap_viewport: self.heatmap_viewport,
            heatmap_status: self.heatmap_status.clone(),
            heatmap_fetching: false,
            candle_fetch_request: None,
            candle_fetch_error: self.candle_fetch_error.clone(),
            secondary_candle_fetch_request: None,
            secondary_candle_fetch_error: self.secondary_candle_fetch_error.clone(),
            last_price_flash: None,
            show_earnings_markers: self.show_earnings_markers,
            earnings_events: self.earnings_events.clone(),
            earnings_fetching: false,
            earnings_status: self.earnings_status.clone(),
            earnings_pending_ticker: None,
            funding_fetch_request: None,
            funding_last_attempt_ms: self.funding_last_attempt_ms,
            macro_candles_request_id: 0,
            macro_indicators: self.macro_indicators.clone(),
            macro_menu_open: false,
            open_interest_as_notional: self.open_interest_as_notional,
            asset_volume_as_notional: self.asset_volume_as_notional,
            outcome_volume_as_notional: self.outcome_volume_as_notional,
        }
    }

    pub(crate) fn set_asset_context(&mut self, asset_ctx: Option<AssetContext>) {
        self.set_asset_context_at(asset_ctx, crate::app_time::now_ms());
    }

    pub(crate) fn set_asset_context_at(&mut self, asset_ctx: Option<AssetContext>, now_ms: u64) {
        match asset_ctx.as_ref() {
            Some(ctx) => self
                .chart
                .set_current_spread_at(ctx.impact_spread(), now_ms),
            None => self.chart.clear_spread_history(),
        }
        self.asset_ctx_updated_at_ms = asset_ctx.as_ref().map(|_| now_ms);
        self.asset_ctx = asset_ctx;
        // This setter is the live `activeAssetCtx` WebSocket / clear path; any
        // prior REST provenance no longer applies.
        self.asset_ctx_from_rest = false;
    }

    /// Apply an `AssetContext` fetched from the REST `metaAndAssetCtxs`
    /// fallback. Marks the context as REST-sourced so a later live WebSocket
    /// push (via [`set_asset_context_at`]) can take over without being treated
    /// as stale, and so the poller knows to refresh it on a timer.
    pub(crate) fn fill_asset_context_from_rest(&mut self, ctx: AssetContext, now_ms: u64) {
        self.chart
            .set_current_spread_at(ctx.impact_spread(), now_ms);
        self.asset_ctx_updated_at_ms = Some(now_ms);
        self.asset_ctx = Some(ctx);
        self.asset_ctx_from_rest = true;
    }

    /// Whether this chart should issue a REST asset-context fetch now.
    ///
    /// Fires when there is no context at all, or when the existing context is
    /// REST-sourced and is approaching the staleness expiry (so it is refreshed
    /// before [`expire_asset_context_if_stale`] would blank the header). Live
    /// WebSocket-sourced context is left untouched. `refresh_ms` must be below
    /// `MARKET_ASSET_CONTEXT_MAX_AGE_MS` to avoid a flicker between expiry and
    /// the refreshed fetch landing.
    pub(crate) fn needs_rest_asset_context(&self, now_ms: u64, refresh_ms: u64) -> bool {
        if self.asset_ctx_rest_in_flight || self.symbol.is_empty() {
            return false;
        }
        match self.asset_ctx {
            None => true,
            Some(_) => {
                self.asset_ctx_from_rest
                    && self.asset_ctx_updated_at_ms.is_some_and(|updated_at_ms| {
                        now_ms.saturating_sub(updated_at_ms) >= refresh_ms
                    })
            }
        }
    }

    pub(crate) fn expire_asset_context_if_stale(&mut self, now_ms: u64) -> bool {
        let Some(updated_at_ms) = self.asset_ctx_updated_at_ms else {
            return false;
        };
        if self.asset_ctx.is_none() {
            self.asset_ctx_updated_at_ms = None;
            return false;
        }
        if now_ms
            .checked_sub(updated_at_ms)
            .is_some_and(|age_ms| age_ms > MARKET_ASSET_CONTEXT_MAX_AGE_MS)
        {
            self.set_asset_context(None);
            return true;
        }
        false
    }

    pub(crate) fn next_macro_candles_request_id(&mut self) -> u64 {
        self.macro_candles_request_id = self.macro_candles_request_id.saturating_add(1);
        self.macro_candles_request_id
    }

    /// Create a new chart with the editor open and no symbol selected.
    pub(crate) fn new_empty(id: ChartId) -> Self {
        Self {
            id,
            symbol: String::new(),
            symbol_display: String::new(),
            secondary_symbol: None,
            secondary_symbol_display: None,
            interval: Timeframe::H1,
            chart: CandlestickChart::new(id),
            asset_ctx: None,
            asset_ctx_updated_at_ms: None,
            asset_ctx_from_rest: false,
            asset_ctx_rest_in_flight: false,
            editor_open: true,
            header_collapsed: false,
            drawing_toolbar_collapsed: false,
            editor_search_query: String::new(),
            editor_selected_index: None,
            secondary_editor_open: false,
            secondary_editor_search_query: String::new(),
            secondary_editor_selected_index: None,
            quick_order: None,
            last_quick_order_is_limit: true,
            last_quick_order_symbol: String::new(),
            last_quick_order_quantity: String::new(),
            last_quick_order_quantity_is_usd: false,
            last_quick_order_percentage: 0.0,
            annotations: Vec::new(),
            next_annotation_id: 0,
            selected_annotation: None,
            show_liquidations: false,
            liquidation_data: None,
            liquidation_fetching: false,
            liquidation_status: None,
            liquidation_pending_key: None,
            show_heatmap: false,
            heatmap_data: None,
            heatmap_last_fetch: None,
            heatmap_viewport: None,
            heatmap_status: None,
            heatmap_fetching: false,
            candle_fetch_request: None,
            candle_fetch_error: None,
            secondary_candle_fetch_request: None,
            secondary_candle_fetch_error: None,
            last_price_flash: None,
            show_earnings_markers: false,
            earnings_events: None,
            earnings_fetching: false,
            earnings_status: None,
            earnings_pending_ticker: None,
            funding_fetch_request: None,
            funding_last_attempt_ms: None,
            macro_candles_request_id: 0,
            macro_indicators: config::MacroIndicatorsConfig::default(),
            macro_menu_open: false,
            open_interest_as_notional: false,
            asset_volume_as_notional: true,
            outcome_volume_as_notional: false,
        }
    }
}
