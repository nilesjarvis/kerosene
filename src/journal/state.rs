use super::{
    AggregatedTrade, JournalNote, JournalTradeDetails, JournalTradeSnapshot,
    JournalTradeSnapshotRequest,
};
use std::collections::{HashMap, HashSet};

mod account_scope;

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
    pub error: Option<String>,
    pub warning: Option<String>,
    pub last_refresh_time: Option<u64>,
    pub edit_modes: HashMap<String, bool>,
    pub edit_source_keys: HashMap<String, String>,
    pub edit_buffers: HashMap<String, JournalNote>,
}

#[derive(Debug, Clone, Default)]
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
}
