use crate::config::LiveWatchlistColumn;

use super::*;

#[test]
fn ordered_columns_use_canonical_config_order() {
    let columns = vec![
        LiveWatchlistColumn::Funding,
        LiveWatchlistColumn::Price,
        LiveWatchlistColumn::Change1h,
    ];

    let ordered = TradingTerminal::ordered_live_watchlist_columns(&columns);

    assert_eq!(
        ordered,
        vec![
            LiveWatchlistColumn::Price,
            LiveWatchlistColumn::Change1h,
            LiveWatchlistColumn::Funding,
        ]
    );
}

#[test]
fn columns_for_width_drops_trailing_columns_until_the_row_fits() {
    let columns = LiveWatchlistColumn::ALL.to_vec();

    let visible = TradingTerminal::live_watchlist_columns_for_width(&columns, 300.0);

    assert_eq!(
        visible,
        vec![LiveWatchlistColumn::Price, LiveWatchlistColumn::Change5m]
    );
}

#[test]
fn columns_for_width_can_drop_every_optional_column() {
    let columns = LiveWatchlistColumn::ALL.to_vec();

    let visible = TradingTerminal::live_watchlist_columns_for_width(&columns, 120.0);

    assert!(visible.is_empty());
}

#[test]
fn columns_for_width_uses_default_width_for_invalid_values() {
    let columns = LiveWatchlistColumn::ALL.to_vec();

    let visible = TradingTerminal::live_watchlist_columns_for_width(&columns, f32::NAN);

    assert_eq!(visible, columns);
}
