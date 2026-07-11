use super::*;
use crate::api::{BookLevel, OrderBook};

fn book(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> OrderBook {
    OrderBook {
        bids: bids
            .iter()
            .map(|(px, sz)| BookLevel { px: *px, sz: *sz })
            .collect(),
        asks: asks
            .iter()
            .map(|(px, sz)| BookLevel { px: *px, sz: *sz })
            .collect(),
    }
}

#[test]
fn planned_slice_validation_returns_crossing_ioc_price() {
    let book = book(&[(99.5, 2.0)], &[(100.0, 0.25), (100.5, 1.0)]);

    let limit_price = validate_twap_planned_slice(&book, true, 1.0, 99.0, 101.0, 2, false).unwrap();

    assert_eq!(limit_price, 100.5);
}

#[test]
fn planned_slice_validation_preserves_range_skip_message() {
    let book = book(&[(99.5, 1.0)], &[(100.0, 0.25)]);

    let skip = validate_twap_planned_slice(&book, true, 1.0, 99.0, 101.0, 2, false).unwrap_err();

    assert_eq!(skip.kind, TwapEventKind::SkippedRange);
    assert_eq!(
        skip.message,
        "TWAP slice skipped: book cannot fill 1 inside 99.00-101.00"
    );
    assert!(!skip.is_error);

    let rendered = format!("{skip:?}");
    assert!(rendered.contains("TwapPlannedSliceSkip"), "{rendered}");
    assert!(rendered.contains("SkippedRange"), "{rendered}");
    assert!(rendered.contains("<redacted>"), "{rendered}");
    assert!(!rendered.contains(skip.message.as_str()), "{rendered}");
}

#[test]
fn planned_slice_validation_preserves_minimum_notional_message() {
    let book = book(&[(99.5, 1.0)], &[(100.0, 1.0)]);

    let skip = validate_twap_planned_slice(&book, true, 0.05, 99.0, 101.0, 2, false).unwrap_err();

    assert_eq!(skip.kind, TwapEventKind::SkippedMinimum);
    assert_eq!(
        skip.message,
        "TWAP slice skipped: child notional $5.00 is below Hyperliquid's $10 minimum"
    );
    assert!(skip.is_error);
}
