use super::*;

fn row_data(mid_px: Option<f64>) -> LiveWatchlistRowData {
    LiveWatchlistRowData {
        sym_key: "BTC".to_string(),
        display: "BTC".to_string(),
        mid_px,
        pct_5m: None,
        pct_30m: None,
        pct_1h: None,
        pct_24h: None,
        funding: None,
    }
}

#[test]
fn watchlist_price_cell_marks_missing_mid_unavailable() {
    let (value, _) = live_watchlist_column_value(
        &config::LiveWatchlistColumn::Price,
        &row_data(None),
        &DisplayDenominationContext::default(),
        Color::WHITE,
        &Theme::Dark,
    );
    assert_eq!(value, "-");

    let (value, _) = live_watchlist_column_value(
        &config::LiveWatchlistColumn::Price,
        &row_data(Some(123.45)),
        &DisplayDenominationContext::default(),
        Color::WHITE,
        &Theme::Dark,
    );
    assert_eq!(value, "123.45");
}
