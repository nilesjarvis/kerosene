use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use crate::{api, journal};
use iced::Task;

impl TradingTerminal {
    pub(crate) fn reconcile_journal_current_positions_from_account(
        &mut self,
    ) -> journal::JournalPositionReconciliation {
        let Some((connected_address, positions, fetched_at_ms)) = self
            .connected_order_account_snapshot()
            .map(|(address, data)| {
                (
                    address,
                    data.clearinghouse.asset_positions.clone(),
                    data.fetched_at_ms,
                )
            })
        else {
            return journal::JournalPositionReconciliation::default();
        };
        if self.journal.loaded_address.as_deref() != Some(connected_address.as_str()) {
            return journal::JournalPositionReconciliation::default();
        }

        let result = journal::reconcile_current_position_trades(
            &mut self.journal.trades,
            &positions,
            fetched_at_ms,
        );
        if result.added_open_positions > 0 || result.removed_stale_positions > 0 {
            self.journal.clear_snapshot_results();
            self.journal.expanded_snapshot_trade_ids.clear();
        }
        result
    }

    pub(crate) fn push_journal_warning_message(&mut self, warning: String) {
        match &mut self.journal.warning {
            Some(existing) if existing.contains(&warning) => {}
            Some(existing) => {
                existing.push(' ');
                existing.push_str(&warning);
            }
            None => self.journal.warning = Some(warning),
        }
    }

    pub(crate) fn update_journal(&mut self, message: Message) -> Task<Message> {
        match message {
            // ----- Trading Journal messages -----
            Message::JournalFillsLoaded {
                request_id,
                account_key,
                address,
                result,
            } => {
                let account_key = account_key.into_option();
                let address = address.into_string();
                if self.journal.sync_request_id != request_id
                    || self.journal.active_account_key != account_key
                    || self.connected_address.as_deref() != Some(address.as_str())
                {
                    return Task::none();
                }

                let had_chart_history = self.journal.trades.len() >= 2;

                match result {
                    Ok(page) => {
                        self.journal.loaded_address = Some(address.clone());
                        let fetched_count = page.fills.len();
                        let next_request = page.next_request;
                        let requested_end_time = page.requested_end_time;
                        let added = journal::merge_fills(&mut self.journal.raw_fills, page.fills);
                        self.journal.sync_status.watermark_ms = Some(requested_end_time);
                        self.journal.sync_status.next_start_ms =
                            next_request.map(|request| request.start_time);
                        self.journal.sync_status.pages_loaded =
                            self.journal.sync_status.pages_loaded.saturating_add(1);
                        self.journal.sync_status.fills_loaded = self.journal.raw_fills.len();
                        self.journal.sync_status.complete = next_request.is_none();
                        if let Some(warning) = page.progress_warning {
                            self.journal.sync_status.pagination_warning = Some(warning);
                        }

                        let mut warnings = self
                            .journal
                            .sync_status
                            .pagination_warning
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>();

                        if added > 0 || next_request.is_none() || self.journal.trades.is_empty() {
                            let aggregation = journal::aggregate_trades_with_diagnostics(
                                self.journal.raw_fills.clone(),
                            );
                            if let Some(warning) = aggregation.diagnostics.warning_message() {
                                warnings.push(warning);
                            }
                            self.journal.trades = aggregation.trades;
                            self.journal.trade_details = aggregation.trade_details;
                            let position_reconciliation =
                                self.reconcile_journal_current_positions_from_account();
                            if position_reconciliation.added_open_positions > 0 {
                                warnings.push(journal::current_position_fallback_warning(
                                    position_reconciliation.added_open_positions,
                                ));
                            }
                            // Preserve in-flight snapshot requests so a response
                            // arriving after this re-aggregation still applies.
                            self.journal.clear_snapshot_results();
                            self.journal.expanded_snapshot_trade_ids.clear();
                            // Drop selection and in-progress edit state for any
                            // trade that no longer exists after re-aggregation,
                            // so stale drafts can't resurface on a reused id.
                            let live_trade_ids: std::collections::HashSet<String> = self
                                .journal
                                .trades
                                .iter()
                                .map(|trade| trade.id.clone())
                                .collect();
                            if let Some(selected) = &self.journal.selected_trade_id
                                && !live_trade_ids.contains(selected)
                            {
                                self.journal.selected_trade_id = None;
                            }
                            self.journal
                                .edit_modes
                                .retain(|id, _| live_trade_ids.contains(id));
                            self.journal
                                .edit_source_keys
                                .retain(|id, _| live_trade_ids.contains(id));
                            self.journal
                                .edit_buffers
                                .retain(|id, _| live_trade_ids.contains(id));
                            self.journal
                                .edit_tag_raw
                                .retain(|id, _| live_trade_ids.contains(id));
                            self.journal.error = None;
                        }

                        if added > 0
                            && !self.journal_active_account_is_ghost()
                            && let Err(e) = journal::save_cache(&address, &self.journal.raw_fills)
                        {
                            let error = redact_sensitive_response_text(&e);
                            warnings.push(format!("Could not save journal cache: {error}"));
                        }

                        if fetched_count == 0 && added == 0 && self.journal.raw_fills.is_empty() {
                            warnings.push("No fills found for this account.".to_string());
                        }

                        self.journal.warning = if warnings.is_empty() {
                            None
                        } else {
                            Some(warnings.join(" "))
                        };

                        if let Some(next_request) = next_request {
                            let request_account_key = account_key.clone();
                            let request_address = address.clone();
                            return Task::perform(
                                api::fetch_user_fills(address, next_request),
                                move |result| Message::JournalFillsLoaded {
                                    request_id,
                                    account_key: request_account_key.clone().into(),
                                    address: request_address.clone().into(),
                                    result,
                                },
                            );
                        }

                        self.journal.loading = false;
                        self.journal.last_refresh_time = Some(requested_end_time);

                        if !had_chart_history && self.journal.trades.len() >= 2 {
                            self.journal.begin_chart_reveal(Self::now_ms());
                        }
                    }
                    Err(e) => {
                        let error = redact_sensitive_response_text(&e);
                        self.journal.sync_status.complete = false;
                        if self.journal.raw_fills.is_empty() {
                            self.journal.error = Some(error);
                        } else {
                            self.journal.warning = Some(format!(
                                "Journal refresh incomplete: {error}. Showing cached data."
                            ));
                        }
                        self.journal.loading = false;
                    }
                }
            }
            Message::JournalClearCache => {
                return self.clear_journal_cache_for_active_account();
            }
            Message::JournalEditStart(id, source_key) => {
                self.journal.edit_modes.insert(id.clone(), true);
                let note = source_key
                    .as_ref()
                    .and_then(|key| self.journal.entries.get(key))
                    .or_else(|| self.journal.entries.get(&id))
                    .cloned()
                    .unwrap_or_default();
                if let Some(source_key) = source_key {
                    self.journal.edit_source_keys.insert(id.clone(), source_key);
                }
                self.journal
                    .edit_tag_raw
                    .insert(id.clone(), journal::journal_tags_input(&note.tags));
                self.journal.edit_buffers.insert(id, note);
            }
            Message::JournalEditCancel(id) => {
                self.journal.edit_modes.remove(&id);
                self.journal.edit_source_keys.remove(&id);
                self.journal.edit_buffers.remove(&id);
                self.journal.edit_tag_raw.remove(&id);
            }
            Message::JournalEditSave(id) => {
                if let Some(note) = self.journal.edit_buffers.remove(&id) {
                    if let Some(source_key) = self.journal.edit_source_keys.remove(&id)
                        && source_key != id
                    {
                        self.journal.entries.remove(&source_key);
                    }
                    if note.is_empty() {
                        self.journal.entries.remove(&id);
                    } else {
                        self.journal.entries.insert(id.clone(), note);
                    }
                    self.persist_config();
                }
                self.journal.edit_modes.remove(&id);
                self.journal.edit_tag_raw.remove(&id);
            }
            Message::JournalBufferChanged(id, is_open, text) => {
                let entry = self.journal.edit_buffers.entry(id).or_default();
                if is_open {
                    entry.open = text;
                } else {
                    entry.close = text;
                }
            }
            Message::JournalCauseOfErrorChanged(id, text) => {
                self.journal
                    .edit_buffers
                    .entry(id)
                    .or_default()
                    .cause_of_error = text;
            }
            Message::JournalTagsChanged(id, raw) => {
                let tags = journal::parse_journal_tags(&raw);
                self.journal
                    .edit_buffers
                    .entry(id.clone())
                    .or_default()
                    .tags = tags;
                self.journal.edit_tag_raw.insert(id, raw);
            }
            Message::JournalSelectTrade(trade_id) => {
                return self.select_journal_trade(trade_id);
            }
            Message::JournalDeselectTrade => {
                self.journal.selected_trade_id = None;
            }
            Message::JournalSnapshotTimeframe(trade_id, timeframe) => {
                return self.request_journal_snapshot_timeframe(trade_id, timeframe);
            }
            Message::JournalFilterChanged(filter) => {
                self.journal.filter = filter;
            }
            Message::JournalSortChanged(sort) => {
                self.journal.sort = sort;
            }
            Message::JournalPortfolioWindowChanged(window) => {
                self.journal.portfolio_window = window;
            }
            Message::JournalChartRevealTick => {
                self.journal.advance_chart_reveal(Self::now_ms());
            }
            Message::JournalToggleAllAssets => {
                self.journal.show_all_assets = !self.journal.show_all_assets;
            }
            Message::JournalToggleAccountValueChart(show) => {
                self.journal.show_account_value_chart = show;
            }
            Message::JournalToggleIncludeFeesInPnl => {
                self.journal.include_fees_in_pnl = !self.journal.include_fees_in_pnl;
            }
            Message::JournalSnapshotLoaded {
                account_key,
                address,
                request,
                result,
            } => {
                return self.apply_journal_snapshot_loaded(
                    account_key.into_option(),
                    address.into_string(),
                    request.into_request(),
                    result,
                );
            }
            Message::JournalRefresh => {
                self.journal.clear_snapshot_cache();
                self.journal.expanded_snapshot_trade_ids.clear();
                return self.load_journal_for_active_account(true);
            }
            _ => {}
        }

        Task::none()
    }

    fn clear_journal_cache_for_active_account(&mut self) -> Task<Message> {
        if self.journal.loading {
            self.push_toast("Journal is already syncing".to_string(), true);
            return Task::none();
        }

        let Some(address) = self.connected_address.clone() else {
            self.journal.error =
                Some("Connect an account before clearing journal cache.".to_string());
            self.push_toast(
                "Connect an account before clearing journal cache".to_string(),
                true,
            );
            return Task::none();
        };

        let mut clear_warning = None;
        match journal::clear_cache(&address) {
            Ok(removed) => {
                let message = if removed == 0 {
                    "Journal cache already clear; reloading full history".to_string()
                } else {
                    format!("Cleared {removed} journal cache file(s); reloading full history")
                };
                self.push_toast(message, false);
            }
            Err(e) => {
                let error = redact_sensitive_response_text(&e);
                let warning =
                    format!("Could not clear journal cache: {error}. Reloading full history.");
                clear_warning = Some(warning.clone());
                self.push_toast(warning, true);
            }
        }

        self.journal
            .clear_active_account_data_for_address(address.clone());
        let task = self.load_journal_for_active_account(true);
        if let Some(warning) = clear_warning {
            self.journal.warning = Some(warning);
        }
        task
    }

    /// Select a trade for the master-detail inspector and lazily load its
    /// chart snapshot.
    fn select_journal_trade(&mut self, trade_id: String) -> Task<Message> {
        self.journal.selected_trade_id = Some(trade_id.clone());
        self.ensure_journal_snapshot(trade_id)
    }

    /// Queue a snapshot fetch for `trade_id` if one is not already loaded for
    /// the active backfill source or in flight.
    fn ensure_journal_snapshot(&mut self, trade_id: String) -> Task<Message> {
        if self
            .journal
            .snapshots
            .get(&trade_id)
            .is_some_and(|snapshot| snapshot.source == self.chart_backfill_source)
            || self.journal.snapshot_requests.contains_key(&trade_id)
        {
            return Task::none();
        }

        let Some(trade) = self
            .journal
            .trades
            .iter()
            .find(|trade| trade.id == trade_id)
            .cloned()
        else {
            self.journal.expanded_snapshot_trade_ids.remove(&trade_id);
            return Task::none();
        };

        let now_ms = Self::now_ms();
        let Some(address) = self.connected_address.clone() else {
            self.journal.snapshots.insert(
                trade_id,
                journal::unavailable_snapshot(
                    &trade,
                    self.chart_backfill_source,
                    now_ms,
                    "Connect an account before loading a snapshot.".to_string(),
                ),
            );
            return Task::none();
        };

        let has_fills = self
            .journal
            .trade_details
            .get(&trade.id)
            .is_some_and(|details| !details.attributed_fills.is_empty());

        match self.journal_snapshot_request_for(&trade, address, has_fills, now_ms, None) {
            Ok(request) => self.queue_journal_snapshot_request(request),
            Err(reason) => {
                self.journal.snapshots.insert(
                    trade_id,
                    journal::unavailable_snapshot(
                        &trade,
                        self.chart_backfill_source,
                        now_ms,
                        reason,
                    ),
                );
                Task::none()
            }
        }
    }

    /// Resolve the snapshot request for a trade, choosing between a fill-based
    /// entry → exit chart and a live-position chart (recent window with the
    /// entry level marked) for open positions whose opening fills are not in
    /// the loaded history. `timeframe` pins the rung for the detail selector.
    fn journal_snapshot_request_for(
        &self,
        trade: &journal::AggregatedTrade,
        address: String,
        has_fills: bool,
        now_ms: u64,
        timeframe: Option<crate::timeframe::Timeframe>,
    ) -> Result<journal::JournalTradeSnapshotRequest, String> {
        let account_key = self.active_journal_account_key();
        let source = self.chart_backfill_source;
        let rdg = self.read_data_provider_generation;
        let hkg = self.hydromancer_key_generation;

        let fill_based = |addr: String| match timeframe {
            Some(tf) => journal::snapshot_request_for_timeframe(
                account_key.clone(),
                addr,
                trade,
                source,
                rdg,
                hkg,
                now_ms,
                tf,
            ),
            None => journal::initial_snapshot_request(
                account_key.clone(),
                addr,
                trade,
                source,
                rdg,
                hkg,
                now_ms,
            ),
        };
        let live = |addr: String| match timeframe {
            Some(tf) => journal::live_position_snapshot_request_for_timeframe(
                account_key.clone(),
                addr,
                trade,
                source,
                rdg,
                hkg,
                now_ms,
                tf,
            ),
            None => journal::live_position_snapshot_request(
                account_key.clone(),
                addr,
                trade,
                source,
                rdg,
                hkg,
                now_ms,
            ),
        };

        if has_fills {
            // A trade with attributed fills is charted against those fills. If
            // its basis is incomplete we report that honestly rather than
            // falling back to a live-position window — the live render path
            // keys on "no fills", so falling back here would build a recent
            // window but render it with fabricated entry/exit boundaries.
            fill_based(address)
        } else {
            live(address).map_err(|reason| {
                // A closed trade with no fills can never be a live position;
                // keep the historical attribution-missing message.
                if trade.end_time.is_some() {
                    "Snapshot unavailable because fill attribution is missing.".to_string()
                } else {
                    reason
                }
            })
        }
    }

    /// Re-request a trade's snapshot pinned to a specific timeframe (detail-view
    /// 1m / 5m / 1h selector).
    fn request_journal_snapshot_timeframe(
        &mut self,
        trade_id: String,
        timeframe: crate::timeframe::Timeframe,
    ) -> Task<Message> {
        let Some(trade) = self
            .journal
            .trades
            .iter()
            .find(|trade| trade.id == trade_id)
            .cloned()
        else {
            return Task::none();
        };

        // Skip if the loaded snapshot already matches the requested timeframe.
        if self
            .journal
            .snapshots
            .get(&trade_id)
            .is_some_and(|snapshot| {
                snapshot.source == self.chart_backfill_source && snapshot.timeframe == timeframe
            })
        {
            return Task::none();
        }

        let now_ms = Self::now_ms();
        let Some(address) = self.connected_address.clone() else {
            let snapshot = self.journal_unavailable_snapshot(
                &trade,
                timeframe,
                "Connect an account before loading a snapshot.".to_string(),
            );
            self.journal.snapshots.insert(trade_id, snapshot);
            return Task::none();
        };

        let has_fills = self
            .journal
            .trade_details
            .get(&trade.id)
            .is_some_and(|details| !details.attributed_fills.is_empty());

        match self.journal_snapshot_request_for(&trade, address, has_fills, now_ms, Some(timeframe))
        {
            Ok(request) => self.queue_journal_snapshot_request(request),
            Err(reason) => {
                let snapshot = self.journal_unavailable_snapshot(&trade, timeframe, reason);
                self.journal.snapshots.insert(trade_id, snapshot);
                Task::none()
            }
        }
    }

    fn apply_journal_snapshot_loaded(
        &mut self,
        account_key: Option<String>,
        address: String,
        request: journal::JournalTradeSnapshotRequest,
        result: Result<Vec<api::Candle>, String>,
    ) -> Task<Message> {
        if self.journal.active_account_key != account_key
            || self.connected_address.as_deref() != Some(address.as_str())
            || request.source != self.chart_backfill_source
            || request.read_data_provider_generation != self.read_data_provider_generation
            || (request.source == crate::config::ChartBackfillSource::Hydromancer
                && !self.hydromancer_key_generation_is_current(request.hydromancer_key_generation))
            || self.journal.snapshot_requests.get(&request.trade_id) != Some(&request)
        {
            return Task::none();
        }

        let Some(trade) = self
            .journal
            .trades
            .iter()
            .find(|trade| trade.id == request.trade_id)
            .cloned()
        else {
            self.journal.snapshot_requests.remove(&request.trade_id);
            return Task::none();
        };

        match result {
            Ok(candles) if candles.is_empty() => {
                if let Some(next_request) = journal::next_snapshot_request(&request) {
                    return self.queue_journal_snapshot_request(next_request);
                }
                self.journal.snapshot_requests.remove(&request.trade_id);
                let snapshot = self.journal_unavailable_snapshot(
                    &trade,
                    request.timeframe,
                    "No candle data returned for the trade window.".to_string(),
                );
                self.journal
                    .snapshots
                    .insert(request.trade_id.clone(), snapshot);
            }
            Ok(candles) => {
                self.journal.snapshot_requests.remove(&request.trade_id);
                let details = self.journal.trade_details.get(&request.trade_id);
                match journal::build_journal_trade_snapshot(&request, &trade, details, candles) {
                    Ok(snapshot) => {
                        self.journal
                            .snapshots
                            .insert(request.trade_id.clone(), snapshot);
                    }
                    Err(_) => {
                        let snapshot = self.journal_unavailable_snapshot(
                            &trade,
                            request.timeframe,
                            "Could not compute snapshot metrics for this trade.".to_string(),
                        );
                        self.journal
                            .snapshots
                            .insert(request.trade_id.clone(), snapshot);
                    }
                }
            }
            Err(error) => {
                let error = redact_sensitive_response_text(&error);
                self.journal.snapshot_requests.remove(&request.trade_id);
                let snapshot = self.journal_unavailable_snapshot(
                    &trade,
                    request.timeframe,
                    format!("Could not load candles: {error}"),
                );
                self.journal
                    .snapshots
                    .insert(request.trade_id.clone(), snapshot);
            }
        }

        Task::none()
    }

    /// Build an unavailable-snapshot placeholder pinned to `timeframe` so the
    /// detail-view 1m/5m/1h selector keeps the requested timeframe highlighted
    /// even when no chart could be produced.
    fn journal_unavailable_snapshot(
        &self,
        trade: &journal::AggregatedTrade,
        timeframe: crate::timeframe::Timeframe,
        reason: String,
    ) -> journal::JournalTradeSnapshot {
        let mut snapshot = journal::unavailable_snapshot(
            trade,
            self.chart_backfill_source,
            Self::now_ms(),
            reason,
        );
        snapshot.timeframe = timeframe;
        snapshot
    }

    fn queue_journal_snapshot_request(
        &mut self,
        request: journal::JournalTradeSnapshotRequest,
    ) -> Task<Message> {
        self.journal
            .snapshot_requests
            .insert(request.trade_id.clone(), request.clone());

        let account_key = request.account_key.clone();
        let address = request.address.clone();
        let hydromancer_api_key = self.hydromancer_api_key_for_task();
        let fetch_request = request.clone();

        Task::perform(
            api::fetch_chart_backfill_candles(
                fetch_request.source,
                hydromancer_api_key,
                fetch_request.coin,
                fetch_request.timeframe.api_str().to_string(),
                fetch_request.start_ms,
                fetch_request.end_ms,
            ),
            move |result| Message::JournalSnapshotLoaded {
                account_key: account_key.clone().into(),
                address: address.clone().into(),
                request: request.clone().into(),
                result,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ChartBackfillSource, ReadDataProvider};
    use crate::journal::JournalTradeSnapshotRequest;
    use crate::timeframe::Timeframe;

    fn snapshot_request(generation: u64) -> JournalTradeSnapshotRequest {
        JournalTradeSnapshotRequest {
            account_key: Some("acct".to_string()),
            address: "0xabc".to_string(),
            trade_id: "perp:BTC:test".to_string(),
            coin: "BTC".to_string(),
            source: ChartBackfillSource::Hydromancer,
            read_data_provider_generation: 0,
            hydromancer_key_generation: generation,
            timeframe: Timeframe::M1,
            ladder_index: 0,
            trade_start_ms: 1_000,
            trade_end_ms: 2_000,
            is_open: false,
            start_ms: 0,
            end_ms: 3_000,
        }
    }

    #[test]
    fn stale_hydromancer_generation_does_not_apply_journal_snapshot() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.chart_backfill_source = ChartBackfillSource::Hydromancer;
        terminal.hydromancer_key_generation = 2;
        terminal.journal.active_account_key = Some("acct".to_string());
        terminal.connected_address = Some("0xabc".to_string());

        let request = snapshot_request(1);
        terminal
            .journal
            .snapshot_requests
            .insert(request.trade_id.clone(), request.clone());

        let _task = terminal.update_journal(Message::JournalSnapshotLoaded {
            account_key: Some("acct".to_string()).into(),
            address: "0xabc".to_string().into(),
            request: request.clone().into(),
            result: Ok(vec![api::Candle::test_flat(0, 100.0)]),
        });

        assert_eq!(
            terminal.journal.snapshot_requests.get(&request.trade_id),
            Some(&request)
        );
        assert!(!terminal.journal.snapshots.contains_key(&request.trade_id));
    }

    #[test]
    fn provider_change_clears_pending_journal_snapshot_requests() {
        let mut terminal = TradingTerminal::boot().0;
        let request = snapshot_request(terminal.hydromancer_key_generation);
        terminal
            .journal
            .snapshot_requests
            .insert(request.trade_id.clone(), request);

        let _task = terminal.update_preferences(Message::ReadDataProviderChanged(
            ReadDataProvider::Hydromancer,
        ));

        assert!(terminal.journal.snapshot_requests.is_empty());
    }

    #[test]
    fn reaggregation_preserves_in_flight_snapshot_requests() {
        // Regression: re-aggregating fills while a snapshot is in flight must
        // keep the pending request so its candle response still applies, rather
        // than being silently dropped by the request-equality guard.
        let mut terminal = journal_terminal_with_account();
        let request = snapshot_request(terminal.hydromancer_key_generation);
        let trade = snapshot_trade(&request.trade_id);
        terminal
            .journal
            .snapshot_requests
            .insert(request.trade_id.clone(), request.clone());
        terminal.journal.snapshots.insert(
            request.trade_id.clone(),
            crate::journal::unavailable_snapshot(
                &trade,
                ChartBackfillSource::Hydromancer,
                1_000,
                "stale".to_string(),
            ),
        );

        terminal.journal.clear_snapshot_results();

        assert!(
            terminal.journal.snapshots.is_empty(),
            "stale snapshot results are cleared"
        );
        assert_eq!(
            terminal.journal.snapshot_requests.get(&request.trade_id),
            Some(&request),
            "in-flight snapshot request survives re-aggregation"
        );
    }

    #[test]
    fn reaggregation_prunes_orphaned_selection_and_edit_state() {
        // Regression: a re-aggregation that drops a trade must also clear its
        // selection and in-progress edit drafts so stale state can't resurface.
        let mut terminal = journal_terminal_with_account();
        terminal.journal.selected_trade_id = Some("ghost".to_string());
        terminal
            .journal
            .edit_modes
            .insert("ghost".to_string(), true);
        terminal
            .journal
            .edit_buffers
            .insert("ghost".to_string(), crate::journal::JournalNote::default());
        terminal
            .journal
            .edit_source_keys
            .insert("ghost".to_string(), "ghost".to_string());
        terminal
            .journal
            .edit_tag_raw
            .insert("ghost".to_string(), "breakout".to_string());

        let request_id = terminal.journal.next_sync_request_id();
        terminal.journal.loading = true;
        let _task = terminal.update_journal(Message::JournalFillsLoaded {
            request_id,
            account_key: Some("acct".to_string()).into(),
            address: "0xabc".to_string().into(),
            result: Ok(empty_journal_page(12_345)),
        });

        assert!(terminal.journal.selected_trade_id.is_none());
        assert!(terminal.journal.edit_modes.is_empty());
        assert!(terminal.journal.edit_buffers.is_empty());
        assert!(terminal.journal.edit_source_keys.is_empty());
        assert!(terminal.journal.edit_tag_raw.is_empty());
    }

    #[test]
    fn journal_cause_of_error_updates_edit_buffer() {
        let mut terminal = TradingTerminal::boot().0;

        let _task = terminal.update_journal(Message::JournalCauseOfErrorChanged(
            "trade-a".to_string(),
            "late chase".to_string(),
        ));

        assert_eq!(
            terminal
                .journal
                .edit_buffers
                .get("trade-a")
                .map(|note| note.cause_of_error.as_str()),
            Some("late chase")
        );
    }

    #[test]
    fn stale_journal_fills_result_does_not_finish_newer_sync() {
        let mut terminal = journal_terminal_with_account();
        let stale_request_id = terminal.journal.next_sync_request_id();
        terminal.journal.loading = true;
        let current_request_id = terminal.journal.next_sync_request_id();

        let _task = terminal.update_journal(Message::JournalFillsLoaded {
            request_id: stale_request_id,
            account_key: Some("acct".to_string()).into(),
            address: "0xabc".to_string().into(),
            result: Err("old request failed".to_string()),
        });

        assert_eq!(terminal.journal.sync_request_id, current_request_id);
        assert!(terminal.journal.loading);
        assert!(terminal.journal.error.is_none());
        assert!(terminal.journal.warning.is_none());
        assert_eq!(terminal.journal.sync_status.pages_loaded, 0);
    }

    #[test]
    fn matching_journal_fills_result_finishes_sync() {
        let mut terminal = journal_terminal_with_account();
        let request_id = terminal.journal.next_sync_request_id();
        terminal.journal.loading = true;

        let _task = terminal.update_journal(Message::JournalFillsLoaded {
            request_id,
            account_key: Some("acct".to_string()).into(),
            address: "0xabc".to_string().into(),
            result: Ok(empty_journal_page(12_345)),
        });

        assert!(!terminal.journal.loading);
        assert_eq!(terminal.journal.last_refresh_time, Some(12_345));
        assert_eq!(terminal.journal.sync_status.pages_loaded, 1);
        assert!(terminal.journal.sync_status.complete);
    }

    #[test]
    fn journal_fills_error_redacts_error_when_no_cached_data() {
        let mut terminal = journal_terminal_with_account();
        let request_id = terminal.journal.next_sync_request_id();
        terminal.journal.loading = true;

        let _task = terminal.update_journal(Message::JournalFillsLoaded {
            request_id,
            account_key: Some("acct".to_string()).into(),
            address: "0xabc".to_string().into(),
            result: Err("fills failed: api_key=journal-secret".to_string()),
        });

        let error = terminal.journal.error.as_deref().expect("journal error");
        assert!(error.contains("api_key=<redacted>"));
        assert!(!error.contains("journal-secret"));
        assert!(!terminal.journal.loading);
    }

    #[test]
    fn journal_fills_error_redacts_incomplete_refresh_warning() {
        let mut terminal = journal_terminal_with_account();
        terminal.journal.raw_fills.push(user_fill(1));
        let request_id = terminal.journal.next_sync_request_id();
        terminal.journal.loading = true;

        let _task = terminal.update_journal(Message::JournalFillsLoaded {
            request_id,
            account_key: Some("acct".to_string()).into(),
            address: "0xabc".to_string().into(),
            result: Err("fills failed: signature=warning-secret".to_string()),
        });

        let warning = terminal
            .journal
            .warning
            .as_deref()
            .expect("journal warning");
        assert!(warning.contains("signature=<redacted>"));
        assert!(!warning.contains("warning-secret"));
        assert!(!terminal.journal.loading);
    }

    #[test]
    fn journal_snapshot_error_redacts_unavailable_reason() {
        let mut terminal = journal_terminal_with_account();
        terminal.chart_backfill_source = ChartBackfillSource::Hydromancer;
        terminal.hydromancer_key_generation = 2;
        let request = snapshot_request(2);
        terminal.journal.trades = vec![snapshot_trade(&request.trade_id)];
        terminal
            .journal
            .snapshot_requests
            .insert(request.trade_id.clone(), request.clone());

        let _task = terminal.update_journal(Message::JournalSnapshotLoaded {
            account_key: Some("acct".to_string()).into(),
            address: "0xabc".to_string().into(),
            request: request.clone().into(),
            result: Err("candles failed: auth_token=snapshot-secret".to_string()),
        });

        let snapshot = terminal
            .journal
            .snapshots
            .get(&request.trade_id)
            .expect("journal snapshot");
        let journal::JournalTradeSnapshotStatus::Unavailable(reason) = &snapshot.status else {
            panic!("snapshot should be unavailable");
        };
        assert!(reason.contains("auth_token=<redacted>"));
        assert!(!reason.contains("snapshot-secret"));
    }

    #[test]
    fn open_trade_with_fills_but_incomplete_basis_does_not_fall_back_to_live() {
        // Regression: an OPEN trade WITH attributed fills but an incomplete
        // basis must report "unavailable" rather than silently producing a
        // live-position window — the build/canvas keys "live" on the absence of
        // fills, so a fallback here would draw a fabricated open boundary.
        let terminal = journal_terminal_with_account();
        let mut trade = snapshot_trade("perp:BTC:open_partial");
        trade.end_time = None;
        trade.status = "OPEN".to_string();
        trade.basis_complete = false;

        let with_fills = terminal.journal_snapshot_request_for(
            &trade,
            "0xabc".to_string(),
            true,
            1_700_000_000_000,
            None,
        );
        assert!(
            with_fills.is_err(),
            "fills + incomplete basis must not fall back to a live window: {with_fills:?}"
        );

        // The same open trade with no fills DOES chart as a live position.
        let without_fills = terminal
            .journal_snapshot_request_for(
                &trade,
                "0xabc".to_string(),
                false,
                1_700_000_000_000,
                None,
            )
            .expect("fill-less open position charts live");
        assert!(without_fills.is_open);
    }

    #[test]
    fn live_position_pinned_fine_timeframe_bounds_the_window() {
        // A 1m timeframe over the fixed live lookback must not request the full
        // multi-day window (tens of thousands of candles).
        let terminal = journal_terminal_with_account();
        let trade = open_position_trade("position:BTC");

        let request = terminal
            .journal_snapshot_request_for(
                &trade,
                "0xabc".to_string(),
                false,
                1_700_000_000_000,
                Some(Timeframe::M1),
            )
            .expect("live 1m request");
        let window_ms = 1_700_000_000_000_u64 - request.trade_start_ms;
        assert!(
            window_ms < 24 * 60 * 60 * 1000,
            "1m live window should be bounded to hours, got {window_ms} ms"
        );
    }

    fn journal_terminal_with_account() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.journal.active_account_key = Some("acct".to_string());
        terminal.connected_address = Some("0xabc".to_string());
        terminal
    }

    fn snapshot_trade(id: &str) -> journal::AggregatedTrade {
        journal::AggregatedTrade {
            id: id.to_string(),
            legacy_note_ids: Vec::new(),
            coin: "BTC".to_string(),
            start_time: 1_000,
            end_time: Some(2_000),
            max_position: 1.0,
            volume: 100.0,
            fee: 1.0,
            pnl: 10.0,
            status: "Closed".to_string(),
            fill_count: 2,
            avg_entry_price: 100.0,
            total_entry_notional: 100.0,
            total_entry_size: 1.0,
            is_long: true,
            basis_complete: true,
        }
    }

    fn open_position_trade(id: &str) -> journal::AggregatedTrade {
        let mut trade = snapshot_trade(id);
        trade.end_time = None;
        trade.status = "OPEN".to_string();
        trade.fill_count = 0;
        trade.basis_complete = false;
        trade
    }

    #[test]
    fn synthetic_open_position_without_fills_queues_live_snapshot() {
        // A carried-in / current-position trade has no attributed fills but a
        // known entry price: selecting it must queue a live-position snapshot
        // rather than report missing fill attribution.
        let mut terminal = journal_terminal_with_account();
        let trade = open_position_trade("position:BTC");
        terminal.journal.trades.push(trade.clone());

        let _task = terminal.update_journal(Message::JournalSelectTrade(trade.id.clone()));

        let request = terminal
            .journal
            .snapshot_requests
            .get(&trade.id)
            .expect("live-position snapshot request queued");
        assert!(request.is_open);
        assert!(request.trade_start_ms < request.trade_end_ms);
        assert!(!terminal.journal.snapshots.contains_key(&trade.id));
    }

    #[test]
    fn closed_trade_without_fills_reports_missing_attribution() {
        // A closed trade can never be a live position, so the historical
        // attribution-missing placeholder is preserved.
        let mut terminal = journal_terminal_with_account();
        let trade = snapshot_trade("perp:BTC:closed");
        terminal.journal.trades.push(trade.clone());

        let _task = terminal.update_journal(Message::JournalSelectTrade(trade.id.clone()));

        assert!(!terminal.journal.snapshot_requests.contains_key(&trade.id));
        match &terminal
            .journal
            .snapshots
            .get(&trade.id)
            .expect("unavailable placeholder")
            .status
        {
            journal::JournalTradeSnapshotStatus::Unavailable(reason) => {
                assert!(reason.contains("fill attribution is missing"), "{reason}");
            }
            other => panic!("expected unavailable snapshot, got {other:?}"),
        }
    }

    fn user_fill(time: u64) -> api::UserFill {
        api::UserFill {
            coin: "BTC".to_string(),
            px: "100".to_string(),
            sz: "1".to_string(),
            side: "B".to_string(),
            time,
            start_position: "0".to_string(),
            dir: "Open Long".to_string(),
            closed_pnl: "0".to_string(),
            hash: format!("hash-{time}"),
            oid: time,
            crossed: false,
            fee: "0".to_string(),
            tid: time,
            fee_token: "USDC".to_string(),
        }
    }

    fn empty_journal_page(requested_end_time: u64) -> api::UserFillsPage {
        api::UserFillsPage {
            fills: Vec::new(),
            next_request: None,
            requested_end_time,
            progress_warning: None,
        }
    }
}
