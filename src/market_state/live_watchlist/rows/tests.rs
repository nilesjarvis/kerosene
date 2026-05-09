use super::*;

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

#[test]
fn watchlist_percent_change_requires_current_and_previous_prices() {
    assert_eq!(percent_change(Some(110.0), Some(100.0)), Some(10.0));
    assert_eq!(percent_change(None, Some(100.0)), None);
    assert_eq!(percent_change(Some(110.0), None), None);
    assert_eq!(percent_change(Some(110.0), Some(0.0)), None);
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
