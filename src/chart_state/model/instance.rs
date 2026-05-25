use super::{ChartId, ChartInstance};
use crate::chart::CandlestickChart;
use crate::config;
use crate::timeframe::Timeframe;

// ---------------------------------------------------------------------------
// Chart Instance Construction
// ---------------------------------------------------------------------------

impl ChartInstance {
    pub(crate) fn new(id: ChartId, symbol: String, interval: Timeframe) -> Self {
        let display = symbol.split(':').nth(1).unwrap_or(&symbol).to_string();
        let mut chart = CandlestickChart::new(id);
        chart.set_timeframe(interval);
        Self {
            id,
            symbol,
            symbol_display: display,
            interval,
            chart,
            asset_ctx: None,
            editor_open: false,
            editor_search_query: String::new(),
            editor_selected_index: None,
            quick_order: None,
            last_quick_order_is_limit: true,
            last_quick_order_symbol: String::new(),
            last_quick_order_quantity: String::new(),
            last_quick_order_quantity_is_usd: false,
            last_quick_order_percentage: 0.0,
            annotations: Vec::new(),
            next_annotation_id: 0,
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
            last_price_flash: None,
            funding_fetch_request: None,
            funding_last_attempt_ms: None,
            macro_indicators: config::MacroIndicatorsConfig::default(),
            macro_menu_open: false,
            open_interest_as_notional: false,
            outcome_volume_as_notional: false,
        }
    }

    pub(crate) fn clone_for_detached_window(&self, id: ChartId) -> Self {
        Self {
            id,
            symbol: self.symbol.clone(),
            symbol_display: self.symbol_display.clone(),
            interval: self.interval,
            chart: self.chart.clone_for_chart_id(id),
            asset_ctx: self.asset_ctx.clone(),
            editor_open: false,
            editor_search_query: String::new(),
            editor_selected_index: None,
            quick_order: None,
            last_quick_order_is_limit: self.last_quick_order_is_limit,
            last_quick_order_symbol: self.last_quick_order_symbol.clone(),
            last_quick_order_quantity: self.last_quick_order_quantity.clone(),
            last_quick_order_quantity_is_usd: self.last_quick_order_quantity_is_usd,
            last_quick_order_percentage: self.last_quick_order_percentage,
            annotations: self.annotations.clone(),
            next_annotation_id: self.next_annotation_id,
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
            last_price_flash: None,
            funding_fetch_request: None,
            funding_last_attempt_ms: self.funding_last_attempt_ms,
            macro_indicators: self.macro_indicators.clone(),
            macro_menu_open: false,
            open_interest_as_notional: self.open_interest_as_notional,
            outcome_volume_as_notional: self.outcome_volume_as_notional,
        }
    }

    /// Create a new chart with the editor open and no symbol selected.
    pub(crate) fn new_empty(id: ChartId) -> Self {
        Self {
            id,
            symbol: String::new(),
            symbol_display: String::new(),
            interval: Timeframe::H1,
            chart: CandlestickChart::new(id),
            asset_ctx: None,
            editor_open: true,
            editor_search_query: String::new(),
            editor_selected_index: None,
            quick_order: None,
            last_quick_order_is_limit: true,
            last_quick_order_symbol: String::new(),
            last_quick_order_quantity: String::new(),
            last_quick_order_quantity_is_usd: false,
            last_quick_order_percentage: 0.0,
            annotations: Vec::new(),
            next_annotation_id: 0,
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
            last_price_flash: None,
            funding_fetch_request: None,
            funding_last_attempt_ms: None,
            macro_indicators: config::MacroIndicatorsConfig::default(),
            macro_menu_open: false,
            open_interest_as_notional: false,
            outcome_volume_as_notional: false,
        }
    }
}
