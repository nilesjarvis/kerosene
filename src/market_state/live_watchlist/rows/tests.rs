use crate::api::WatchlistContext;
use crate::api::{MarketType, OutcomeSymbolInfo};

use super::*;

fn outcome_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: "OUT95-YES".to_string(),
        category: "outcome".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 95,
            question_id: None,
            question_name: Some("Will BTC close green?".to_string()),
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
            outcome_name: "Recurring".to_string(),
            description: "Will BTC close green?".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDH".to_string(),
            quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
            encoding: 950,
        }),
    }
}

fn watchlist(symbols: &[&str]) -> LiveWatchlistInstance {
    LiveWatchlistInstance {
        id: 1,
        symbols: symbols.iter().map(|symbol| (*symbol).to_string()).collect(),
        search_query: String::new(),
        sort_column: config::LiveWatchlistSortColumn::Symbol,
        sort_direction: config::SortDirection::Ascending,
        visible_columns: Vec::new(),
        row_cache: Vec::new(),
    }
}

fn row(symbol: &str, display: &str, mid_px: Option<f64>) -> LiveWatchlistRowData {
    LiveWatchlistRowData {
        sym_key: symbol.to_string(),
        display: display.to_string(),
        mid_px,
        pct_5m: None,
        pct_30m: None,
        pct_1h: None,
        pct_24h: None,
        funding: None,
    }
}

fn context(prev_day_px: f64) -> WatchlistContext {
    WatchlistContext {
        funding: None,
        prev_day_px: Some(prev_day_px),
        day_vlm: None,
    }
}

#[test]
fn watchlist_rows_resolve_outcome_display_through_canonical_label() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols.push(outcome_symbol("#950"));

    let rows = terminal.live_watchlist_rows_for(&watchlist(&["#950"]));

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].display, "YES: Will BTC close green?");
}

#[test]
fn watchlist_rows_fall_back_to_cached_outcome_label_for_unloaded_keys() {
    let mut terminal = TradingTerminal::boot().0;
    terminal
        .outcome_display_labels
        .insert("#950".to_string(), "YES: Will BTC close green?".to_string());

    let rows = terminal.live_watchlist_rows_for(&watchlist(&["#950"]));

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].display, "YES: Will BTC close green?");
}

#[test]
fn watchlist_percent_change_requires_current_and_previous_prices() {
    assert_eq!(percent_change(Some(110.0), Some(100.0)), Some(10.0));
    assert_eq!(percent_change(None, Some(100.0)), None);
    assert_eq!(percent_change(Some(110.0), None), None);
    assert_eq!(percent_change(Some(110.0), Some(0.0)), None);
    assert_eq!(percent_change(Some(f64::NAN), Some(100.0)), None);
    assert_eq!(percent_change(Some(100.0), Some(f64::INFINITY)), None);
}

#[test]
fn watchlist_rows_do_not_borrow_native_context_for_prefixed_hip3_symbol() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols.push(ExchangeSymbol {
        key: "xyz:NVDA".to_string(),
        ticker: "NVDA".to_string(),
        category: "perp".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 10,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
    });
    terminal.all_mids.insert("xyz:NVDA".to_string(), 110.0);
    terminal
        .all_mids_updated_at_ms
        .insert("xyz:NVDA".to_string(), crate::ws::now_ms());
    terminal
        .live_watchlist_ctxs
        .insert("NVDA".to_string(), context(100.0));

    let rows = terminal.live_watchlist_rows_for(&watchlist(&["xyz:NVDA"]));

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].pct_24h, None);
}

#[test]
fn watchlist_price_sort_puts_missing_prices_last_in_ascending_order() {
    assert_eq!(sortable_cmp(Some(10.0), Some(20.0), false), Ordering::Less);
    assert_eq!(
        sortable_cmp(Some(10.0), Some(20.0), true),
        Ordering::Greater
    );
    assert_eq!(sortable_cmp(Some(10.0), None, true), Ordering::Less);
    assert_eq!(sortable_cmp(None, Some(10.0), true), Ordering::Greater);
}

#[test]
fn sorted_rows_use_requested_column_and_direction() {
    let rows = vec![
        row("BTC", "Bitcoin", Some(10.0)),
        row("ETH", "Ethereum", None),
        row("SOL", "Solana", Some(20.0)),
    ];

    let sorted = sort_live_watchlist_rows(
        rows,
        config::LiveWatchlistSortColumn::Price,
        config::SortDirection::Descending,
    );

    assert_eq!(
        sorted
            .iter()
            .map(|row| row.sym_key.as_str())
            .collect::<Vec<_>>(),
        vec!["SOL", "BTC", "ETH"]
    );
}
