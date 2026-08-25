use crate::api::{self, ExchangeSymbol, WatchlistContext};
use crate::app_state::TradingTerminal;
use crate::config::{self, AxisConfig, PaneKindConfig, PaneLayoutConfig};
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use iced::Task;
use std::collections::{HashMap, HashSet};

const BUILT_IN_CHART_COUNT: usize = 8;

// ---------------------------------------------------------------------------
// Built-In Layout State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BuiltInLayout {
    TopVolume24h,
    TopOpenInterest,
}

impl BuiltInLayout {
    pub(crate) const ALL: [Self; 2] = [Self::TopVolume24h, Self::TopOpenInterest];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::TopVolume24h => "Top 8 by 24h Volume",
            Self::TopOpenInterest => "Top 8 by Open Interest",
        }
    }

    pub(crate) fn kind_label(self) -> &'static str {
        match self {
            Self::TopVolume24h | Self::TopOpenInterest => "Dynamic",
        }
    }

    pub(crate) fn loading_label(self) -> &'static str {
        match self {
            Self::TopVolume24h => "Refreshing 24h volumes...",
            Self::TopOpenInterest => "Refreshing open interest...",
        }
    }

    pub(crate) fn preview_layout(self) -> PaneLayoutConfig {
        match self {
            Self::TopVolume24h | Self::TopOpenInterest => top_eight_grid_layout(),
        }
    }

    fn supports_symbol(self, symbol: &ExchangeSymbol) -> bool {
        match self {
            // Outcome-market volume is calculated from candles on its own refresh
            // path; the exchange context endpoint used here covers perp and spot.
            Self::TopVolume24h => symbol.market_type != api::MarketType::Outcome,
            Self::TopOpenInterest => symbol.market_type == api::MarketType::Perp,
        }
    }

    fn metric_value(self, context: &WatchlistContext) -> Option<f64> {
        match self {
            Self::TopVolume24h => context.day_vlm,
            Self::TopOpenInterest => context.open_interest_notional,
        }
    }

    fn metric_label(self) -> &'static str {
        match self {
            Self::TopVolume24h => "24h-volume",
            Self::TopOpenInterest => "open-interest",
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
    let mut ranked = symbols
        .into_iter()
        .filter(|symbol| symbol.is_user_selectable_market())
        .filter(|symbol| layout.supports_symbol(symbol))
        .filter(|symbol| seen.insert(symbol.key.clone()))
        .filter_map(|symbol| {
            let metric = layout.metric_value(contexts.get(&symbol.key)?)?;
            (metric.is_finite() && metric >= 0.0).then_some((metric, symbol))
        })
        .collect::<Vec<_>>();
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
            market_type: MarketType::Perp,
            outcome: None,
        }
    }

    fn context(volume: f64, open_interest_notional: f64) -> WatchlistContext {
        WatchlistContext {
            funding: None,
            prev_day_px: None,
            day_vlm: Some(volume),
            open_interest_notional: Some(open_interest_notional),
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
