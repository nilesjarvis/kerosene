use super::{
    AggregatedTrade, JournalNote, JournalTradeDetails, JournalTradeSnapshot,
    JournalTradeSnapshotRequest,
};
use crate::portfolio_state::PortfolioWindow;
use std::collections::{HashMap, HashSet};

mod account_scope;

pub(crate) const DEFAULT_JOURNAL_WINDOW_WIDTH: f32 = 800.0;
pub(crate) const DEFAULT_JOURNAL_WINDOW_HEIGHT: f32 = 600.0;
pub(crate) const JOURNAL_CHART_REVEAL_DURATION_MS: u64 = 850;

// ---------------------------------------------------------------------------
// Filters
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalFilter {
    All,
    Perp,
    Spot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalSort {
    TimeDesc,
    TimeAsc,
    PnlDesc,
    PnlAsc,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct JournalState {
    pub window_id: Option<iced::window::Id>,
    pub open: bool,
    pub width: f32,
    pub height: f32,
    pub chart_reveal_started_ms: Option<u64>,
    pub chart_reveal_progress: f32,
    pub active_account_key: Option<String>,
    pub account_states: HashMap<String, JournalAccountState>,
    pub loaded_address: Option<String>,
    pub entries: HashMap<String, JournalNote>,
    pub raw_fills: Vec<crate::api::UserFill>,
    pub trades: Vec<AggregatedTrade>,
    pub trade_details: HashMap<String, JournalTradeDetails>,
    pub expanded_snapshot_trade_ids: HashSet<String>,
    pub snapshot_requests: HashMap<String, JournalTradeSnapshotRequest>,
    pub snapshots: HashMap<String, JournalTradeSnapshot>,
    pub loading: bool,
    pub filter: JournalFilter,
    pub sort: JournalSort,
    pub show_all_assets: bool,
    pub show_account_value_chart: bool,
    pub include_fees_in_pnl: bool,
    pub portfolio_window: PortfolioWindow,
    pub error: Option<String>,
    pub warning: Option<String>,
    pub last_refresh_time: Option<u64>,
    pub edit_modes: HashMap<String, bool>,
    pub edit_source_keys: HashMap<String, String>,
    pub edit_buffers: HashMap<String, JournalNote>,
}

impl JournalState {
    pub fn begin_chart_reveal(&mut self, now_ms: u64) {
        self.chart_reveal_started_ms = Some(now_ms);
        self.chart_reveal_progress = 0.0;
    }

    pub fn finish_chart_reveal(&mut self) {
        self.chart_reveal_started_ms = None;
        self.chart_reveal_progress = 1.0;
    }

    pub fn chart_reveal_active(&self) -> bool {
        self.window_id.is_some() && self.chart_reveal_progress < 1.0
    }

    pub fn advance_chart_reveal(&mut self, now_ms: u64) {
        let Some(started_ms) = self.chart_reveal_started_ms else {
            self.finish_chart_reveal();
            return;
        };

        let elapsed_ms = now_ms.saturating_sub(started_ms);
        self.chart_reveal_progress =
            (elapsed_ms as f32 / JOURNAL_CHART_REVEAL_DURATION_MS as f32).clamp(0.0, 1.0);

        if self.chart_reveal_progress >= 1.0 {
            self.chart_reveal_started_ms = None;
        }
    }
}

#[derive(Debug, Clone)]
pub struct JournalAccountState {
    pub loaded_address: Option<String>,
    pub entries: HashMap<String, JournalNote>,
    pub raw_fills: Vec<crate::api::UserFill>,
    pub trades: Vec<AggregatedTrade>,
    pub trade_details: HashMap<String, JournalTradeDetails>,
    pub expanded_snapshot_trade_ids: HashSet<String>,
    pub snapshot_requests: HashMap<String, JournalTradeSnapshotRequest>,
    pub snapshots: HashMap<String, JournalTradeSnapshot>,
    pub loading: bool,
    pub error: Option<String>,
    pub warning: Option<String>,
    pub last_refresh_time: Option<u64>,
    pub edit_modes: HashMap<String, bool>,
    pub edit_source_keys: HashMap<String, String>,
    pub edit_buffers: HashMap<String, JournalNote>,
    pub show_account_value_chart: bool,
    pub include_fees_in_pnl: bool,
    pub portfolio_window: PortfolioWindow,
}

impl Default for JournalAccountState {
    fn default() -> Self {
        Self {
            loaded_address: None,
            entries: HashMap::new(),
            raw_fills: Vec::new(),
            trades: Vec::new(),
            trade_details: HashMap::new(),
            expanded_snapshot_trade_ids: HashSet::new(),
            snapshot_requests: HashMap::new(),
            snapshots: HashMap::new(),
            loading: false,
            error: None,
            warning: None,
            last_refresh_time: None,
            edit_modes: HashMap::new(),
            edit_source_keys: HashMap::new(),
            edit_buffers: HashMap::new(),
            show_account_value_chart: false,
            include_fees_in_pnl: true,
            portfolio_window: PortfolioWindow::Week,
        }
    }
}
