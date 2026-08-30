use crate::api::{self, ExchangeSymbol, WatchlistContext};
use crate::app_state::TradingTerminal;
use crate::config::{self, AxisConfig, PaneKindConfig, PaneLayoutConfig};
use crate::helpers::{positive_percent_change, redact_sensitive_response_text};
use crate::message::Message;
use iced::Task;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

const BUILT_IN_CHART_COUNT: usize = 8;

// ---------------------------------------------------------------------------
// Built-In Layout State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BuiltInLayout {
    TopVolume24h,
    TopOpenInterest,
    TopGainers24h,
}

impl BuiltInLayout {
    pub(crate) const ALL: [Self; 3] = [
        Self::TopVolume24h,
        Self::TopOpenInterest,
        Self::TopGainers24h,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::TopVolume24h => "Top 8 by 24h Volume",
            Self::TopOpenInterest => "Top 8 by Open Interest",
            Self::TopGainers24h => "Top 8 by 24h Gain %",
        }
    }

    pub(crate) fn kind_label(self) -> &'static str {
        match self {
            Self::TopVolume24h | Self::TopOpenInterest | Self::TopGainers24h => "Dynamic",
        }
    }

    pub(crate) fn loading_label(self) -> &'static str {
        match self {
            Self::TopVolume24h => "Refreshing 24h volumes...",
            Self::TopOpenInterest => "Refreshing open interest...",
            Self::TopGainers24h => "Refreshing 24h gains...",
        }
    }

    pub(crate) fn preview_layout(self) -> PaneLayoutConfig {
        match self {
            Self::TopVolume24h | Self::TopOpenInterest | Self::TopGainers24h => {
                top_eight_grid_layout()
            }
        }
    }

    fn supports_symbol(self, symbol: &ExchangeSymbol) -> bool {
        match self {
            // Outcome-market volume is calculated from candles on its own refresh
            // path; the exchange context endpoint used here covers perp and spot.
            Self::TopVolume24h => symbol.market_type != api::MarketType::Outcome,
            Self::TopGainers24h | Self::TopOpenInterest => {
                symbol.market_type == api::MarketType::Perp
            }
        }
    }

    fn metric_value(self, context: &WatchlistContext) -> Option<f64> {
        match self {
            Self::TopVolume24h => context.day_vlm,
            Self::TopOpenInterest => context.open_interest_notional,
            Self::TopGainers24h => positive_percent_change(context.mark_px, context.prev_day_px),
        }
    }

    fn metric_is_rankable(self, metric: f64) -> bool {
        metric.is_finite()
            && match self {
                Self::TopGainers24h => metric > 0.0,
                Self::TopVolume24h | Self::TopOpenInterest => metric >= 0.0,
            }
    }

    fn metric_label(self) -> &'static str {
        match self {
            Self::TopVolume24h => "24h-volume",
            Self::TopOpenInterest => "open-interest",
            Self::TopGainers24h => "positive 24h-gain",
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct BuiltInLayoutState {
    active: Option<BuiltInLayout>,
    loading: Option<BuiltInLayout>,
    request_id: u64,
}

impl BuiltInLayoutState {
    pub(crate) fn active(&self) -> Option<BuiltInLayout> {
        self.active
    }

    pub(crate) fn is_loading(&self, layout: BuiltInLayout) -> bool {
        self.loading == Some(layout)
    }

    fn begin_request(&mut self, layout: BuiltInLayout) -> u64 {
        self.request_id = self.request_id.wrapping_add(1);
        self.loading = Some(layout);
        self.request_id
    }

    fn request_is_current(&self, request_id: u64, layout: BuiltInLayout) -> bool {
        self.request_id == request_id && self.loading == Some(layout)
    }

    fn activate(&mut self, layout: BuiltInLayout) {
        self.active = Some(layout);
        self.loading = None;
    }

    fn finish_request(&mut self) {
        self.loading = None;
    }

    pub(crate) fn deactivate(&mut self) {
        self.request_id = self.request_id.wrapping_add(1);
        self.active = None;
        self.loading = None;
    }
}

// ---------------------------------------------------------------------------
// Built-In Layout Updates
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn update_built_in_layouts(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LoadBuiltInLayout(layout) => self.request_built_in_layout(layout),
            Message::BuiltInLayoutContextsLoaded(request_id, layout, result) => {
                self.apply_built_in_layout_contexts(request_id, layout, result)
            }
            _ => Task::none(),
        }
    }

    fn request_built_in_layout(&mut self, layout: BuiltInLayout) -> Task<Message> {
        let symbols = self.built_in_layout_symbols(layout);
        if symbols.len() < BUILT_IN_CHART_COUNT {
            self.push_toast(
                format!(
                    "{} needs at least {BUILT_IN_CHART_COUNT} visible markets",
                    layout.label()
                ),
                true,
            );
            return Task::none();
        }

        let request_id = self.built_in_layout_state.begin_request(layout);
        Task::perform(
            api::fetch_watchlist_contexts_uncached(symbols),
            move |result| Message::BuiltInLayoutContextsLoaded(request_id, layout, result),
        )
    }

    fn apply_built_in_layout_contexts(
        &mut self,
        request_id: u64,
        layout: BuiltInLayout,
        result: Result<api::WatchlistContextsResponse, String>,
    ) -> Task<Message> {
        if !self
            .built_in_layout_state
            .request_is_current(request_id, layout)
        {
            return Task::none();
        }
        self.built_in_layout_state.finish_request();

        let response = match result {
            Ok(response) if response.partial_errors.is_empty() => response,
            Ok(response) => {
                let detail = response.partial_errors.join("; ");
                self.push_toast(
                    format!(
                        "Could not load {} because some market-data sources failed: {}",
                        layout.label(),
                        redact_sensitive_response_text(&detail)
                    ),
                    true,
                );
                return Task::none();
            }
            Err(error) => {
                self.push_toast(
                    format!(
                        "Could not load {}: {}",
                        layout.label(),
                        redact_sensitive_response_text(&error)
                    ),
                    true,
                );
                return Task::none();
            }
        };

        let candidates = self
            .exchange_symbols
            .iter()
            .filter(|symbol| !self.exchange_symbol_is_hidden(symbol))
            .collect::<Vec<_>>();
        let top_symbols =
            top_ranked_symbols(layout, candidates, &response.contexts, BUILT_IN_CHART_COUNT);
        if top_symbols.len() < BUILT_IN_CHART_COUNT {
            self.push_toast(
                format!(
                    "{} needs {} data for at least {BUILT_IN_CHART_COUNT} visible markets",
                    layout.label(),
                    layout.metric_label(),
                ),
                true,
            );
            return Task::none();
        }

        let symbol_keys = top_symbols
            .into_iter()
            .map(|symbol| symbol.key.clone())
            .collect::<Vec<_>>();
        let generated = self.built_in_layout_snapshot(layout, &symbol_keys);
        self.close_chart_header_menus();
        self.active_layout_name = None;
        self.built_in_layout_state.activate(layout);
        let task = self.apply_layout(generated);
        self.persist_config();
        task
    }

    fn built_in_layout_symbols(&self, layout: BuiltInLayout) -> Vec<String> {
        let mut symbols = self
            .exchange_symbols
            .iter()
            .filter(|symbol| symbol.is_user_selectable_market())
            .filter(|symbol| !self.exchange_symbol_is_hidden(symbol))
            .filter(|symbol| layout.supports_symbol(symbol))
            .map(|symbol| symbol.key.clone())
            .collect::<Vec<_>>();
        symbols.sort();
        symbols.dedup();
        symbols
    }

    fn built_in_layout_snapshot(
        &self,
        layout: BuiltInLayout,
        symbol_keys: &[String],
    ) -> config::SavedLayout {
        let timeframe = self.active_timeframe_config_value();
        let mut snapshot = self.saved_layout_snapshot(layout.label().to_string());
        snapshot.pane_layout = Some(layout.preview_layout());
        snapshot.canvases.clear();
        snapshot.layout_ratios.clear();
        snapshot.charts = symbol_keys
            .iter()
            .take(BUILT_IN_CHART_COUNT)
            .enumerate()
            .map(|(id, symbol)| {
                config::ChartConfig::empty(id as u64, symbol.clone(), timeframe.clone())
            })
            .collect();
        snapshot.order_books.clear();
        snapshot.live_watchlists.clear();
        snapshot.positioning_infos.clear();
        snapshot.session_data.clear();
        snapshot.x_feeds.clear();
        snapshot.spaghetti_charts.clear();
        snapshot.widget_padding.overrides.clear();
        if let Some(symbol) = symbol_keys.first() {
            snapshot.active_symbol = symbol.clone();
        }
        snapshot
    }
}

// ---------------------------------------------------------------------------
// Dynamic Ranking And Grid
// ---------------------------------------------------------------------------

fn top_ranked_symbols<'a>(
    layout: BuiltInLayout,
    symbols: impl IntoIterator<Item = &'a ExchangeSymbol>,
    contexts: &HashMap<String, WatchlistContext>,
    limit: usize,
) -> Vec<&'a ExchangeSymbol> {
    let mut seen = HashSet::new();
    // Several dexes can list the same underlying asset (HIP-3); group those
    // contracts by asset and keep the highest-24h-volume one per asset so the
    // grid never shows the same coin twice. Non-perp markets rank individually.
    let mut best_by_asset = HashMap::<String, (f64, f64, &'a ExchangeSymbol)>::new();
    let mut ranked = Vec::new();

    for symbol in symbols {
        if !symbol.is_user_selectable_market()
            || !layout.supports_symbol(symbol)
            || !seen.insert(symbol.key.clone())
        {
            continue;
        }
        let Some(context) = contexts.get(&symbol.key) else {
            continue;
        };
        let Some(metric) = layout.metric_value(context) else {
            continue;
        };
        if !layout.metric_is_rankable(metric) {
            continue;
        }

        let Some(asset) = symbol.underlying_asset() else {
            ranked.push((metric, symbol));
            continue;
        };
        let volume = context.day_vlm.unwrap_or(0.0);
        match best_by_asset.entry(asset.to_string()) {
            Entry::Occupied(mut entry) => {
                let (best_volume, _, _) = *entry.get();
                if volume > best_volume {
                    entry.insert((volume, metric, symbol));
                }
            }
            Entry::Vacant(entry) => {
                entry.insert((volume, metric, symbol));
            }
        }
    }
    ranked.extend(
        best_by_asset
            .into_values()
            .map(|(_, metric, symbol)| (metric, symbol)),
    );

    ranked.sort_by(|(a_metric, a_symbol), (b_metric, b_symbol)| {
        b_metric
            .total_cmp(a_metric)
            .then_with(|| a_symbol.key.cmp(&b_symbol.key))
    });
    ranked
        .into_iter()
        .take(limit)
        .map(|(_, symbol)| symbol)
        .collect()
}

fn top_eight_grid_layout() -> PaneLayoutConfig {
    let columns = [
        chart_column(0, 4),
        chart_column(1, 5),
        chart_column(2, 6),
        chart_column(3, 7),
    ];
    split(
        AxisConfig::Vertical,
        split(AxisConfig::Vertical, columns[0].clone(), columns[1].clone()),
        split(AxisConfig::Vertical, columns[2].clone(), columns[3].clone()),
    )
}

fn chart_column(top_id: u64, bottom_id: u64) -> PaneLayoutConfig {
    split(
        AxisConfig::Horizontal,
        chart_leaf(top_id),
        chart_leaf(bottom_id),
    )
}

fn chart_leaf(chart_id: u64) -> PaneLayoutConfig {
    PaneLayoutConfig::Leaf(PaneKindConfig::Chart { chart_id })
}

fn split(axis: AxisConfig, a: PaneLayoutConfig, b: PaneLayoutConfig) -> PaneLayoutConfig {
    PaneLayoutConfig::Split {
        axis,
        ratio: 0.5,
        a: Box::new(a),
        b: Box::new(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::MarketType;
    use crate::config::KeroseneConfig;

    fn symbol(key: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 2,
            max_leverage: 50,
            only_isolated: false,
            growth_mode: false,
            market_type: MarketType::Perp,
            outcome: None,
        }
    }

    fn context(volume: f64, open_interest_notional: f64) -> WatchlistContext {
        WatchlistContext {
            funding: None,
            prev_day_px: None,
            mark_px: None,
            day_vlm: Some(volume),
            open_interest_notional: Some(open_interest_notional),
        }
    }

    fn gainer_context(previous: f64, mark: f64) -> WatchlistContext {
        gainer_context_with_volume(previous, mark, 0.0)
    }

    fn gainer_context_with_volume(previous: f64, mark: f64, day_vlm: f64) -> WatchlistContext {
        WatchlistContext {
            funding: None,
            prev_day_px: Some(previous),
            mark_px: Some(mark),
            day_vlm: Some(day_vlm),
            open_interest_notional: None,
        }
    }

    #[test]
    fn top_volume_ranking_is_descending_bounded_and_deterministic() {
        let symbols = [symbol("B"), symbol("A"), symbol("C"), symbol("D")];
        let contexts = HashMap::from([
            ("A".to_string(), context(20.0, 1.0)),
            ("B".to_string(), context(20.0, 2.0)),
            ("C".to_string(), context(30.0, 3.0)),
            ("D".to_string(), context(f64::NAN, 4.0)),
        ]);

        let ranked = top_ranked_symbols(BuiltInLayout::TopVolume24h, symbols.iter(), &contexts, 3)
            .into_iter()
            .map(|symbol| symbol.key.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ranked, vec!["C", "A", "B"]);
    }

    #[test]
    fn top_open_interest_ranking_uses_notional_value() {
        let mut spot = symbol("SPOT");
        spot.market_type = MarketType::Spot;
        let symbols = [symbol("BTC"), symbol("ALT"), symbol("HYPE"), spot];
        let contexts = HashMap::from([
            ("BTC".to_string(), context(1.0, 1_000_000.0)),
            ("ALT".to_string(), context(3.0, 50_000.0)),
            ("HYPE".to_string(), context(2.0, 500_000.0)),
            ("SPOT".to_string(), context(4.0, 10_000_000.0)),
        ]);

        let ranked =
            top_ranked_symbols(BuiltInLayout::TopOpenInterest, symbols.iter(), &contexts, 4)
                .into_iter()
                .map(|symbol| symbol.key.as_str())
                .collect::<Vec<_>>();

        assert_eq!(ranked, vec!["BTC", "HYPE", "ALT"]);
    }

    #[test]
    fn top_gainers_ranking_uses_positive_percentage_change_for_perps_only() {
        let mut spot = symbol("SPOT");
        spot.market_type = MarketType::Spot;
        let mut outcome = symbol("OUTCOME");
        outcome.market_type = MarketType::Outcome;
        let symbols = [
            symbol("GAIN_10"),
            symbol("GAIN_50"),
            symbol("FLAT"),
            symbol("LOSS"),
            spot,
            outcome,
        ];
        let contexts = HashMap::from([
            ("GAIN_10".to_string(), gainer_context(100.0, 110.0)),
            ("GAIN_50".to_string(), gainer_context(1.0, 1.5)),
            ("FLAT".to_string(), gainer_context(100.0, 100.0)),
            ("LOSS".to_string(), gainer_context(100.0, 90.0)),
            ("SPOT".to_string(), gainer_context(2.0, 2.4)),
            ("OUTCOME".to_string(), gainer_context(1.0, 10.0)),
        ]);

        let ranked = top_ranked_symbols(BuiltInLayout::TopGainers24h, symbols.iter(), &contexts, 8)
            .into_iter()
            .map(|symbol| symbol.key.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ranked, vec!["GAIN_50", "GAIN_10"]);
    }

    fn hip3_symbol(base: &str, dex_prefix: Option<&str>) -> ExchangeSymbol {
        let key = dex_prefix
            .map(|dex| format!("{dex}:{base}"))
            .unwrap_or_else(|| base.to_string());
        let mut symbol = symbol(&key);
        symbol.ticker = base.to_string();
        symbol
    }

    #[test]
    fn top_gainers_ranking_collapses_hip3_duplicate_contracts_to_highest_volume() {
        let main = hip3_symbol("FARTCOIN", None);
        let hip3 = hip3_symbol("FARTCOIN", Some("builder"));
        let secondary = hip3_symbol("FARTCOIN", Some("other"));
        let symbols = [main, symbol("ALPHA"), hip3, secondary];
        let contexts = HashMap::from([
            (
                "FARTCOIN".to_string(),
                gainer_context_with_volume(1.0, 1.2, 100.0),
            ),
            (
                "builder:FARTCOIN".to_string(),
                gainer_context_with_volume(1.0, 1.6, 1_000.0),
            ),
            (
                "other:FARTCOIN".to_string(),
                gainer_context_with_volume(1.0, 1.4, 50.0),
            ),
            (
                "ALPHA".to_string(),
                gainer_context_with_volume(1.0, 1.5, 500.0),
            ),
        ]);

        let ranked = top_ranked_symbols(BuiltInLayout::TopGainers24h, symbols.iter(), &contexts, 8)
            .into_iter()
            .map(|symbol| symbol.key.as_str())
            .collect::<Vec<_>>();

        // The highest-volume "builder:FARTCOIN" (60% gain) represents the asset,
        // and no other FARTCOIN contract appears.
        assert_eq!(ranked, vec!["builder:FARTCOIN", "ALPHA"]);
    }

    #[test]
    fn top_volume_ranking_keeps_only_the_highest_volume_contract_per_asset() {
        let main = hip3_symbol("TOSHI", None);
        let hip3 = hip3_symbol("TOSHI", Some("builder"));
        let symbols = [main, hip3, symbol("OTHER")];
        let contexts = HashMap::from([
            ("TOSHI".to_string(), context(500.0, 1.0)),
            ("builder:TOSHI".to_string(), context(2_000.0, 2.0)),
            ("OTHER".to_string(), context(1_500.0, 3.0)),
        ]);

        let ranked = top_ranked_symbols(BuiltInLayout::TopVolume24h, symbols.iter(), &contexts, 8)
            .into_iter()
            .map(|symbol| symbol.key.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ranked, vec!["builder:TOSHI", "OTHER"]);
    }

    #[test]
    fn top_eight_grid_is_four_columns_by_two_rows_in_rank_order() {
        let expected = split(
            AxisConfig::Vertical,
            split(AxisConfig::Vertical, chart_column(0, 4), chart_column(1, 5)),
            split(AxisConfig::Vertical, chart_column(2, 6), chart_column(3, 7)),
        );

        assert_eq!(top_eight_grid_layout(), expected);
    }

    #[test]
    fn current_volume_response_replaces_the_workspace_with_ranked_charts() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.exchange_symbols = (0..BUILT_IN_CHART_COUNT)
            .map(|index| symbol(&format!("COIN{index}")))
            .collect();
        let contexts = (0..BUILT_IN_CHART_COUNT)
            .map(|index| {
                (
                    format!("COIN{index}"),
                    context((BUILT_IN_CHART_COUNT - index) as f64, index as f64),
                )
            })
            .collect();
        let layout = BuiltInLayout::TopVolume24h;
        let request_id = terminal.built_in_layout_state.begin_request(layout);

        let _task = terminal.apply_built_in_layout_contexts(
            request_id,
            layout,
            Ok(api::WatchlistContextsResponse::complete(contexts)),
        );

        assert_eq!(terminal.built_in_layout_state.active(), Some(layout));
        assert_eq!(terminal.active_layout_name, None);
        assert_eq!(terminal.charts.len(), BUILT_IN_CHART_COUNT);
        for id in 0..BUILT_IN_CHART_COUNT as u64 {
            assert_eq!(
                terminal.charts.get(&id).map(|chart| chart.symbol.clone()),
                Some(format!("COIN{id}"))
            );
        }
        assert_eq!(terminal.panes.iter().count(), BUILT_IN_CHART_COUNT);
    }

    #[test]
    fn current_open_interest_response_replaces_the_workspace_with_ranked_charts() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.exchange_symbols = (0..BUILT_IN_CHART_COUNT)
            .map(|index| symbol(&format!("COIN{index}")))
            .collect();
        let contexts = (0..BUILT_IN_CHART_COUNT)
            .map(|index| {
                (
                    format!("COIN{index}"),
                    context(index as f64, (index + 1) as f64),
                )
            })
            .collect();
        let layout = BuiltInLayout::TopOpenInterest;
        let request_id = terminal.built_in_layout_state.begin_request(layout);

        let _task = terminal.apply_built_in_layout_contexts(
            request_id,
            layout,
            Ok(api::WatchlistContextsResponse::complete(contexts)),
        );

        assert_eq!(terminal.built_in_layout_state.active(), Some(layout));
        assert_eq!(terminal.charts.len(), BUILT_IN_CHART_COUNT);
        for id in 0..BUILT_IN_CHART_COUNT as u64 {
            assert_eq!(
                terminal.charts.get(&id).map(|chart| chart.symbol.clone()),
                Some(format!("COIN{}", BUILT_IN_CHART_COUNT - 1 - id as usize))
            );
        }
    }

    #[test]
    fn current_gainers_response_replaces_the_workspace_with_ranked_charts() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.exchange_symbols = (0..BUILT_IN_CHART_COUNT)
            .map(|index| symbol(&format!("COIN{index}")))
            .collect();
        let contexts = (0..BUILT_IN_CHART_COUNT)
            .map(|index| {
                (
                    format!("COIN{index}"),
                    gainer_context(100.0, 101.0 + index as f64),
                )
            })
            .collect();
        let layout = BuiltInLayout::TopGainers24h;
        let request_id = terminal.built_in_layout_state.begin_request(layout);

        let _task = terminal.apply_built_in_layout_contexts(
            request_id,
            layout,
            Ok(api::WatchlistContextsResponse::complete(contexts)),
        );

        assert_eq!(terminal.built_in_layout_state.active(), Some(layout));
        assert_eq!(terminal.charts.len(), BUILT_IN_CHART_COUNT);
        for id in 0..BUILT_IN_CHART_COUNT as u64 {
            assert_eq!(
                terminal.charts.get(&id).map(|chart| chart.symbol.clone()),
                Some(format!("COIN{}", BUILT_IN_CHART_COUNT - 1 - id as usize))
            );
        }
    }

    #[test]
    fn stale_volume_response_cannot_replace_a_newer_request() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let layout = BuiltInLayout::TopVolume24h;
        let stale_request_id = terminal.built_in_layout_state.begin_request(layout);
        let current_request_id = terminal.built_in_layout_state.begin_request(layout);
        let original_chart_symbols = terminal
            .charts
            .iter()
            .map(|(id, chart)| (*id, chart.symbol.clone()))
            .collect::<HashMap<_, _>>();

        let _task = terminal.apply_built_in_layout_contexts(
            stale_request_id,
            layout,
            Err("stale failure".to_string()),
        );

        assert!(terminal.built_in_layout_state.is_loading(layout));
        assert_eq!(
            terminal.built_in_layout_state.request_id,
            current_request_id
        );
        assert_eq!(
            terminal
                .charts
                .iter()
                .map(|(id, chart)| (*id, chart.symbol.clone()))
                .collect::<HashMap<_, _>>(),
            original_chart_symbols
        );
    }
}
