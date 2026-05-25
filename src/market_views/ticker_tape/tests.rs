use crate::denomination::DisplayDenominationContext;

use super::formatting::{
    TickerTapeItem, percent_change, percent_label, price_label, ticker_tape_item_width,
};
use super::{TICKER_TAPE_ITEM_MAX_WIDTH, TICKER_TAPE_ITEM_MIN_WIDTH};

// ---------------------------------------------------------------------------
// Ticker Tape Formatting Tests
// ---------------------------------------------------------------------------

#[test]
fn percent_change_preserves_signed_relative_move() {
    assert_eq!(percent_change(Some(110.0), Some(100.0)), Some(10.0));
    assert_eq!(percent_change(Some(75.0), Some(100.0)), Some(-25.0));
}

#[test]
fn percent_change_requires_positive_current_and_previous_price() {
    assert_eq!(percent_change(None, Some(100.0)), None);
    assert_eq!(percent_change(Some(100.0), None), None);
    assert_eq!(percent_change(Some(0.0), Some(100.0)), None);
    assert_eq!(percent_change(Some(100.0), Some(0.0)), None);
    assert_eq!(percent_change(Some(f64::NAN), Some(100.0)), None);
    assert_eq!(percent_change(Some(100.0), Some(f64::INFINITY)), None);
}

#[test]
fn ticker_tape_labels_use_existing_placeholder_and_sign_conventions() {
    let denomination = DisplayDenominationContext::usd();

    assert_eq!(price_label(None, &denomination), "-");
    assert_eq!(percent_label(None), "-");
    assert_eq!(percent_label(Some(1.234)), "+1.23%");
    assert_eq!(percent_label(Some(-1.234)), "-1.23%");
}

#[test]
fn ticker_tape_item_width_stays_within_layout_bounds() {
    let denomination = DisplayDenominationContext::usd();
    let narrow_item = TickerTapeItem {
        symbol: "BTC".to_string(),
        ticker: "BTC".to_string(),
        price: None,
        pct_24h: None,
    };
    let wide_item = TickerTapeItem {
        symbol: "VERY-LONG-SYMBOL-NAME".to_string(),
        ticker: "VERY-LONG-SYMBOL-NAME".to_string(),
        price: Some(123_456.789),
        pct_24h: Some(123.456),
    };

    assert_eq!(
        ticker_tape_item_width(&narrow_item, &denomination),
        TICKER_TAPE_ITEM_MIN_WIDTH
    );
    assert_eq!(
        ticker_tape_item_width(&wide_item, &denomination),
        TICKER_TAPE_ITEM_MAX_WIDTH
    );
}
