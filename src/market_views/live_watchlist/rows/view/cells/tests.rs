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
        ema_distance: None,
        ema_status: None,
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

#[test]
fn ema_distance_cell_formats_signed_percent_and_missing_values() {
    for (distance, expected, color) in [
        (Some(12.345), "+12.35%", Theme::Dark.palette().success),
        (Some(-2.5), "-2.50%", Theme::Dark.palette().danger),
        (None, "-", Theme::Dark.palette().text),
    ] {
        let mut data = row_data(Some(100.0));
        data.ema_distance = distance;
        let (value, actual_color) = live_watchlist_column_value(
            &config::LiveWatchlistColumn::EmaDistance,
            &data,
            &DisplayDenominationContext::default(),
            Color::WHITE,
            &Theme::Dark,
        );
        assert_eq!(value, expected);
        assert_eq!(actual_color, color);
    }
}
