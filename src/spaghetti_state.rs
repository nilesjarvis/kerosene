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
    pub(crate) editor_open: bool,
    pub(crate) editor_search_query: String,
    /// Monotonic counter for color assignment (never resets on removal).
    pub(crate) next_color_idx: usize,
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
            editor_open: true,
            editor_search_query: String::new(),
            next_color_idx: 0,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpaghettiCandleFetch {
    pub(crate) chart_id: SpaghettiChartId,
    pub(crate) symbol: String,
    pub(crate) timeframe: Timeframe,
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
