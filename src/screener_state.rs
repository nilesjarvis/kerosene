use crate::api::{ExchangeSymbol, MarketType, WatchlistContext};
use crate::app_state::TradingTerminal;
use crate::config::SortDirection;
use crate::helpers::positive_percent_change as percent_change;

use iced::window;
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};

// ---------------------------------------------------------------------------
// Screener State
// ---------------------------------------------------------------------------

pub(crate) const SCREENER_CONTEXT_REFRESH_MS: u64 = 60_000;
pub(crate) const SCREENER_HISTORY_BATCH_SIZE: usize = 10;
pub(crate) const SCREENER_HISTORY_REFRESH_MS: u64 = 15_000;
const SCREENER_SAMPLE_INTERVAL_MS: u64 = 60_000;
const SCREENER_SAMPLE_RETENTION_MS: u64 = 65 * 60_000;

#[derive(Debug, Clone)]
pub(crate) struct ScreenerState {
    pub(crate) window_id: Option<window::Id>,
    pub(crate) contexts: HashMap<String, WatchlistContext>,
    pub(crate) contexts_loading: bool,
    pub(crate) contexts_request_id: u64,
    pub(crate) contexts_request_symbols: Vec<String>,
    pub(crate) contexts_refresh_pending: bool,
    pub(crate) contexts_last_fetch_ms: Option<u64>,
    pub(crate) history: HashMap<String, (f64, f64)>,
    pub(crate) history_loaded_at: HashMap<String, u64>,
    pub(crate) history_loading: bool,
    pub(crate) history_request_id: u64,
    pub(crate) history_request_symbols: Vec<String>,
    pub(crate) history_refresh_pending: bool,
    pub(crate) history_last_fetch_ms: Option<u64>,
    pub(crate) status: Option<(String, bool)>,
    pub(crate) exchange_filter: ScreenerExchangeFilter,
    pub(crate) sort_column: ScreenerSortColumn,
    pub(crate) sort_direction: SortDirection,
    samples: HashMap<String, VecDeque<ScreenerPriceSample>>,
}

impl Default for ScreenerState {
    fn default() -> Self {
        Self {
            window_id: None,
            contexts: HashMap::new(),
            contexts_loading: false,
            contexts_request_id: 0,
            contexts_request_symbols: Vec::new(),
            contexts_refresh_pending: false,
            contexts_last_fetch_ms: None,
            history: HashMap::new(),
            history_loaded_at: HashMap::new(),
            history_loading: false,
            history_request_id: 0,
            history_request_symbols: Vec::new(),
            history_refresh_pending: false,
            history_last_fetch_ms: None,
            status: None,
            exchange_filter: ScreenerExchangeFilter::default(),
            sort_column: ScreenerSortColumn::Volume,
            sort_direction: SortDirection::Descending,
            samples: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum ScreenerExchangeFilter {
    #[default]
    AllMarkets,
    AllHip3,
    Hip3Dex(String),
}

impl ScreenerExchangeFilter {
    pub(crate) fn normalized(self) -> Self {
        match self {
            Self::AllMarkets => Self::AllMarkets,
            Self::AllHip3 => Self::AllHip3,
            Self::Hip3Dex(dex) => {
                let dex = dex.trim().to_ascii_lowercase();
                if dex.is_empty() {
                    Self::AllMarkets
                } else {
                    Self::Hip3Dex(dex)
                }
            }
        }
    }
}

impl std::fmt::Display for ScreenerExchangeFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllMarkets => f.write_str("All Markets"),
            Self::AllHip3 => f.write_str("All HIP-3"),
            Self::Hip3Dex(dex) => write!(f, "HIP-3: {dex}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ScreenerSortColumn {
    Symbol,
    Price,
    Change24h,
    Change1h,
    Change15m,
    #[default]
    Volume,
    Funding,
}

#[derive(Debug, Clone, Copy)]
struct ScreenerPriceSample {
    timestamp_ms: u64,
    price: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct ScreenerRow {
    pub(crate) symbol_key: String,
    pub(crate) display: String,
    pub(crate) price: Option<f64>,
    pub(crate) pct_24h: Option<f64>,
    pub(crate) pct_1h: Option<f64>,
    pub(crate) pct_15m: Option<f64>,
    pub(crate) volume_24h: Option<f64>,
    pub(crate) funding: Option<f64>,
}

impl ScreenerState {
    pub(crate) fn invalidate_refreshes(&mut self) {
        self.invalidate_context_refresh();
        self.invalidate_history_refresh();
    }

    pub(crate) fn invalidate_context_refresh(&mut self) {
        self.contexts_request_id = self.contexts_request_id.saturating_add(1);
        self.contexts_request_symbols.clear();
        self.contexts_refresh_pending = false;
        self.contexts_loading = false;
    }

    pub(crate) fn invalidate_history_refresh(&mut self) {
        self.history_request_id = self.history_request_id.saturating_add(1);
        self.history_request_symbols.clear();
        self.history_refresh_pending = false;
        self.history_loading = false;
    }

    pub(crate) fn set_exchange_filter(&mut self, filter: ScreenerExchangeFilter) -> bool {
        let filter = filter.normalized();
        if self.exchange_filter == filter {
            return false;
        }
        self.exchange_filter = filter;
        true
    }

    pub(crate) fn apply_sort_change(&mut self, column: ScreenerSortColumn) {
        if self.sort_column == column {
            self.sort_direction = match self.sort_direction {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.sort_column = column;
            self.sort_direction = SortDirection::Descending;
        }
    }

    pub(crate) fn record_mid_samples(&mut self, mids: &HashMap<String, f64>, now_ms: u64) {
        let cutoff = now_ms.saturating_sub(SCREENER_SAMPLE_RETENTION_MS);

        for (symbol, price) in mids {
            if !price.is_finite() || *price <= 0.0 {
                continue;
            }

            let samples = self.samples.entry(symbol.clone()).or_default();
            let should_sample = samples
                .back()
                .map(|sample| {
                    now_ms.saturating_sub(sample.timestamp_ms) >= SCREENER_SAMPLE_INTERVAL_MS
                })
                .unwrap_or(true);

            if should_sample {
                samples.push_back(ScreenerPriceSample {
                    timestamp_ms: now_ms,
                    price: *price,
                });
            }

            while samples
                .front()
                .is_some_and(|sample| sample.timestamp_ms < cutoff)
            {
                samples.pop_front();
            }
        }
    }

    fn baseline_for_candidates(
        &self,
        candidates: &[String],
        minutes_ago: u64,
        now_ms: u64,
    ) -> Option<f64> {
        let lookback_ms = minutes_ago * 60_000;
        if now_ms < lookback_ms {
            return None;
        }
        let target_ms = now_ms - lookback_ms;

        candidates.iter().find_map(|candidate| {
            self.samples
                .get(candidate)
                .and_then(|samples| sample_at_or_before(samples, target_ms))
        })
    }

    /// Most recent recorded mid at or before `target_ms` across any of the
    /// candidate mid keys. Used to anchor a Telegram ticker's price-impact
    /// baseline to the message publication time rather than to whenever the app
    /// first noticed the post.
    pub(crate) fn mid_sample_at_or_before(
        &self,
        candidates: &[String],
        target_ms: u64,
    ) -> Option<f64> {
        candidates.iter().find_map(|candidate| {
            self.samples
                .get(candidate)
                .and_then(|samples| sample_at_or_before(samples, target_ms))
        })
    }

    /// Recorded mids at or after `from_ms`, in chronological order, for the first
    /// candidate key that has any. Used to shape the Telegram impact-chip
    /// sparkline over the window since a post was first seen. Returns an empty
    /// vec when no samples fall inside the window.
    pub(crate) fn mid_samples_since(&self, candidates: &[String], from_ms: u64) -> Vec<f32> {
        candidates
            .iter()
            .find_map(|candidate| {
                let samples = self.samples.get(candidate)?;
                let values: Vec<f32> = samples
                    .iter()
                    .filter(|sample| sample.timestamp_ms >= from_ms)
                    .filter(|sample| sample.price.is_finite() && sample.price > 0.0)
                    .map(|sample| sample.price as f32)
                    .collect();
                (!values.is_empty()).then_some(values)
            })
            .unwrap_or_default()
    }
}

impl TradingTerminal {
    pub(crate) fn record_screener_mid_samples(&mut self, mids: &HashMap<String, f64>, now_ms: u64) {
        self.screener.record_mid_samples(mids, now_ms);
    }

    pub(crate) fn screener_symbols(&self) -> Vec<&ExchangeSymbol> {
        self.exchange_symbols
            .iter()
            .filter(|symbol| symbol.is_user_selectable_market())
            .filter(|symbol| !self.exchange_symbol_is_hidden(symbol))
            .filter(|symbol| {
                screener_exchange_filter_matches(symbol, &self.screener.exchange_filter)
            })
            .collect()
    }

    pub(crate) fn screener_exchange_filter_options(&self) -> Vec<ScreenerExchangeFilter> {
        let mut dexes = std::collections::BTreeSet::new();
        for symbol in &self.exchange_symbols {
            if symbol.is_user_selectable_market()
                && !self.exchange_symbol_is_hidden(symbol)
                && symbol.market_type == MarketType::Perp
                && let Some((dex, _)) = symbol.key.split_once(':')
            {
                dexes.insert(dex.to_ascii_lowercase());
            }
        }

        let mut options = vec![ScreenerExchangeFilter::AllMarkets];
        if !dexes.is_empty() {
            options.push(ScreenerExchangeFilter::AllHip3);
            options.extend(dexes.into_iter().map(ScreenerExchangeFilter::Hip3Dex));
        }
        if !options.contains(&self.screener.exchange_filter) {
            options.push(self.screener.exchange_filter.clone());
        }
        options
    }

    pub(crate) fn screener_symbol_keys(&self) -> Vec<String> {
        self.screener_symbols()
            .into_iter()
            .map(|symbol| symbol.key.clone())
            .collect()
    }

    pub(crate) fn screener_history_symbol_keys(&self, now_ms: u64, force: bool) -> Vec<String> {
        let mut symbols = self
            .screener_symbols()
            .into_iter()
            .filter(|symbol| symbol.market_type == MarketType::Perp)
            .filter(|symbol| force || !self.screener.history_loaded_at.contains_key(&symbol.key))
            .filter(|symbol| {
                if force {
                    return true;
                }
                let candidates = self.mid_candidates_for_symbol(&symbol.key);
                self.screener
                    .baseline_for_candidates(&candidates, 60, now_ms)
                    .is_none()
            })
            .collect::<Vec<_>>();

        symbols.sort_by(|a, b| {
            let a_volume = self
                .screener_context_for_symbol(a)
                .and_then(|context| context.day_vlm);
            let b_volume = self
                .screener_context_for_symbol(b)
                .and_then(|context| context.day_vlm);

            sortable_cmp(a_volume, b_volume, true).then(
                Self::exchange_symbol_display_name(a)
                    .cmp(&Self::exchange_symbol_display_name(b))
                    .then(a.key.cmp(&b.key)),
            )
        });

        symbols
            .into_iter()
            .take(SCREENER_HISTORY_BATCH_SIZE)
            .map(|symbol| symbol.key.clone())
            .collect()
    }

    pub(crate) fn screener_rows(&self) -> Vec<ScreenerRow> {
        let now_ms = Self::now_ms();
        let rows = self
            .screener_symbols()
            .into_iter()
            .map(|symbol| self.screener_row(symbol, now_ms))
            .collect::<Vec<_>>();

        sort_screener_rows(
            rows,
            self.screener.sort_column,
            self.screener.sort_direction,
        )
    }

    fn screener_row(&self, symbol: &ExchangeSymbol, now_ms: u64) -> ScreenerRow {
        let price = self.resolve_mid_for_symbol(&symbol.key);
        let context = self.screener_context_for_symbol(symbol);
        let candidates = self.mid_candidates_for_symbol(&symbol.key);
        let history = self.screener.history.get(&symbol.key).copied();
        let baseline_15m = self
            .screener
            .baseline_for_candidates(&candidates, 15, now_ms)
            .or_else(|| history.map(|(price_15m, _)| price_15m));
        let baseline_1h = self
            .screener
            .baseline_for_candidates(&candidates, 60, now_ms)
            .or_else(|| history.map(|(_, price_1h)| price_1h));
        let display = Self::exchange_symbol_display_name(symbol);

        ScreenerRow {
            symbol_key: symbol.key.clone(),
            display,
            price,
            pct_24h: context
                .and_then(|context| context.prev_day_px)
                .and_then(|previous| percent_change(price, Some(previous))),
            pct_1h: percent_change(price, baseline_1h),
            pct_15m: percent_change(price, baseline_15m),
            volume_24h: context.and_then(|context| context.day_vlm),
            funding: context.and_then(|context| context.funding),
        }
    }

    fn screener_context_for_symbol(&self, symbol: &ExchangeSymbol) -> Option<&WatchlistContext> {
        self.screener.contexts.get(&symbol.key).or_else(|| {
            (symbol.key == symbol.ticker)
                .then(|| self.screener.contexts.get(&symbol.ticker))
                .flatten()
        })
    }
}

fn sample_at_or_before(samples: &VecDeque<ScreenerPriceSample>, target_ms: u64) -> Option<f64> {
    samples
        .iter()
        .rev()
        .find(|sample| sample.timestamp_ms <= target_ms)
        .map(|sample| sample.price)
}

fn sort_screener_rows(
    mut rows: Vec<ScreenerRow>,
    sort_column: ScreenerSortColumn,
    sort_direction: SortDirection,
) -> Vec<ScreenerRow> {
    let descending = sort_direction == SortDirection::Descending;
    rows.sort_by(|a, b| {
        let primary = match sort_column {
            ScreenerSortColumn::Symbol => {
                let cmp = a
                    .display
                    .cmp(&b.display)
                    .then(a.symbol_key.cmp(&b.symbol_key));
                if descending { cmp.reverse() } else { cmp }
            }
            ScreenerSortColumn::Price => sortable_cmp(a.price, b.price, descending),
            ScreenerSortColumn::Change24h => sortable_cmp(a.pct_24h, b.pct_24h, descending),
            ScreenerSortColumn::Change1h => sortable_cmp(a.pct_1h, b.pct_1h, descending),
            ScreenerSortColumn::Change15m => sortable_cmp(a.pct_15m, b.pct_15m, descending),
            ScreenerSortColumn::Volume => sortable_cmp(a.volume_24h, b.volume_24h, descending),
            ScreenerSortColumn::Funding => sortable_cmp(a.funding, b.funding, descending),
        };

        primary.then(
            a.display
                .cmp(&b.display)
                .then(a.symbol_key.cmp(&b.symbol_key)),
        )
    });
    rows
}

fn sortable_cmp(a: Option<f64>, b: Option<f64>, descending: bool) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => {
            let cmp = a.partial_cmp(&b).unwrap_or(Ordering::Equal);
            if descending { cmp.reverse() } else { cmp }
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn screener_exchange_filter_matches(
    symbol: &ExchangeSymbol,
    filter: &ScreenerExchangeFilter,
) -> bool {
    if symbol.market_type == MarketType::Spot {
        return false;
    }

    match filter {
        ScreenerExchangeFilter::AllMarkets => true,
        ScreenerExchangeFilter::AllHip3 => {
            symbol.market_type == MarketType::Perp && symbol.key.contains(':')
        }
        ScreenerExchangeFilter::Hip3Dex(selected_dex) => {
            symbol.market_type == MarketType::Perp
                && symbol
                    .key
                    .split_once(':')
                    .is_some_and(|(dex, _)| dex.eq_ignore_ascii_case(selected_dex))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_record_once_per_interval_and_prune_old_values() {
        let mut state = ScreenerState::default();
        let mut mids = HashMap::from([("BTC".to_string(), 100.0)]);

        state.record_mid_samples(&mids, 1_000);
        mids.insert("BTC".to_string(), 101.0);
        state.record_mid_samples(&mids, 30_000);
        mids.insert("BTC".to_string(), 102.0);
        state.record_mid_samples(&mids, 61_000);

        let samples = state.samples.get("BTC").expect("BTC samples");
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].price, 100.0);
        assert_eq!(samples[1].price, 102.0);

        state.record_mid_samples(&mids, 70 * 60_000);
        let samples = state.samples.get("BTC").expect("BTC samples");
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].price, 102.0);
    }

    #[test]
    fn mid_samples_since_returns_window_in_order_for_first_candidate() {
        let mut state = ScreenerState::default();
        state.record_mid_samples(&HashMap::from([("BTC".to_string(), 100.0)]), 0);
        state.record_mid_samples(&HashMap::from([("BTC".to_string(), 110.0)]), 60_000);
        state.record_mid_samples(&HashMap::from([("BTC".to_string(), 120.0)]), 120_000);

        let candidates = vec!["MISSING".to_string(), "BTC".to_string()];
        // Only samples at or after the window start are returned, oldest first.
        assert_eq!(
            state.mid_samples_since(&candidates, 60_000),
            vec![110.0_f32, 120.0_f32]
        );
        // A window past every sample yields nothing.
        assert!(state.mid_samples_since(&candidates, 200_000).is_empty());
        // Unknown candidates yield nothing.
        assert!(state.mid_samples_since(&["NONE".to_string()], 0).is_empty());
    }

    #[test]
    fn baseline_requires_a_sample_at_or_before_target() {
        let mut state = ScreenerState::default();
        state.record_mid_samples(&HashMap::from([("BTC".to_string(), 100.0)]), 0);
        state.record_mid_samples(&HashMap::from([("BTC".to_string(), 110.0)]), 60_000);

        let candidates = vec!["BTC".to_string()];
        assert_eq!(
            state.baseline_for_candidates(&candidates, 1, 2 * 60_000),
            Some(110.0)
        );
        assert_eq!(
            state.baseline_for_candidates(&candidates, 15, 14 * 60_000),
            None
        );
    }

    #[test]
    fn screener_rows_sort_numeric_columns_with_missing_values_last() {
        let rows = vec![
            row("BTC", "BTC", Some(100.0), None),
            row("ETH", "ETH", Some(50.0), Some(0.01)),
            row("SOL", "SOL", None, Some(-0.01)),
        ];

        let sorted = sort_screener_rows(rows, ScreenerSortColumn::Price, SortDirection::Descending);

        assert_eq!(
            sorted
                .iter()
                .map(|row| row.symbol_key.as_str())
                .collect::<Vec<_>>(),
            vec!["BTC", "ETH", "SOL"]
        );
    }

    #[test]
    fn screener_sort_change_toggles_current_column() {
        let mut state = ScreenerState::default();

        state.apply_sort_change(ScreenerSortColumn::Price);
        assert_eq!(state.sort_column, ScreenerSortColumn::Price);
        assert_eq!(state.sort_direction, SortDirection::Descending);

        state.apply_sort_change(ScreenerSortColumn::Price);
        assert_eq!(state.sort_direction, SortDirection::Ascending);
    }

    #[test]
    fn screener_default_sort_is_highest_volume_first() {
        let state = ScreenerState::default();

        assert_eq!(state.sort_column, ScreenerSortColumn::Volume);
        assert_eq!(state.sort_direction, SortDirection::Descending);
    }

    #[test]
    fn screener_exchange_filter_matches_specific_hip3_dex() {
        let native = symbol("BTC", MarketType::Perp);
        let spot = symbol("@107", MarketType::Spot);
        let flx = symbol("flx:BTC", MarketType::Perp);
        let xyz = symbol("xyz:NVDA", MarketType::Perp);

        assert!(!screener_exchange_filter_matches(
            &spot,
            &ScreenerExchangeFilter::AllMarkets
        ));
        assert!(screener_exchange_filter_matches(
            &flx,
            &ScreenerExchangeFilter::AllHip3
        ));
        assert!(!screener_exchange_filter_matches(
            &native,
            &ScreenerExchangeFilter::AllHip3
        ));
        assert!(screener_exchange_filter_matches(
            &flx,
            &ScreenerExchangeFilter::Hip3Dex("flx".to_string())
        ));
        assert!(!screener_exchange_filter_matches(
            &xyz,
            &ScreenerExchangeFilter::Hip3Dex("flx".to_string())
        ));
    }

    #[test]
    fn screener_rows_do_not_borrow_native_context_for_prefixed_hip3_symbol() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.exchange_symbols = vec![symbol("xyz:NVDA", MarketType::Perp)];
        terminal.all_mids.insert("xyz:NVDA".to_string(), 110.0);
        terminal
            .all_mids_updated_at_ms
            .insert("xyz:NVDA".to_string(), crate::ws::now_ms());
        terminal.screener.contexts.insert(
            "NVDA".to_string(),
            WatchlistContext {
                funding: None,
                prev_day_px: Some(100.0),
                day_vlm: Some(1_000.0),
            },
        );

        let rows = terminal.screener_rows();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pct_24h, None);
        assert_eq!(rows[0].volume_24h, None);
    }

    fn row(
        symbol_key: &str,
        display: &str,
        price: Option<f64>,
        funding: Option<f64>,
    ) -> ScreenerRow {
        ScreenerRow {
            symbol_key: symbol_key.to_string(),
            display: display.to_string(),
            price,
            pct_24h: None,
            pct_1h: None,
            pct_15m: None,
            volume_24h: None,
            funding,
        }
    }

    fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key
                .split_once(':')
                .map(|(_, ticker)| ticker)
                .unwrap_or(key)
                .to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 2,
            max_leverage: 1,
            only_isolated: false,
            market_type,
            outcome: None,
        }
    }
}
