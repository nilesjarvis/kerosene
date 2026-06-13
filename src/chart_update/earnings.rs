use crate::api::{ExchangeSymbol, MarketType, SecEarningsEvent, fetch_sec_earnings_events};
use crate::app_state::TradingTerminal;
use crate::chart::EarningsMarker;
use crate::chart_state::{ChartId, ChartInstance};
use crate::message::Message;

use iced::Task;

// ---------------------------------------------------------------------------
// SEC Earnings Markers
// ---------------------------------------------------------------------------

const SEC_EARNINGS_CACHE_MAX_ENTRIES: usize = 32;
const SEC_EARNINGS_STOCK_CATEGORY: &str = "stocks";

impl TradingTerminal {
    pub(super) fn update_chart_earnings(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleChartEarningsMarkers(chart_id) => {
                self.toggle_chart_earnings_markers(chart_id)
            }
            Message::ChartEarningsEventsLoaded(ticker, request_id, result) => {
                self.apply_sec_earnings_loaded(ticker, request_id, *result);
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub(crate) fn chart_earnings_markers_available(&self, instance: &ChartInstance) -> bool {
        self.sec_earnings_ticker_for_symbol(&instance.symbol)
            .is_some()
    }

    pub(crate) fn sec_earnings_ticker_for_symbol(&self, symbol_key: &str) -> Option<String> {
        if symbol_key.trim().is_empty() || self.symbol_key_is_hidden(symbol_key) {
            return None;
        }

        let symbol = self.resolve_exchange_symbol_by_key_or_ticker(symbol_key)?;
        Self::sec_earnings_ticker_for_exchange_symbol(symbol)
    }

    pub(crate) fn sec_earnings_ticker_for_exchange_symbol(
        symbol: &ExchangeSymbol,
    ) -> Option<String> {
        if symbol.market_type != MarketType::Perp
            || !symbol.is_user_selectable_market()
            || !symbol
                .category
                .eq_ignore_ascii_case(SEC_EARNINGS_STOCK_CATEGORY)
        {
            return None;
        }

        let (_, ticker) = symbol.key.split_once(':')?;
        let ticker = ticker.trim();
        (!ticker.is_empty()).then(|| ticker.to_ascii_uppercase())
    }

    pub(crate) fn maybe_fetch_chart_earnings(&mut self, chart_id: ChartId) -> Task<Message> {
        let ticker = match self.chart_earnings_request_ticker(chart_id) {
            Some(ticker) => ticker,
            None => {
                if let Some(instance) = self.charts.get_mut(&chart_id)
                    && instance.show_earnings_markers
                {
                    Self::clear_earnings_display(instance);
                }
                return Task::none();
            }
        };

        if let Some(events) = self.sec_earnings_cache.get(&ticker).cloned() {
            self.apply_sec_earnings_to_chart(chart_id, &ticker, &events, true);
            return Task::none();
        }

        if let Some(waiting_charts) = self.sec_earnings_pending_charts.get_mut(&ticker) {
            if !waiting_charts.contains(&chart_id) {
                waiting_charts.push(chart_id);
            }
            if let Some(instance) = self.charts.get_mut(&chart_id) {
                instance.earnings_fetching = true;
                instance.earnings_pending_ticker = Some(ticker);
                instance.earnings_status =
                    Some(("EARN waiting for shared request".to_string(), false));
            }
            return Task::none();
        }

        self.sec_earnings_pending_charts
            .insert(ticker.clone(), vec![chart_id]);
        self.sec_earnings_request_id = self.sec_earnings_request_id.wrapping_add(1);
        let request_id = self.sec_earnings_request_id;
        self.sec_earnings_pending_request_ids
            .insert(ticker.clone(), request_id);
        if let Some(instance) = self.charts.get_mut(&chart_id) {
            instance.earnings_fetching = true;
            instance.earnings_pending_ticker = Some(ticker.clone());
            instance.earnings_status = Some(("EARN loading".to_string(), false));
        }

        Task::perform(fetch_sec_earnings_events(ticker.clone()), move |result| {
            Message::ChartEarningsEventsLoaded(ticker.clone(), request_id, Box::new(result))
        })
    }

    pub(crate) fn refresh_enabled_earnings_charts(&mut self) -> Task<Message> {
        let ids: Vec<ChartId> = self
            .charts
            .iter()
            .filter(|(_, instance)| instance.show_earnings_markers)
            .map(|(id, _)| *id)
            .collect();

        if ids.is_empty() {
            return Task::none();
        }

        Task::batch(
            ids.into_iter()
                .map(|chart_id| self.maybe_fetch_chart_earnings(chart_id)),
        )
    }

    pub(crate) fn clear_earnings_display(instance: &mut ChartInstance) {
        instance.earnings_events = None;
        instance.earnings_fetching = false;
        instance.earnings_status = None;
        instance.earnings_pending_ticker = None;
        instance.chart.clear_earnings_markers();
    }

    fn toggle_chart_earnings_markers(&mut self, chart_id: ChartId) -> Task<Message> {
        let Some(is_enabled) = self
            .charts
            .get(&chart_id)
            .map(|instance| instance.show_earnings_markers)
        else {
            return Task::none();
        };

        if is_enabled {
            if let Some(instance) = self.charts.get_mut(&chart_id) {
                instance.show_earnings_markers = false;
                Self::clear_earnings_display(instance);
            }
            self.clear_chart_earnings_pending_request_state(chart_id);
            self.persist_config();
            return Task::none();
        }

        let ticker = self
            .charts
            .get(&chart_id)
            .and_then(|instance| self.sec_earnings_ticker_for_symbol(&instance.symbol));

        if ticker.is_none() {
            if let Some(instance) = self.charts.get_mut(&chart_id) {
                instance.show_earnings_markers = false;
                Self::clear_earnings_display(instance);
                instance.earnings_status =
                    Some(("EARN unavailable for this market".to_string(), true));
            }
            self.push_toast(
                "SEC earnings markers are available only for HIP-3 stock tickers".to_string(),
                true,
            );
            return Task::none();
        }

        if let Some(instance) = self.charts.get_mut(&chart_id) {
            instance.show_earnings_markers = true;
        }
        self.persist_config();
        self.maybe_fetch_chart_earnings(chart_id)
    }

    fn chart_earnings_request_ticker(&self, chart_id: ChartId) -> Option<String> {
        let instance = self.charts.get(&chart_id)?;
        if !instance.show_earnings_markers || instance.symbol.is_empty() {
            return None;
        }
        self.sec_earnings_ticker_for_symbol(&instance.symbol)
    }

    fn apply_sec_earnings_loaded(
        &mut self,
        ticker: String,
        request_id: u64,
        result: Result<Vec<SecEarningsEvent>, String>,
    ) {
        let ticker = ticker.to_ascii_uppercase();
        let Some(pending_request_id) = self.sec_earnings_pending_request_ids.get(&ticker).copied()
        else {
            return;
        };
        if pending_request_id != request_id {
            return;
        }
        self.sec_earnings_pending_request_ids.remove(&ticker);
        let pending = self
            .sec_earnings_pending_charts
            .remove(&ticker)
            .unwrap_or_default();
        match result {
            Ok(events) => {
                self.cache_sec_earnings_events(ticker.clone(), events.clone());
                for chart_id in pending {
                    self.apply_sec_earnings_to_chart(chart_id, &ticker, &events, false);
                }
            }
            Err(error) => {
                for chart_id in pending {
                    if let Some(instance) = self.charts.get_mut(&chart_id)
                        && instance.earnings_pending_ticker.as_deref() == Some(ticker.as_str())
                    {
                        instance.earnings_fetching = false;
                        instance.earnings_pending_ticker = None;
                        instance.earnings_events = None;
                        instance.earnings_status = Some((earnings_error_status(&error), true));
                        instance.chart.clear_earnings_markers();
                    }
                }
                self.push_toast(
                    format!("SEC earnings fetch failed for {ticker}: {error}"),
                    true,
                );
            }
        }
    }

    fn cache_sec_earnings_events(&mut self, ticker: String, events: Vec<SecEarningsEvent>) {
        self.sec_earnings_cache_order.retain(|key| key != &ticker);
        self.sec_earnings_cache.insert(ticker.clone(), events);
        self.sec_earnings_cache_order.push_back(ticker);

        while self.sec_earnings_cache_order.len() > SEC_EARNINGS_CACHE_MAX_ENTRIES {
            if let Some(oldest) = self.sec_earnings_cache_order.pop_front() {
                self.sec_earnings_cache.remove(&oldest);
            }
        }
    }

    fn apply_sec_earnings_to_chart(
        &mut self,
        chart_id: ChartId,
        ticker: &str,
        events: &[SecEarningsEvent],
        from_cache: bool,
    ) {
        let can_accept = self
            .charts
            .get(&chart_id)
            .and_then(|instance| self.chart_earnings_request_ticker_for_instance(instance))
            .is_some_and(|chart_ticker| chart_ticker == ticker);

        if !can_accept {
            if let Some(instance) = self.charts.get_mut(&chart_id)
                && instance.earnings_pending_ticker.as_deref() == Some(ticker)
            {
                instance.earnings_fetching = false;
                instance.earnings_pending_ticker = None;
            }
            return;
        }

        if let Some(instance) = self.charts.get_mut(&chart_id) {
            let markers = earnings_markers_from_events(events);
            instance.earnings_events = Some(events.to_vec());
            instance.earnings_fetching = false;
            instance.earnings_pending_ticker = None;
            instance.earnings_status = Some((
                if events.is_empty() {
                    "EARN no SEC Item 2.02 filings".to_string()
                } else if from_cache {
                    format!("EARN cached, {} events", events.len())
                } else {
                    format!("EARN {} events", events.len())
                },
                false,
            ));
            instance.chart.set_earnings_markers(markers);
        }
    }

    fn chart_earnings_request_ticker_for_instance(
        &self,
        instance: &ChartInstance,
    ) -> Option<String> {
        if !instance.show_earnings_markers || instance.symbol.is_empty() {
            return None;
        }
        self.sec_earnings_ticker_for_symbol(&instance.symbol)
    }
}

fn earnings_error_status(error: &str) -> String {
    if error.contains(" 403") {
        "EARN SEC blocked".to_string()
    } else if error.contains("SEC CIK not found") {
        "EARN no SEC ticker match".to_string()
    } else {
        "EARN fetch failed".to_string()
    }
}

fn earnings_markers_from_events(events: &[SecEarningsEvent]) -> Vec<EarningsMarker> {
    events
        .iter()
        .map(|event| EarningsMarker {
            time_ms: event.filing_time_ms,
            filing_date: event.filing_date.clone(),
            accession_number: event.accession_number.clone(),
            quarter_label: earnings_quarter_label(&event.filing_date),
        })
        .collect()
}

fn earnings_quarter_label(filing_date: &str) -> Option<String> {
    let mut parts = filing_date.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let (quarter, label_year) = match month {
        1..=3 => (4, year - 1),
        4..=6 => (1, year),
        7..=9 => (2, year),
        10..=12 => (3, year),
        _ => return None,
    };
    Some(format!("Q{quarter} {label_year}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ExchangeSymbol;
    use crate::chart_state::ChartInstance;
    use crate::config::KeroseneConfig;
    use crate::timeframe::Timeframe;

    fn symbol(key: &str, ticker: &str, category: &str, market_type: MarketType) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: ticker.to_string(),
            category: category.to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 0,
            max_leverage: 0,
            only_isolated: false,
            market_type,
            outcome: None,
        }
    }

    fn earnings_event(ticker: &str, accession_number: &str) -> SecEarningsEvent {
        SecEarningsEvent {
            ticker: ticker.to_string(),
            company_name: format!("{ticker} Inc."),
            cik: 1_045_819,
            filing_date: "2026-05-28".to_string(),
            filing_time_ms: 1_779_926_400_000,
            report_date: Some("2026-05-28".to_string()),
            form: "8-K".to_string(),
            accession_number: accession_number.to_string(),
            primary_document: format!("{}-20260528.htm", ticker.to_ascii_lowercase()),
        }
    }

    #[test]
    fn sec_earnings_ticker_requires_hip3_stock_perp() {
        assert_eq!(
            TradingTerminal::sec_earnings_ticker_for_exchange_symbol(&symbol(
                "xyz:NVDA",
                "NVDA",
                "stocks",
                MarketType::Perp
            )),
            Some("NVDA".to_string())
        );
        assert_eq!(
            TradingTerminal::sec_earnings_ticker_for_exchange_symbol(&symbol(
                "BTC",
                "BTC",
                "crypto",
                MarketType::Perp
            )),
            None
        );
        assert_eq!(
            TradingTerminal::sec_earnings_ticker_for_exchange_symbol(&symbol(
                "km:US500",
                "US500",
                "indices",
                MarketType::Perp
            )),
            None
        );
        assert_eq!(
            TradingTerminal::sec_earnings_ticker_for_exchange_symbol(&symbol(
                "@107",
                "HYPE",
                "spot",
                MarketType::Spot
            )),
            None
        );
    }

    #[test]
    fn earnings_markers_keep_event_timestamp_and_identifiers() {
        let events = vec![SecEarningsEvent {
            ticker: "GOOGL".to_string(),
            company_name: "Alphabet Inc.".to_string(),
            cik: 1_652_044,
            filing_date: "2026-04-29".to_string(),
            filing_time_ms: 1_777_420_800_000,
            report_date: Some("2026-04-29".to_string()),
            form: "8-K".to_string(),
            accession_number: "0001652044-26-000043".to_string(),
            primary_document: "goog-20260429.htm".to_string(),
        }];

        let markers = earnings_markers_from_events(&events);

        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].time_ms, 1_777_420_800_000);
        assert_eq!(markers[0].filing_date, "2026-04-29");
        assert_eq!(markers[0].accession_number, "0001652044-26-000043");
        assert_eq!(markers[0].quarter_label.as_deref(), Some("Q1 2026"));
    }

    #[test]
    fn earnings_quarter_label_infers_reporting_quarter_from_filing_date() {
        assert_eq!(
            earnings_quarter_label("2026-02-03").as_deref(),
            Some("Q4 2025")
        );
        assert_eq!(
            earnings_quarter_label("2026-04-29").as_deref(),
            Some("Q1 2026")
        );
        assert_eq!(
            earnings_quarter_label("2026-07-23").as_deref(),
            Some("Q2 2026")
        );
        assert_eq!(
            earnings_quarter_label("2026-10-22").as_deref(),
            Some("Q3 2026")
        );
        assert_eq!(earnings_quarter_label("bad-date"), None);
    }

    #[test]
    fn earnings_error_status_labels_common_sec_failures() {
        assert_eq!(
            earnings_error_status("SEC request to ticker map returned 403 Forbidden"),
            "EARN SEC blocked"
        );
        assert_eq!(
            earnings_error_status("SEC CIK not found for FOO"),
            "EARN no SEC ticker match"
        );
        assert_eq!(
            earnings_error_status("SEC response parse failed: expected value"),
            "EARN fetch failed"
        );
    }

    #[test]
    fn stale_earnings_result_does_not_clear_current_pending_request() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal
            .sec_earnings_pending_request_ids
            .insert("NVDA".to_string(), 2);
        terminal
            .sec_earnings_pending_charts
            .insert("NVDA".to_string(), vec![1]);

        terminal.apply_sec_earnings_loaded(
            "NVDA".to_string(),
            1,
            Ok(vec![earnings_event("NVDA", "old")]),
        );

        assert_eq!(
            terminal.sec_earnings_pending_request_ids.get("NVDA"),
            Some(&2)
        );
        assert_eq!(
            terminal.sec_earnings_pending_charts.get("NVDA"),
            Some(&vec![1])
        );
        assert!(!terminal.sec_earnings_cache.contains_key("NVDA"));
    }

    #[test]
    fn stale_earnings_error_does_not_clear_current_chart_or_toast() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.charts.clear();
        let mut instance = ChartInstance::new(1, "xyz:NVDA".to_string(), Timeframe::H1);
        instance.show_earnings_markers = true;
        instance.earnings_fetching = true;
        instance.earnings_pending_ticker = Some("NVDA".to_string());
        terminal.charts.insert(1, instance);
        terminal
            .sec_earnings_pending_request_ids
            .insert("NVDA".to_string(), 2);
        terminal
            .sec_earnings_pending_charts
            .insert("NVDA".to_string(), vec![1]);

        terminal.apply_sec_earnings_loaded("NVDA".to_string(), 1, Err("old failure".to_string()));

        assert_eq!(
            terminal.sec_earnings_pending_request_ids.get("NVDA"),
            Some(&2)
        );
        assert_eq!(
            terminal.sec_earnings_pending_charts.get("NVDA"),
            Some(&vec![1])
        );
        let instance = terminal.charts.get(&1).expect("chart");
        assert!(instance.earnings_fetching);
        assert_eq!(instance.earnings_pending_ticker.as_deref(), Some("NVDA"));
        assert!(terminal.toasts.is_empty());
    }

    #[test]
    fn current_earnings_result_applies_and_clears_pending_request() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal
            .sec_earnings_pending_request_ids
            .insert("NVDA".to_string(), 3);
        terminal
            .sec_earnings_pending_charts
            .insert("NVDA".to_string(), Vec::new());

        terminal.apply_sec_earnings_loaded(
            "NVDA".to_string(),
            3,
            Ok(vec![earnings_event("NVDA", "accepted")]),
        );

        assert!(
            !terminal
                .sec_earnings_pending_request_ids
                .contains_key("NVDA")
        );
        assert!(!terminal.sec_earnings_pending_charts.contains_key("NVDA"));
        let events = terminal.sec_earnings_cache.get("NVDA").expect("cache");
        assert_eq!(events[0].accession_number, "accepted");
    }

    #[test]
    fn duplicate_earnings_result_after_completion_does_not_overwrite_cache() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal
            .sec_earnings_cache
            .insert("NVDA".to_string(), vec![earnings_event("NVDA", "accepted")]);

        terminal.apply_sec_earnings_loaded(
            "NVDA".to_string(),
            7,
            Ok(vec![earnings_event("NVDA", "duplicate")]),
        );

        let events = terminal.sec_earnings_cache.get("NVDA").expect("cache");
        assert_eq!(events[0].accession_number, "accepted");
    }

    #[test]
    fn disabling_earnings_markers_removes_pending_waiter_and_ignores_late_error() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.exchange_symbols = vec![symbol("xyz:NVDA", "NVDA", "stocks", MarketType::Perp)];
        terminal.charts.clear();
        let mut instance = ChartInstance::new(1, "xyz:NVDA".to_string(), Timeframe::H1);
        instance.show_earnings_markers = true;
        instance.earnings_fetching = true;
        instance.earnings_pending_ticker = Some("NVDA".to_string());
        instance.earnings_status = Some(("EARN loading".to_string(), false));
        terminal.charts.insert(1, instance);
        terminal
            .sec_earnings_pending_request_ids
            .insert("NVDA".to_string(), 7);
        terminal
            .sec_earnings_pending_charts
            .insert("NVDA".to_string(), vec![1]);

        let _task = terminal.toggle_chart_earnings_markers(1);
        terminal.apply_sec_earnings_loaded("NVDA".to_string(), 7, Err("late failure".to_string()));

        assert!(!terminal.sec_earnings_pending_charts.contains_key("NVDA"));
        assert!(
            !terminal
                .sec_earnings_pending_request_ids
                .contains_key("NVDA")
        );
        assert!(terminal.toasts.is_empty());
        let instance = terminal.charts.get(&1).expect("chart");
        assert!(!instance.show_earnings_markers);
        assert!(!instance.earnings_fetching);
        assert!(instance.earnings_pending_ticker.is_none());
        assert!(instance.earnings_status.is_none());
    }

    #[test]
    fn stale_earnings_result_does_not_apply_after_symbol_change() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.exchange_symbols = vec![
            symbol("xyz:NVDA", "NVDA", "stocks", MarketType::Perp),
            symbol("xyz:TSLA", "TSLA", "stocks", MarketType::Perp),
        ];
        terminal.charts.clear();

        let mut instance = ChartInstance::new(1, "xyz:TSLA".to_string(), Timeframe::H1);
        instance.show_earnings_markers = true;
        instance.earnings_fetching = true;
        instance.earnings_pending_ticker = Some("NVDA".to_string());
        terminal.charts.insert(1, instance);

        let events = vec![SecEarningsEvent {
            ticker: "NVDA".to_string(),
            company_name: "NVIDIA Corporation".to_string(),
            cik: 1_045_819,
            filing_date: "2026-05-28".to_string(),
            filing_time_ms: 1_779_926_400_000,
            report_date: Some("2026-05-28".to_string()),
            form: "8-K".to_string(),
            accession_number: "0001045819-26-000001".to_string(),
            primary_document: "nvda-20260528.htm".to_string(),
        }];

        terminal.apply_sec_earnings_to_chart(1, "NVDA", &events, false);

        let instance = terminal.charts.get(&1).expect("chart");
        assert!(instance.chart.earnings_markers.is_empty());
        assert!(instance.earnings_events.is_none());
        assert!(!instance.earnings_fetching);
        assert!(instance.earnings_pending_ticker.is_none());
    }
}
