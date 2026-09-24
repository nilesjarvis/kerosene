use crate::api::{self, MarketType};
use crate::app_state::TradingTerminal;
use crate::canvas_state::WorkspaceId;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;
use std::collections::{BTreeSet, HashMap, HashSet};

impl TradingTerminal {
    pub(crate) fn request_outcome_volume_refresh(&mut self) -> Task<Message> {
        let symbols = self.current_outcome_volume_symbols();
        if symbols != self.outcome_volumes_requested_symbols || symbols.is_empty() {
            self.cancel_outcome_volume_refresh();
        }
        self.outcome_volumes_requested_symbols = symbols.clone();
        if symbols.is_empty() {
            return Task::none();
        }
        // Metadata refreshes and additional widgets share the current batch.
        if self.outcome_volumes_loading {
            return Task::none();
        }

        self.outcome_volumes_request_id = self.outcome_volumes_request_id.saturating_add(1);
        let request_id = self.outcome_volumes_request_id;
        self.outcome_volumes_error = None;
        self.outcome_volumes_loading = true;
        let requested_symbols = symbols.clone();
        let (task, handle) =
            Task::perform(api::fetch_outcome_volumes_24h(symbols), move |result| {
                Message::OutcomeVolumesLoaded(request_id, requested_symbols.clone(), result)
            })
            .abortable();
        self.outcome_volumes_task = Some(handle.abort_on_drop());
        task
    }

    fn outcome_volume_widget_is_open(&self) -> bool {
        self.panes
            .iter()
            .any(|(_, kind)| matches!(kind, PaneKind::Outcomes))
            || self.canvases.values().any(|canvas| {
                canvas.window_id.is_some()
                    && canvas
                        .panes
                        .iter()
                        .any(|(_, kind)| matches!(kind, PaneKind::Outcomes))
            })
    }

    /// Reconcile demand after pane/window changes and chart symbol changes.
    /// Unchanged demand does not trigger another refresh or retry.
    pub(crate) fn sync_outcome_volume_demand(&mut self) -> Task<Message> {
        if self.current_outcome_volume_symbols() != self.outcome_volumes_requested_symbols {
            self.request_outcome_volume_refresh()
        } else {
            Task::none()
        }
    }

    fn cancel_outcome_volume_refresh(&mut self) {
        if let Some(handle) = self.outcome_volumes_task.take() {
            handle.abort();
        }
        if self.outcome_volumes_loading {
            self.outcome_volumes_request_id = self.outcome_volumes_request_id.saturating_add(1);
        }
        self.outcome_volumes_loading = false;
        self.outcome_volumes_error = None;
    }

    fn current_outcome_volume_symbols(&self) -> Vec<String> {
        let all_outcomes = self.outcome_volume_widget_is_open();
        // Chart headers consume only their primary symbol's volume. Saved charts
        // in closed canvases must not expand demand to the entire outcome universe.
        let chart_symbols: HashSet<&str> = self
            .workspace_pane_kinds()
            .filter(|(workspace, _, _)| match workspace {
                WorkspaceId::Main => true,
                WorkspaceId::Canvas(id) => self
                    .canvases
                    .get(id)
                    .is_some_and(|canvas| canvas.window_id.is_some()),
            })
            .filter_map(|(_, _, kind)| match kind {
                PaneKind::Chart(id) => Some(*id),
                _ => None,
            })
            .chain(
                self.detached_chart_windows
                    .values()
                    .map(|window| window.chart_id),
            )
            .filter_map(|id| self.charts.get(&id).map(|chart| chart.symbol.as_str()))
            .collect();
        self.exchange_symbols
            .iter()
            .filter(|symbol| symbol.market_type == MarketType::Outcome)
            .filter(|symbol| all_outcomes || chart_symbols.contains(symbol.key.as_str()))
            .filter(|symbol| symbol.is_user_selectable_market())
            .filter(|symbol| !self.exchange_symbol_is_hidden(symbol))
            .map(|symbol| symbol.key.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub(super) fn apply_outcome_volumes_loaded(
        &mut self,
        request_id: u64,
        requested_symbols: Vec<String>,
        result: Result<HashMap<String, api::OutcomeVolume24h>, String>,
    ) -> Task<Message> {
        if !self.outcome_volumes_loading || request_id != self.outcome_volumes_request_id {
            return Task::none();
        }
        if self.current_outcome_volume_symbols() != self.outcome_volumes_requested_symbols {
            return self.sync_outcome_volume_demand();
        }

        self.outcome_volumes_request_id = self.outcome_volumes_request_id.saturating_add(1);
        self.outcome_volumes_loading = false;
        self.outcome_volumes_task = None;
        match result {
            Ok(mut volumes) => {
                let requested_symbols: HashSet<String> = requested_symbols.into_iter().collect();
                let current_symbols: HashSet<String> =
                    self.current_outcome_volume_symbols().into_iter().collect();
                volumes.retain(|symbol, _| {
                    requested_symbols.contains(symbol) && current_symbols.contains(symbol)
                });
                self.outcome_volumes_24h = volumes;
                self.outcome_volumes_error = None;
            }
            Err(error) => {
                self.outcome_volumes_error = Some(redact_sensitive_response_text(&error));
            }
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, OutcomeSymbolInfo, OutcomeVolume24h};

    #[test]
    fn stale_outcome_volume_result_after_newer_request_is_ignored() {
        let mut terminal = terminal_with_outcomes();
        terminal.exchange_symbols = vec![outcome_symbol("#1"), outcome_symbol("#2")];
        let _ = terminal.request_outcome_volume_refresh();
        let stale_request_id = terminal.outcome_volumes_request_id;
        close_outcomes(&mut terminal);
        let _ = terminal.update_panes(Message::AddOutcomesPane);
        let current_request_id = terminal.outcome_volumes_request_id;

        let _ = terminal.apply_outcome_volumes_loaded(
            stale_request_id,
            vec!["#1".to_string(), "#2".to_string()],
            Ok(HashMap::from([("#1".to_string(), volume(1.0))])),
        );

        assert!(terminal.outcome_volumes_loading);
        assert!(terminal.outcome_volumes_24h.is_empty());

        let _ = terminal.apply_outcome_volumes_loaded(
            current_request_id,
            vec!["#1".to_string(), "#2".to_string()],
            Ok(HashMap::from([("#2".to_string(), volume(2.0))])),
        );

        assert!(!terminal.outcome_volumes_loading);
        assert_eq!(
            terminal
                .outcome_volumes_24h
                .get("#2")
                .map(|volume| volume.contract),
            Some(2.0)
        );
        assert!(!terminal.outcome_volumes_24h.contains_key("#1"));
    }

    #[test]
    fn empty_outcome_universe_invalidates_in_flight_volume_request() {
        let mut terminal = terminal_with_outcomes();
        terminal.exchange_symbols = vec![outcome_symbol("#1")];
        let _ = terminal.request_outcome_volume_refresh();
        let stale_request_id = terminal.outcome_volumes_request_id;

        terminal.exchange_symbols.clear();
        let _ = terminal.request_outcome_volume_refresh();
        let _ = terminal.apply_outcome_volumes_loaded(
            stale_request_id,
            vec!["#1".to_string()],
            Ok(HashMap::from([("#1".to_string(), volume(1.0))])),
        );

        assert!(!terminal.outcome_volumes_loading);
        assert!(terminal.outcome_volumes_24h.is_empty());
    }

    #[test]
    fn outcome_volume_result_keeps_only_requested_current_symbols() {
        let mut terminal = terminal_with_outcomes();
        terminal.exchange_symbols = vec![outcome_symbol("#1"), outcome_symbol("#2")];
        let _ = terminal.request_outcome_volume_refresh();
        let request_id = terminal.outcome_volumes_request_id;

        let _ = terminal.apply_outcome_volumes_loaded(
            request_id,
            vec!["#1".to_string()],
            Ok(HashMap::from([
                ("#1".to_string(), volume(1.0)),
                ("#2".to_string(), volume(2.0)),
                ("#3".to_string(), volume(3.0)),
            ])),
        );

        assert_eq!(terminal.outcome_volumes_24h.len(), 1);
        assert_eq!(
            terminal
                .outcome_volumes_24h
                .get("#1")
                .map(|volume| volume.contract),
            Some(1.0)
        );
    }

    #[test]
    fn outcome_volume_error_redacts_sensitive_text() {
        let mut terminal = terminal_with_outcomes();
        terminal.exchange_symbols = vec![outcome_symbol("#1")];
        let _ = terminal.request_outcome_volume_refresh();
        let request_id = terminal.outcome_volumes_request_id;

        let _ = terminal.apply_outcome_volumes_loaded(
            request_id,
            vec!["#1".to_string()],
            Err("outcome volume fetch failed: api_key=super-secret".to_string()),
        );

        assert!(!terminal.outcome_volumes_loading);
        let error = terminal.outcome_volumes_error.as_ref().expect("error");
        assert!(error.contains("api_key=<redacted>"));
        assert!(!error.contains("super-secret"));
    }

    #[test]
    fn symbols_loaded_without_outcomes_widget_does_not_schedule_volume_work() {
        let mut terminal = TradingTerminal::boot().0;
        let request_id = terminal.outcome_volumes_request_id;
        let _ = terminal.update_market(Message::SymbolsLoaded(Ok(api::ExchangeSymbolsPayload {
            symbols: vec![perp_symbol("HYPE"), outcome_symbol("#1")],
            loaded_from_cache: false,
            perp_meta_failed: false,
            spot_meta_failed: false,
            outcome_meta_failed: false,
        })));

        assert!(!terminal.outcome_volumes_loading);
        assert!(terminal.outcome_volumes_task.is_none());
        assert_eq!(terminal.outcome_volumes_request_id, request_id);
        assert_eq!(terminal.exchange_symbols.len(), 2);
    }

    #[test]
    fn opening_outcomes_starts_volume_refresh_and_repeated_requests_share_it() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![outcome_symbol("#1")];
        let _ = terminal.update_panes(Message::AddOutcomesPane);
        let request_id = terminal.outcome_volumes_request_id;
        let handle = terminal.outcome_volumes_task.clone().expect("volume task");

        let _ = terminal.request_outcome_volume_refresh();
        let _ = terminal.update_panes(Message::AddOutcomesPane);

        assert!(terminal.outcome_volumes_loading);
        assert_eq!(terminal.outcome_volumes_request_id, request_id);
        assert!(!handle.is_aborted());
    }

    #[test]
    fn closing_last_outcomes_widget_aborts_work_and_preserves_known_volumes() {
        let mut terminal = terminal_with_outcomes();
        terminal.exchange_symbols = vec![outcome_symbol("#1")];
        terminal
            .outcome_volumes_24h
            .insert("#1".into(), volume(5.0));
        let _ = terminal.request_outcome_volume_refresh();
        let request_id = terminal.outcome_volumes_request_id;
        let handle = terminal.outcome_volumes_task.clone().expect("volume task");

        close_outcomes(&mut terminal);
        let _ = terminal.apply_outcome_volumes_loaded(
            request_id,
            vec!["#1".into()],
            Ok(HashMap::from([("#1".into(), volume(99.0))])),
        );

        assert!(handle.is_aborted());
        assert!(!terminal.outcome_volumes_loading);
        assert!(terminal.outcome_volumes_task.is_none());
        assert_eq!(terminal.outcome_volumes_24h["#1"], volume(5.0));
    }

    #[test]
    fn closed_canvas_does_not_fetch_and_reopening_resumes_outcome_volumes() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![outcome_symbol("#1")];
        terminal.insert_test_canvas_pane(7, PaneKind::Outcomes);
        let _ = terminal.request_outcome_volume_refresh();
        assert!(!terminal.outcome_volumes_loading);

        let _ = terminal.open_canvas_window(7);
        let handle = terminal.outcome_volumes_task.clone().expect("volume task");
        let window_id = terminal.canvases[&7].window_id.expect("canvas window");
        assert!(terminal.outcome_volumes_loading);

        let _ = terminal.update_window(Message::WindowClosed(window_id));
        assert!(handle.is_aborted());
        assert!(!terminal.outcome_volumes_loading);
        assert!(terminal.canvases.contains_key(&7));

        let _ = terminal.open_canvas_window(7);
        assert!(terminal.outcome_volumes_loading);
        assert!(
            !terminal
                .outcome_volumes_task
                .as_ref()
                .expect("new task")
                .is_aborted()
        );
    }

    #[test]
    fn closing_one_widget_keeps_refresh_for_another_open_canvas() {
        let mut terminal = terminal_with_outcomes();
        terminal.exchange_symbols = vec![outcome_symbol("#1")];
        terminal.insert_test_canvas_pane(7, PaneKind::Outcomes);
        let _ = terminal.open_canvas_window(7);
        let handle = terminal.outcome_volumes_task.clone().expect("volume task");

        close_outcomes(&mut terminal);

        assert!(terminal.outcome_volumes_loading);
        assert!(!handle.is_aborted());
    }

    #[test]
    fn changing_layout_cancels_hidden_volumes_and_restoring_outcomes_starts_them() {
        let mut terminal = TradingTerminal::boot().0;
        let without_outcomes = terminal.saved_layout_snapshot("without outcomes".into());
        terminal.exchange_symbols = vec![outcome_symbol("#1")];
        let _ = terminal.update_panes(Message::AddOutcomesPane);
        let with_outcomes = terminal.saved_layout_snapshot("with outcomes".into());
        let handle = terminal.outcome_volumes_task.clone().expect("volume task");

        let _ = terminal.apply_layout(without_outcomes);
        assert!(!terminal.outcome_volumes_loading);
        assert!(handle.is_aborted());

        let _ = terminal.apply_layout(with_outcomes);
        assert!(terminal.outcome_volumes_loading);
    }

    #[test]
    fn outcome_chart_demand_is_limited_to_open_primary_symbols_and_tracks_changes() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![
            perp_symbol("HYPE"),
            outcome_symbol("#1"),
            outcome_symbol("#2"),
        ];
        let chart_id = terminal.primary_chart_id.expect("chart");
        terminal.charts.get_mut(&chart_id).expect("chart").symbol = "#1".into();

        let _ = terminal.update(Message::StatusBarTick);
        assert_eq!(terminal.outcome_volumes_requested_symbols, vec!["#1"]);
        let old_handle = terminal.outcome_volumes_task.clone().expect("volume task");

        terminal.charts.get_mut(&chart_id).expect("chart").symbol = "#2".into();
        let _ = terminal.update(Message::StatusBarTick);
        assert!(old_handle.is_aborted());
        assert_eq!(terminal.outcome_volumes_requested_symbols, vec!["#2"]);
        let handle = terminal
            .outcome_volumes_task
            .clone()
            .expect("new volume task");

        terminal.charts.get_mut(&chart_id).expect("chart").symbol = "HYPE".into();
        let _ = terminal.update(Message::StatusBarTick);
        assert!(handle.is_aborted());
        assert!(!terminal.outcome_volumes_loading);
        assert!(terminal.outcome_volumes_requested_symbols.is_empty());
    }

    #[test]
    fn closing_outcomes_cancels_unrelated_markets_but_keeps_open_chart_volume() {
        let mut terminal = terminal_with_outcomes();
        terminal.exchange_symbols = vec![outcome_symbol("#1"), outcome_symbol("#2")];
        let chart_id = terminal.primary_chart_id.expect("chart");
        terminal.charts.get_mut(&chart_id).expect("chart").symbol = "#1".into();
        let _ = terminal.request_outcome_volume_refresh();
        let old_handle = terminal.outcome_volumes_task.clone().expect("volume task");
        assert_eq!(terminal.outcome_volumes_requested_symbols, vec!["#1", "#2"]);

        close_outcomes(&mut terminal);

        assert!(old_handle.is_aborted());
        assert!(terminal.outcome_volumes_loading);
        assert_eq!(terminal.outcome_volumes_requested_symbols, vec!["#1"]);
    }

    #[test]
    fn closed_canvas_charts_do_not_fetch_but_detached_outcome_charts_do() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![outcome_symbol("#1"), outcome_symbol("#2")];
        let chart_id = terminal.primary_chart_id.expect("chart");
        terminal.charts.get_mut(&chart_id).expect("chart").symbol = "#1".into();
        terminal.insert_test_canvas_pane(7, PaneKind::Chart(chart_id));
        let pane = terminal
            .find_pane_matching(|kind| matches!(kind, PaneKind::Chart(id) if *id == chart_id))
            .expect("main chart pane");
        let _ = terminal.update_pane_interactions(Message::ClosePane(WorkspaceId::Main, pane));
        assert!(!terminal.outcome_volumes_loading);
        assert!(terminal.current_outcome_volume_symbols().is_empty());

        let _ = terminal.update_chart(Message::OpenDetachedChart(chart_id));
        let _ = terminal.update(Message::StatusBarTick);
        assert_eq!(terminal.outcome_volumes_requested_symbols, vec!["#1"]);
        let handle = terminal
            .outcome_volumes_task
            .clone()
            .expect("chart volume task");
        let window_id = *terminal
            .detached_chart_windows
            .keys()
            .next()
            .expect("detached window");

        let _ = terminal.update_window(Message::WindowClosed(window_id));
        let _ = terminal.update(Message::StatusBarTick);
        assert!(handle.is_aborted());
        assert!(!terminal.outcome_volumes_loading);
    }

    fn terminal_with_outcomes() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        let _ = terminal.update_panes(Message::AddOutcomesPane);
        terminal
    }

    fn close_outcomes(terminal: &mut TradingTerminal) {
        let pane = terminal
            .find_pane_matching(|kind| matches!(kind, PaneKind::Outcomes))
            .expect("outcomes pane");
        let _ = terminal.update_pane_interactions(Message::ClosePane(
            crate::canvas_state::WorkspaceId::Main,
            pane,
        ));
    }

    fn perp_symbol(key: &str) -> ExchangeSymbol {
        let mut symbol = outcome_symbol(key);
        symbol.market_type = MarketType::Perp;
        symbol.outcome = None;
        symbol
    }

    fn outcome_symbol(key: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: "outcome".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 0,
            max_leverage: 1,
            only_isolated: false,
            growth_mode: false,
            market_type: MarketType::Outcome,
            outcome: Some(OutcomeSymbolInfo {
                outcome_id: 1,
                question_id: None,
                question_name: None,
                question_description: None,
                question_class: None,
                question_underlying: None,
                question_expiry: None,
                question_price_thresholds: Vec::new(),
                question_period: None,
                question_named_outcomes: Vec::new(),
                question_settled_named_outcomes: Vec::new(),
                question_fallback_outcome: None,
                bucket_index: None,
                is_question_fallback: false,
                side_index: 0,
                side_name: "Yes".to_string(),
                outcome_name: "Yes".to_string(),
                description: "Outcome".to_string(),
                class: None,
                underlying: None,
                expiry: None,
                target_price: None,
                period: None,
                quote_symbol: "USDC".to_string(),
                quote_token_index: None,
                encoding: 0,
            }),
        }
    }

    fn volume(contract: f64) -> OutcomeVolume24h {
        OutcomeVolume24h {
            contract,
            notional: contract * 2.0,
        }
    }
}
