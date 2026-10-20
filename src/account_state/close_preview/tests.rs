use super::*;
use crate::api::BookLevel;

fn book() -> OrderBook {
    OrderBook {
        bids: vec![
            BookLevel { px: 110.0, sz: 1.0 },
            BookLevel { px: 108.0, sz: 4.0 },
        ],
        asks: vec![
            BookLevel { px: 112.0, sz: 1.0 },
            BookLevel { px: 114.0, sz: 4.0 },
        ],
    }
}

fn assert_near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9);
}

#[test]
fn long_close_consumes_bids_and_only_part_of_final_level() {
    let estimate =
        estimate_position_close(&book(), 3.0, 100.0, 0.001, 90.0).expect("sufficient depth");
    assert_near(estimate.average_price, 326.0 / 3.0);
    assert_near(estimate.gross_pnl, 26.0);
    assert_near(estimate.exit_fee, 0.326);
    assert_near(estimate.net_pnl, 25.674);
    assert_eq!(estimate.levels_used, 2);
}

#[test]
fn short_close_consumes_asks_and_charges_fees_on_buy_notional() {
    let estimate =
        estimate_position_close(&book(), -3.0, 120.0, 0.002, 130.0).expect("sufficient depth");
    assert_near(estimate.gross_pnl, 20.0);
    assert_near(estimate.exit_fee, 0.68);
    assert_near(estimate.net_pnl, 19.32);
    assert_eq!(estimate.levels_used, 2);
}

#[test]
fn exact_level_fill_zero_fees_and_losses_are_preserved() {
    let estimate =
        estimate_position_close(&book(), 1.0, 120.0, 0.0, 110.0).expect("fills at limit");
    assert_near(estimate.net_pnl, -10.0);
    assert_eq!(estimate.levels_used, 1);
}

#[test]
fn full_close_is_unavailable_for_missing_depth_or_slippage_limits() {
    for (book, size, limit, error) in [
        (book(), 6.0, 90.0, CloseEstimateError::InsufficientDepth),
        (book(), -6.0, 130.0, CloseEstimateError::InsufficientDepth),
        (
            OrderBook::empty(),
            1.0,
            90.0,
            CloseEstimateError::InsufficientDepth,
        ),
        (book(), 3.0, 109.0, CloseEstimateError::SlippageLimit),
        (book(), -3.0, 113.0, CloseEstimateError::SlippageLimit),
    ] {
        assert_eq!(
            estimate_position_close(&book, size, 100.0, 0.001, limit).err(),
            Some(error)
        );
    }
}

#[test]
fn invalid_inputs_books_and_overflow_do_not_produce_estimates() {
    for (size, entry, fee, limit) in [
        (0.0, 100.0, 0.001, 90.0),
        (f64::NAN, 100.0, 0.001, 90.0),
        (1.0, f64::INFINITY, 0.001, 90.0),
        (1.0, 0.0, 0.001, 90.0),
        (1.0, 100.0, -0.1, 90.0),
        (1.0, 100.0, f64::NAN, 90.0),
        (1.0, 100.0, 0.1, f64::NAN),
    ] {
        assert!(estimate_position_close(&book(), size, entry, fee, limit).is_err());
    }
    for invalid in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
        let mut depth = book();
        depth.bids[0].sz = invalid;
        assert!(estimate_position_close(&depth, 3.0, 100.0, 0.001, 90.0).is_err());
    }
    let mut depth = book();
    depth.bids[1].px = 111.0;
    assert_eq!(
        estimate_position_close(&depth, 3.0, 100.0, 0.001, 90.0).err(),
        Some(CloseEstimateError::InvalidBook)
    );
    depth.bids = vec![BookLevel {
        px: f64::MAX,
        sz: f64::MAX,
    }];
    assert!(estimate_position_close(&depth, f64::MAX, 100.0, 0.001, 90.0).is_err());
}

#[test]
fn preview_depth_expires_and_clear_invalidates_old_streams() {
    let now = Instant::now();
    let source = crate::app_state::TradingTerminal::boot_from_config(Default::default())
        .0
        .market_data_source_context();
    let mut preview = PositionClosePreview {
        generation: 7,
        coin: Some("BTC".into()),
        book: Some((book(), source, now)),
        fee_multiplier: Some(Ok(1.0)),
        now_ms: 0,
    };
    assert!(preview.fresh_book(now - Duration::from_secs(1)).is_some());
    assert!(preview.fresh_book(now + CLOSE_PREVIEW_MAX_AGE).is_some());
    assert!(
        preview
            .fresh_book(now + CLOSE_PREVIEW_MAX_AGE + Duration::from_millis(1))
            .is_none()
    );
    preview.clear();
    assert_eq!(preview.generation, 8);
    assert!(preview.coin.is_none() && preview.book.is_none() && preview.fee_multiplier.is_none());
}
