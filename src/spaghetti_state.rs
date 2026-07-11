use crate::app_state::TradingTerminal;
use crate::timeframe::Timeframe;

use crate::spaghetti;

const SPAGHETTI_MAX_SESSION_CANDLES: u64 = 10_000;

pub(crate) type SpaghettiChartId = u64;

pub(crate) struct SpaghettiChartInstance {
    pub(crate) id: SpaghettiChartId,
    pub(crate) canvas: spaghetti::SpaghettiCanvas,
    pub(crate) interval: Timeframe,
    pub(crate) pair_mode: bool,
    pub(crate) pair_candle_mode: bool,
    pub(crate) session_granularity: Option<Timeframe>,
    pub(crate) style_menu_open: bool,
    pub(crate) editor_open: bool,
    pub(crate) editor_search_query: String,
    /// Monotonic counter for color assignment (never resets on removal).
    pub(crate) next_color_idx: usize,
    next_candle_request_id: u64,
    pending_candle_request_ids: std::collections::HashMap<String, u64>,
}

impl SpaghettiChartInstance {
    pub(crate) fn new_empty(id: SpaghettiChartId) -> Self {
        Self {
            id,
            canvas: spaghetti::SpaghettiCanvas::new(),
            interval: Timeframe::H1,
            pair_mode: false,
            pair_candle_mode: false,
            session_granularity: None,
            style_menu_open: false,
            editor_open: true,
            editor_search_query: String::new(),
            next_color_idx: 0,
            next_candle_request_id: 0,
            pending_candle_request_ids: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn new_pair(id: SpaghettiChartId) -> Self {
        let mut inst = Self::new_empty(id);
        inst.pair_mode = true;
        inst.canvas.pair_ratio_mode = true;
        inst.pair_candle_mode = true;
        inst.canvas.pair_candle_mode = true;
        inst.interval = Timeframe::M5;
        inst.editor_open = true;
        inst
    }

    /// Install a new owner for one series. The sequence survives series
    /// removal/re-addition within this chart instance.
    pub(crate) fn begin_spaghetti_candle_request(&mut self, symbol: &str) -> Option<u64> {
        if !self
            .canvas
            .series
            .iter()
            .any(|series| series.symbol == symbol)
        {
            return None;
        }
        self.next_candle_request_id = self.next_candle_request_id.wrapping_add(1);
        let request_id = self.next_candle_request_id;
        self.pending_candle_request_ids
            .insert(symbol.to_string(), request_id);
        Some(request_id)
    }

    pub(crate) fn pending_spaghetti_candle_request_id(&self, symbol: &str) -> Option<u64> {
        self.pending_candle_request_ids.get(symbol).copied()
    }

    pub(crate) fn finish_spaghetti_candle_request(
        &mut self,
        symbol: &str,
        request_id: u64,
    ) -> bool {
        if self.pending_spaghetti_candle_request_id(symbol) != Some(request_id) {
            return false;
        }
        self.pending_candle_request_ids.remove(symbol);
        true
    }

    pub(crate) fn forget_spaghetti_candle_request(&mut self, symbol: &str) {
        self.pending_candle_request_ids.remove(symbol);
    }

    pub(crate) fn clear_spaghetti_candle_requests(&mut self) {
        self.pending_candle_request_ids.clear();
    }

    pub(crate) fn retain_spaghetti_candle_requests_for_current_series(&mut self) {
        let symbols = self
            .canvas
            .series
            .iter()
            .map(|series| series.symbol.clone())
            .collect::<std::collections::HashSet<_>>();
        self.pending_candle_request_ids
            .retain(|symbol, _| symbols.contains(symbol));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpaghettiCandleFetch {
    pub(crate) chart_id: SpaghettiChartId,
    pub(crate) chart_instance_generation: u64,
    pub(crate) request_id: u64,
    pub(crate) symbol: String,
    pub(crate) timeframe: Timeframe,
    pub(crate) source: crate::config::ChartBackfillSource,
    pub(crate) read_data_provider_generation: u64,
    pub(crate) hydromancer_key_generation: u64,
    pub(crate) session: Option<spaghetti::Session>,
    pub(crate) session_granularity: Option<Timeframe>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpaghettiWsCandleContext {
    pub(crate) chart_id: SpaghettiChartId,
    pub(crate) symbol: String,
    pub(crate) timeframe: Timeframe,
    pub(crate) source_context: crate::read_data_provider::MarketDataSourceContext,
    pub(crate) session: Option<spaghetti::Session>,
    pub(crate) session_granularity: Option<Timeframe>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnchorGranularityOption {
    Auto,
    Manual(Timeframe),
}

impl std::fmt::Display for AnchorGranularityOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "Auto"),
            Self::Manual(tf) => write!(f, "{}", tf.label()),
        }
    }
}

impl TradingTerminal {
    pub(crate) fn spaghetti_fetch_plan(
        tf: Timeframe,
        session: Option<spaghetti::Session>,
        session_granularity: Option<Timeframe>,
        now_ms: u64,
    ) -> (Timeframe, u64) {
        if let Some(session) = session {
            let (granularity, start) =
                Self::spaghetti_session_fetch_granularity(session, session_granularity, now_ms);
            (granularity, start)
        } else {
            (tf, now_ms.saturating_sub(tf.lookback_ms()))
        }
    }

    pub(crate) fn spaghetti_auto_session_granularity(span_ms: u64) -> Timeframe {
        if span_ms <= 12 * 60 * 60 * 1000 {
            Timeframe::M1
        } else if span_ms <= 3 * 24 * 60 * 60 * 1000 {
            Timeframe::M5
        } else if span_ms <= 14 * 24 * 60 * 60 * 1000 {
            Timeframe::M15
        } else if span_ms <= 90 * 24 * 60 * 60 * 1000 {
            Timeframe::H1
        } else if span_ms <= 365 * 24 * 60 * 60 * 1000 {
            Timeframe::H4
        } else {
            Timeframe::D1
        }
    }

    pub(crate) fn spaghetti_session_granularity_fits(span_ms: u64, tf: Timeframe) -> bool {
        let candle_count =
            span_ms.saturating_add(tf.duration_ms().saturating_sub(1)) / tf.duration_ms().max(1);
        candle_count <= SPAGHETTI_MAX_SESSION_CANDLES
    }

    pub(crate) fn spaghetti_session_fetch_granularity(
        session: spaghetti::Session,
        session_granularity: Option<Timeframe>,
        now_ms: u64,
    ) -> (Timeframe, u64) {
        let start = session.last_open_ms(now_ms);
        let span = now_ms.saturating_sub(start);
        let auto = Self::spaghetti_auto_session_granularity(span);
        let granularity = session_granularity
            .filter(|tf| Self::spaghetti_session_granularity_fits(span, *tf))
            .unwrap_or(auto);
        (granularity, start)
    }

    pub(crate) fn spaghetti_effective_timeframe_for(
        interval: Timeframe,
        session: Option<spaghetti::Session>,
        session_granularity: Option<Timeframe>,
        now_ms: u64,
    ) -> Timeframe {
        if let Some(session) = session {
            Self::spaghetti_session_fetch_granularity(session, session_granularity, now_ms).0
        } else {
            interval
        }
    }

    pub(crate) fn normalize_spaghetti_session_granularity(
        inst: &mut SpaghettiChartInstance,
        now_ms: u64,
    ) {
        let Some(session) = inst.canvas.active_session else {
            return;
        };
        let start = session.last_open_ms(now_ms);
        let span = now_ms.saturating_sub(start);
        if let Some(tf) = inst.session_granularity
            && !Self::spaghetti_session_granularity_fits(span, tf)
        {
            inst.session_granularity = None;
        }
    }

    pub(crate) fn refresh_spaghetti_session_anchor(inst: &mut SpaghettiChartInstance) {
        let Some(session) = inst.canvas.active_session else {
            inst.canvas.base_timestamp = None;
            return;
        };

        let latest_candle_ts = inst
            .canvas
            .series
            .iter()
            .filter_map(|s| s.candles.last().map(|c| c.open_time))
            .max()
            .unwrap_or_else(Self::now_ms);

        inst.canvas.base_timestamp = Some(session.last_open_ms(latest_candle_ts));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Color;

    #[test]
    fn spaghetti_request_owner_wraps_and_replaces_same_series_owner() {
        let mut instance = SpaghettiChartInstance::new_empty(7);
        instance.canvas.series.push(spaghetti::Series {
            symbol: "BTC".to_string(),
            display: "BTC".to_string(),
            candles: Vec::new(),
            color: Color::BLACK,
            loaded: false,
        });
        instance.next_candle_request_id = u64::MAX;

        assert_eq!(instance.begin_spaghetti_candle_request("BTC"), Some(0));
        assert_eq!(instance.pending_spaghetti_candle_request_id("BTC"), Some(0));
        assert_eq!(instance.begin_spaghetti_candle_request("BTC"), Some(1));
        assert_eq!(instance.pending_spaghetti_candle_request_id("BTC"), Some(1));
        assert!(!instance.finish_spaghetti_candle_request("BTC", 0));
        assert_eq!(instance.pending_spaghetti_candle_request_id("BTC"), Some(1));
    }
}
