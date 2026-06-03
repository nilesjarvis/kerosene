use crate::app_state::TradingTerminal;
use crate::message::Message;

use crate::{api, journal};
use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_journal(&mut self, message: Message) -> Task<Message> {
        match message {
            // ----- Trading Journal messages -----
            Message::JournalFillsLoaded {
                account_key,
                address,
                result,
            } => {
                if self.journal.active_account_key != account_key
                    || self.connected_address.as_deref() != Some(address.as_str())
                {
                    return Task::none();
                }

                let had_chart_history = self.journal.trades.len() >= 2;

                match result {
                    Ok(page) => {
                        self.journal.loaded_address = Some(address.clone());
                        let fetched_count = page.fills.len();
                        let added = journal::merge_fills(&mut self.journal.raw_fills, page.fills);

                        if let Some(next_request) = page.next_request {
                            let request_account_key = account_key.clone();
                            let request_address = address.clone();
                            return Task::perform(
                                api::fetch_user_fills(address, next_request),
                                move |result| Message::JournalFillsLoaded {
                                    account_key: request_account_key.clone(),
                                    address: request_address.clone(),
                                    result,
                                },
                            );
                        }

                        let aggregation = journal::aggregate_trades_with_diagnostics(
                            self.journal.raw_fills.clone(),
                        );
                        let mut warnings = Vec::new();
                        if let Some(warning) = aggregation.diagnostics.warning_message() {
                            warnings.push(warning);
                        }
                        self.journal.trades = aggregation.trades;
                        self.journal.trade_details = aggregation.trade_details;
                        self.journal.clear_snapshot_cache();
                        self.journal.expanded_snapshot_trade_ids.clear();
                        self.journal.error = None;

                        if !self.journal_active_account_is_ghost()
                            && let Err(e) = journal::save_cache(&address, &self.journal.raw_fills)
                        {
                            warnings.push(format!("Could not save journal cache: {}", e));
                        }

                        self.journal.loading = false;
                        self.journal.last_refresh_time = Some(page.requested_end_time);

                        if fetched_count == 0 && added == 0 && self.journal.raw_fills.is_empty() {
                            warnings.push("No fills found for this account.".to_string());
                        }

                        self.journal.warning = if warnings.is_empty() {
                            None
                        } else {
                            Some(warnings.join(" "))
                        };

                        if !had_chart_history && self.journal.trades.len() >= 2 {
                            self.journal.begin_chart_reveal(Self::now_ms());
                        }
                    }
                    Err(e) => {
                        if self.journal.raw_fills.is_empty() {
                            self.journal.error = Some(e);
                        } else {
                            self.journal.warning = Some(format!(
                                "Journal refresh incomplete: {}. Showing cached data.",
                                e
                            ));
                        }
                        self.journal.loading = false;
                    }
                }
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
                self.journal.edit_buffers.insert(id, note);
            }
            Message::JournalEditCancel(id) => {
                self.journal.edit_modes.remove(&id);
                self.journal.edit_source_keys.remove(&id);
                self.journal.edit_buffers.remove(&id);
            }
            Message::JournalEditSave(id) => {
                if let Some(note) = self.journal.edit_buffers.remove(&id) {
                    if let Some(source_key) = self.journal.edit_source_keys.remove(&id)
                        && source_key != id
                    {
                        self.journal.entries.remove(&source_key);
                    }
                    if note.open.trim().is_empty() && note.close.trim().is_empty() {
                        self.journal.entries.remove(&id);
                    } else {
                        self.journal.entries.insert(id.clone(), note);
                    }
                    self.persist_config();
                }
                self.journal.edit_modes.remove(&id);
            }
            Message::JournalBufferChanged(id, is_open, text) => {
                let entry = self.journal.edit_buffers.entry(id).or_default();
                if is_open {
                    entry.open = text;
                } else {
                    entry.close = text;
                }
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
            Message::JournalSnapshotToggle(trade_id) => {
                return self.toggle_journal_snapshot(trade_id);
            }
            Message::JournalSnapshotLoaded {
                account_key,
                address,
                request,
                result,
            } => {
                return self.apply_journal_snapshot_loaded(account_key, address, request, result);
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

    fn toggle_journal_snapshot(&mut self, trade_id: String) -> Task<Message> {
        if self.journal.expanded_snapshot_trade_ids.remove(&trade_id) {
            self.journal.snapshot_requests.remove(&trade_id);
            return Task::none();
        }

        self.journal
            .expanded_snapshot_trade_ids
            .insert(trade_id.clone());

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

        let Some(details) = self.journal.trade_details.get(&trade.id) else {
            self.journal.snapshots.insert(
                trade_id,
                journal::unavailable_snapshot(
                    &trade,
                    self.chart_backfill_source,
                    now_ms,
                    "Snapshot unavailable because fill attribution is missing.".to_string(),
                ),
            );
            return Task::none();
        };
        if details.attributed_fills.is_empty() {
            self.journal.snapshots.insert(
                trade_id,
                journal::unavailable_snapshot(
                    &trade,
                    self.chart_backfill_source,
                    now_ms,
                    "Snapshot unavailable because this trade has no attributed fills.".to_string(),
                ),
            );
            return Task::none();
        }

        let request = match journal::initial_snapshot_request(
            self.active_journal_account_key(),
            address,
            &trade,
            self.chart_backfill_source,
            now_ms,
        ) {
            Ok(request) => request,
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
                return Task::none();
            }
        };

        self.queue_journal_snapshot_request(request)
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
                self.journal.snapshots.insert(
                    request.trade_id.clone(),
                    journal::unavailable_snapshot(
                        &trade,
                        self.chart_backfill_source,
                        Self::now_ms(),
                        "No candle data returned for the trade window.".to_string(),
                    ),
                );
            }
            Ok(candles) => {
                self.journal.snapshot_requests.remove(&request.trade_id);
                match self
                    .journal
                    .trade_details
                    .get(&request.trade_id)
                    .and_then(|details| {
                        journal::build_journal_trade_snapshot(&request, &trade, details, candles)
                            .ok()
                    }) {
                    Some(snapshot) => {
                        self.journal
                            .snapshots
                            .insert(request.trade_id.clone(), snapshot);
                    }
                    None => {
                        self.journal.snapshots.insert(
                            request.trade_id.clone(),
                            journal::unavailable_snapshot(
                                &trade,
                                self.chart_backfill_source,
                                Self::now_ms(),
                                "Could not compute snapshot metrics for this trade.".to_string(),
                            ),
                        );
                    }
                }
            }
            Err(error) => {
                self.journal.snapshot_requests.remove(&request.trade_id);
                self.journal.snapshots.insert(
                    request.trade_id.clone(),
                    journal::unavailable_snapshot(
                        &trade,
                        self.chart_backfill_source,
                        Self::now_ms(),
                        format!("Could not load candles: {error}"),
                    ),
                );
            }
        }

        Task::none()
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
        let hydromancer_api_key = self.hydromancer_api_key.trim().to_string();
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
                account_key: account_key.clone(),
                address: address.clone(),
                request: request.clone(),
                result,
            },
        )
    }
}
