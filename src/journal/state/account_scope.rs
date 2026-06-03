use super::{JournalAccountState, JournalFilter, JournalSort, JournalState};
use crate::journal::JournalNote;
use std::collections::{HashMap, HashSet};

impl JournalState {
    pub fn new_for_account(
        active_account_key: Option<String>,
        entries_by_account: HashMap<String, HashMap<String, JournalNote>>,
        legacy_entries: HashMap<String, JournalNote>,
    ) -> Self {
        let mut account_states: HashMap<String, JournalAccountState> = entries_by_account
            .into_iter()
            .map(|(key, entries)| {
                (
                    key,
                    JournalAccountState {
                        entries,
                        ..JournalAccountState::default()
                    },
                )
            })
            .collect();

        let entries = match active_account_key.as_ref() {
            Some(key) => match account_states.get(key) {
                Some(state) => state.entries.clone(),
                None => {
                    let entries = legacy_entries;
                    if !entries.is_empty() {
                        account_states.insert(
                            key.clone(),
                            JournalAccountState {
                                entries: entries.clone(),
                                ..JournalAccountState::default()
                            },
                        );
                    }
                    entries
                }
            },
            None => legacy_entries,
        };

        Self {
            window_id: None,
            open: false,
            width: super::DEFAULT_JOURNAL_WINDOW_WIDTH,
            height: super::DEFAULT_JOURNAL_WINDOW_HEIGHT,
            chart_reveal_started_ms: None,
            chart_reveal_progress: 1.0,
            active_account_key,
            account_states,
            loaded_address: None,
            entries,
            raw_fills: Vec::new(),
            trades: Vec::new(),
            trade_details: HashMap::new(),
            expanded_snapshot_trade_ids: HashSet::new(),
            snapshot_requests: HashMap::new(),
            snapshots: HashMap::new(),
            loading: false,
            filter: JournalFilter::All,
            sort: JournalSort::TimeDesc,
            show_all_assets: false,
            show_account_value_chart: false,
            include_fees_in_pnl: true,
            portfolio_window: crate::portfolio_state::PortfolioWindow::Week,
            error: None,
            warning: None,
            last_refresh_time: None,
            edit_modes: HashMap::new(),
            edit_source_keys: HashMap::new(),
            edit_buffers: HashMap::new(),
        }
    }

    pub fn save_active_account_state(&mut self) {
        let Some(key) = self.active_account_key.clone() else {
            return;
        };
        self.account_states
            .insert(key, self.snapshot_active_account_state());
    }

    pub fn switch_active_account(&mut self, key: Option<String>) {
        if self.active_account_key == key {
            return;
        }

        self.save_active_account_state();
        self.active_account_key = key.clone();

        let state = key
            .and_then(|key| self.account_states.get(&key).cloned())
            .unwrap_or_default();
        self.restore_active_account_state(state);
    }

    pub fn entries_by_account_snapshot(&self) -> HashMap<String, HashMap<String, JournalNote>> {
        let mut entries_by_account: HashMap<String, HashMap<String, JournalNote>> = self
            .account_states
            .iter()
            .filter(|(_key, state)| !state.entries.is_empty())
            .map(|(key, state)| (key.clone(), state.entries.clone()))
            .collect();

        if let Some(key) = &self.active_account_key {
            if self.entries.is_empty() {
                entries_by_account.remove(key);
            } else {
                entries_by_account.insert(key.clone(), self.entries.clone());
            }
        }

        entries_by_account
    }

    pub fn clear_active_account_data_for_address(&mut self, address: String) {
        self.loaded_address = Some(address);
        self.raw_fills.clear();
        self.trades.clear();
        self.clear_snapshot_data();
        self.loading = false;
        self.error = None;
        self.warning = None;
        self.last_refresh_time = None;
        self.edit_modes.clear();
        self.edit_source_keys.clear();
        self.edit_buffers.clear();
    }

    pub fn clear_active_account_data(&mut self) {
        self.loaded_address = None;
        self.raw_fills.clear();
        self.trades.clear();
        self.clear_snapshot_data();
        self.loading = false;
        self.error = None;
        self.warning = None;
        self.last_refresh_time = None;
        self.edit_modes.clear();
        self.edit_source_keys.clear();
        self.edit_buffers.clear();
    }

    fn snapshot_active_account_state(&self) -> JournalAccountState {
        JournalAccountState {
            loaded_address: self.loaded_address.clone(),
            entries: self.entries.clone(),
            raw_fills: self.raw_fills.clone(),
            trades: self.trades.clone(),
            trade_details: self.trade_details.clone(),
            expanded_snapshot_trade_ids: self.expanded_snapshot_trade_ids.clone(),
            snapshot_requests: self.snapshot_requests.clone(),
            snapshots: self.snapshots.clone(),
            loading: false,
            error: self.error.clone(),
            warning: self.warning.clone(),
            last_refresh_time: self.last_refresh_time,
            edit_modes: self.edit_modes.clone(),
            edit_source_keys: self.edit_source_keys.clone(),
            edit_buffers: self.edit_buffers.clone(),
            show_account_value_chart: self.show_account_value_chart,
            include_fees_in_pnl: self.include_fees_in_pnl,
            portfolio_window: self.portfolio_window,
        }
    }

    fn restore_active_account_state(&mut self, state: JournalAccountState) {
        self.loaded_address = state.loaded_address;
        self.entries = state.entries;
        self.raw_fills = state.raw_fills;
        self.trades = state.trades;
        self.trade_details = state.trade_details;
        self.expanded_snapshot_trade_ids = state.expanded_snapshot_trade_ids;
        self.snapshot_requests = state.snapshot_requests;
        self.snapshots = state.snapshots;
        self.loading = state.loading;
        self.error = state.error;
        self.warning = state.warning;
        self.last_refresh_time = state.last_refresh_time;
        self.edit_modes = state.edit_modes;
        self.edit_source_keys = state.edit_source_keys;
        self.edit_buffers = state.edit_buffers;
        self.show_account_value_chart = state.show_account_value_chart;
        self.include_fees_in_pnl = state.include_fees_in_pnl;
        self.portfolio_window = state.portfolio_window;
    }

    pub fn clear_snapshot_data(&mut self) {
        self.trade_details.clear();
        self.expanded_snapshot_trade_ids.clear();
        self.snapshot_requests.clear();
        self.snapshots.clear();
    }

    pub fn clear_snapshot_cache(&mut self) {
        self.snapshot_requests.clear();
        self.snapshots.clear();
    }
}
