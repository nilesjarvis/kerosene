use super::super::tests::{live_trade, request, trade};
use super::*;

fn request_settings(
    account_key: Option<&str>,
    address: &str,
    coverage: JournalSnapshotCoverage,
    now_ms: u64,
) -> JournalSnapshotRequestSettings {
    JournalSnapshotRequestSettings {
        account_key: account_key.map(str::to_string),
        address: address.to_string(),
        source: ChartBackfillSource::Hyperliquid,
        read_data_provider_generation: 0,
        hydromancer_key_generation: 0,
        coverage,
        now_ms,
    }
}

#[test]
fn planner_chooses_one_minute_for_short_trades() {
    let idx = initial_ladder_index(1_000, 61_000, JournalSnapshotCoverage::default());
    assert_eq!(SNAPSHOT_LADDER[idx], Timeframe::M1);
}

#[test]
fn default_coverage_doubles_snapshot_padding() {
    let mut trade = trade(true);
    trade.start_time = 10_000_000;
    trade.end_time = Some(10_060_000);

    let request = initial_snapshot_request(
        request_settings(
            Some("acct"),
            "0xabc",
            JournalSnapshotCoverage::default(),
            10_060_000,
        ),
        &trade,
    )
    .expect("snapshot request");

    let expected_padding = MIN_PADDING_MS * 2;
    assert_eq!(request.coverage, JournalSnapshotCoverage::TwoX);
    assert_eq!(request.start_ms, trade.start_time - expected_padding);
    assert_eq!(
        request.end_ms,
        trade.end_time.expect("end") + expected_padding
    );
}

#[test]
fn higher_coverage_widens_fixed_timeframe_requests() {
    let mut trade = trade(true);
    trade.start_time = 20_000_000;
    trade.end_time = Some(20_060_000);

    let two_x = snapshot_request_for_timeframe(
        request_settings(None, "0xabc", JournalSnapshotCoverage::TwoX, 20_060_000),
        &trade,
        Timeframe::M1,
    )
    .expect("2x request");
    let four_x = snapshot_request_for_timeframe(
        request_settings(None, "0xabc", JournalSnapshotCoverage::FourX, 20_060_000),
        &trade,
        Timeframe::M1,
    )
    .expect("4x request");

    assert_eq!(four_x.start_ms, two_x.start_ms - MIN_PADDING_MS * 2);
    assert_eq!(four_x.end_ms, two_x.end_ms + MIN_PADDING_MS * 2);
}

#[test]
fn empty_retry_advances_to_next_timeframe() {
    let next = next_snapshot_request(&request()).expect("next request");
    assert_eq!(next.timeframe, Timeframe::M3);
    assert_eq!(next.ladder_index, 1);
}

#[test]
fn live_position_request_charts_recent_window_for_open_position() {
    let now = 1_000_000_000_000;
    let request = live_position_snapshot_request(
        request_settings(
            Some("acct"),
            "0xabc",
            JournalSnapshotCoverage::default(),
            now,
        ),
        &live_trade(),
    )
    .expect("live request");

    assert!(request.is_open);
    assert_eq!(request.trade_end_ms, now);
    assert_eq!(request.trade_start_ms, now - LIVE_POSITION_LOOKBACK_MS);
    // Open requests fetch up to "now" with no forward padding.
    assert_eq!(request.end_ms, request.trade_end_ms);
}

#[test]
fn live_position_request_rejects_closed_spot_and_missing_entry() {
    let now = 1_000_000_000_000;

    // A closed trade is not a live position.
    assert!(
        live_position_snapshot_request(
            request_settings(None, "a", JournalSnapshotCoverage::default(), now),
            &trade(true),
        )
        .is_err()
    );

    let mut spot = live_trade();
    spot.coin = "PURR/USDC".to_string();
    assert!(
        live_position_snapshot_request(
            request_settings(None, "a", JournalSnapshotCoverage::default(), now),
            &spot,
        )
        .is_err()
    );

    let mut no_entry = live_trade();
    no_entry.avg_entry_price = 0.0;
    assert!(
        live_position_snapshot_request(
            request_settings(None, "a", JournalSnapshotCoverage::default(), now),
            &no_entry,
        )
        .is_err()
    );
}

#[test]
fn planners_preserve_admission_error_precedence() {
    let non_perp = "Chart snapshots are currently available for perp trades only.";
    let missing_basis = "Snapshot unavailable because opening fills are outside loaded history.";
    let closed = "Live-position snapshots are only available while a position is open.";
    let missing_price =
        "Snapshot unavailable because no entry price is available for this position.";
    let unsupported = "Unsupported snapshot timeframe.";
    for (coin, basis, end_time, entry, historical_error, live_error) in [
        (
            "@1",
            false,
            Some(2_000),
            0.0,
            Some(non_perp),
            Some(non_perp),
        ),
        (
            "BTC",
            false,
            Some(2_000),
            0.0,
            Some(missing_basis),
            Some(closed),
        ),
        (
            "BTC",
            false,
            None,
            f64::NAN,
            Some(missing_basis),
            Some(missing_price),
        ),
        ("BTC", false, None, 100.0, Some(missing_basis), None),
        ("BTC", true, None, 100.0, None, None),
    ] {
        let mut trade = trade(true);
        trade.coin = coin.to_string();
        trade.basis_complete = basis;
        trade.end_time = end_time;
        trade.avg_entry_price = entry;
        let settings = request_settings(None, "test-address", JournalSnapshotCoverage::TwoX, 3_000);

        assert_eq!(
            initial_snapshot_request(settings.clone(), &trade)
                .err()
                .as_deref(),
            historical_error
        );
        assert_eq!(
            snapshot_request_for_timeframe(settings.clone(), &trade, Timeframe::S1)
                .err()
                .as_deref(),
            historical_error.or(Some(unsupported))
        );
        assert_eq!(
            live_position_snapshot_request(settings.clone(), &trade)
                .err()
                .as_deref(),
            live_error
        );
        assert_eq!(
            live_position_snapshot_request_for_timeframe(settings, &trade, Timeframe::S1)
                .err()
                .as_deref(),
            live_error.or(Some(unsupported))
        );
    }
}

#[test]
fn historical_request_bounds_keep_saturation_and_open_padding_rules() {
    for (start, end, now, expected_trade_end, expected_start, expected_end) in [
        (
            8_000_000,
            Some(8_060_000),
            0,
            8_060_000,
            800_000,
            15_260_000,
        ),
        (8_000_000, None, 8_060_000, 8_060_000, 800_000, 8_060_000),
        (
            10_000_000,
            Some(9_000_000),
            0,
            10_000_000,
            2_800_000,
            17_200_000,
        ),
        (1_000, Some(2_000), 0, 2_000, 0, 7_202_000),
        (
            u64::MAX - 1_000,
            Some(u64::MAX),
            0,
            u64::MAX,
            u64::MAX - 7_201_000,
            u64::MAX,
        ),
    ] {
        let mut trade = trade(true);
        trade.start_time = start;
        trade.end_time = end;
        let request = snapshot_request_for_timeframe(
            request_settings(None, "test-address", JournalSnapshotCoverage::TwoX, now),
            &trade,
            Timeframe::M1,
        )
        .expect("historical request");
        assert_eq!(request.trade_end_ms, expected_trade_end);
        assert_eq!(
            (request.start_ms, request.end_ms),
            (expected_start, expected_end)
        );
        assert_eq!(request.is_open, end.is_none());
    }

    let mut open = live_trade();
    open.start_time = 20_000_000;
    let request = initial_snapshot_request(
        request_settings(
            None,
            "test-address",
            JournalSnapshotCoverage::TwoX,
            25_400_000,
        ),
        &open,
    )
    .expect("open request");
    // Rung selection budgets padding on both sides, even for open trades.
    assert_eq!(request.timeframe, Timeframe::M3);
    assert_eq!((request.start_ms, request.end_ms), (11_900_000, 25_400_000));
}

#[test]
fn retries_match_pinned_ranges_and_preserve_request_context() {
    for coverage in JournalSnapshotCoverage::OPTIONS {
        for end in [None, Some(2_000), Some(u64::MAX)] {
            let mut trade = trade(true);
            trade.end_time = end;
            let mut settings =
                request_settings(Some("test-account"), "test-address", coverage, 3_000);
            settings.source = ChartBackfillSource::Hydromancer;
            settings.read_data_provider_generation = 17;
            settings.hydromancer_key_generation = 29;
            let mut request =
                snapshot_request_for_timeframe(settings.clone(), &trade, Timeframe::M1)
                    .expect("first rung");
            for &timeframe in &SNAPSHOT_LADDER[1..] {
                let expected = snapshot_request_for_timeframe(settings.clone(), &trade, timeframe)
                    .expect("pinned rung");
                let next = next_snapshot_request(&request).expect("next rung");
                assert_eq!(next, expected);
                request = next;
            }
            assert!(next_snapshot_request(&request).is_none());
            request.ladder_index = usize::MAX;
            assert!(next_snapshot_request(&request).is_none());
        }
    }
}

#[test]
fn pinned_live_requests_cap_fine_timeframes_without_capping_auto_lookback() {
    for now in [1_000, 1_000_000_000_000] {
        let settings = request_settings(None, "test-address", JournalSnapshotCoverage::TwoX, now);
        let automatic = live_position_snapshot_request(settings.clone(), &live_trade())
            .expect("automatic live request");
        assert_eq!(automatic.trade_start_ms, now.saturating_sub(604_800_000));
        for (timeframe, lookback) in [(Timeframe::M1, 15_600_000), (Timeframe::H1, 604_800_000)] {
            let pinned = live_position_snapshot_request_for_timeframe(
                settings.clone(),
                &live_trade(),
                timeframe,
            )
            .expect("pinned live request");
            assert_eq!(pinned.trade_start_ms, now.saturating_sub(lookback));
            assert_eq!(pinned.trade_end_ms, now);
            assert_eq!(pinned.end_ms, now);
            assert!(pinned.is_open);
        }
    }
}
