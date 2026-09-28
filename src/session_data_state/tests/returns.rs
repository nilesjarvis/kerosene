use super::*;

#[test]
fn session_returns_use_utc_open_weekday_and_open_to_close_return() {
    let bars = session_return_bars(&[candle(0, 100.0, 105.0)]);

    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].weekday, SessionWeekday::Mon);
    crate::helpers::assert_close(bars[0].return_pct, 5.0);
}

#[test]
fn session_returns_skip_invalid_open_values() {
    let bars = session_return_bars(&[
        candle(0, 0.0, 105.0),
        candle(1, f64::NAN, 105.0),
        candle(2, 100.0, 99.0),
    ]);

    assert_eq!(bars.len(), 1);
    crate::helpers::assert_close(bars[0].return_pct, -1.0);
}

#[test]
fn completed_session_returns_skip_open_daily_candle() {
    let completed = candle(0, 100.0, 105.0);
    let partial = candle_with_close_time(1, 1_704_239_999_999, 100.0, 90.0);
    let bars = completed_session_return_bars(&[completed, partial], 1_704_157_200_000);

    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].weekday, SessionWeekday::Mon);
    crate::helpers::assert_close(bars[0].return_pct, 5.0);
}

#[test]
fn weekday_summaries_track_average_count_and_win_rate() {
    let bars = session_return_bars(&[
        candle(0, 100.0, 110.0),
        candle(7, 100.0, 90.0),
        candle(1, 100.0, 102.0),
    ]);

    let summaries = weekday_summaries(&bars);
    let monday = &summaries[SessionWeekday::Mon.index()];
    let tuesday = &summaries[SessionWeekday::Tue.index()];

    assert_eq!(monday.sample_count, 2);
    crate::helpers::assert_close(monday.average_return_pct, 0.0);
    crate::helpers::assert_close(monday.win_rate_pct, 50.0);
    assert_eq!(tuesday.sample_count, 1);
    crate::helpers::assert_close(tuesday.average_return_pct, 2.0);
    crate::helpers::assert_close(tuesday.win_rate_pct, 100.0);
}

#[test]
fn weekday_summaries_include_all_weekdays() {
    let bars = session_return_bars(&[candle(0, 100.0, 110.0)]);

    let summaries = weekday_summaries(&bars);

    assert_eq!(summaries.len(), SessionWeekday::ALL.len());
    for (idx, weekday) in SessionWeekday::ALL.into_iter().enumerate() {
        assert_eq!(summaries[idx].weekday, weekday);
    }

    let monday = &summaries[SessionWeekday::Mon.index()];
    let tuesday = &summaries[SessionWeekday::Tue.index()];
    assert_eq!(monday.sample_count, 1);
    crate::helpers::assert_close(monday.average_return_pct, 10.0);
    assert_eq!(tuesday.sample_count, 0);
    crate::helpers::assert_close(tuesday.average_return_pct, 0.0);
    crate::helpers::assert_close(tuesday.win_rate_pct, 0.0);
}

#[test]
fn market_session_returns_split_intraday_candles_by_session_band() {
    // 2026-01-14 (winter offsets): Asia 00:00-08:00, London 08:00-14:30,
    // New York 14:30-21:00, Overnight 21:00-00:00 UTC.
    let day_start = ts(2026, 1, 14, 0, 0);
    let day_end = ts(2026, 1, 15, 0, 0);
    let candles = half_hour_candles(day_start, day_end, hours_price(day_start));

    let bars = market_session_return_bars(&candles, day_end);

    let kinds = bars.iter().map(|bar| bar.kind).collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            MarketSession::Asia,
            MarketSession::London,
            MarketSession::NewYork,
            MarketSession::Overnight,
        ]
    );
    assert_eq!(bars[0].start_ms, day_start);
    assert_eq!(bars[0].end_ms, ts(2026, 1, 14, 8, 0));
    crate::helpers::assert_close(bars[0].return_pct, 8.0);
    crate::helpers::assert_close(bars[1].return_pct, (114.5 - 108.0) / 108.0 * 100.0);
    crate::helpers::assert_close(bars[2].return_pct, (121.0 - 114.5) / 114.5 * 100.0);
    crate::helpers::assert_close(bars[3].return_pct, (124.0 - 121.0) / 121.0 * 100.0);
}

#[test]
fn market_session_returns_drop_partially_covered_bands() {
    let day_start = ts(2026, 1, 14, 0, 0);
    // Data starts after the Asia open and the clock stops before the
    // Overnight band completes, so only London and New York qualify.
    let candles = half_hour_candles(
        ts(2026, 1, 14, 1, 0),
        ts(2026, 1, 15, 0, 0),
        hours_price(day_start),
    );

    let bars = market_session_return_bars(&candles, ts(2026, 1, 14, 22, 0));

    let kinds = bars.iter().map(|bar| bar.kind).collect::<Vec<_>>();
    assert_eq!(kinds, vec![MarketSession::London, MarketSession::NewYork]);
}

#[test]
fn market_session_returns_skip_invalid_candles() {
    let day_start = ts(2026, 1, 14, 0, 0);
    let price_at = hours_price(day_start);
    let mut candles = half_hour_candles(day_start, ts(2026, 1, 15, 0, 0), &price_at);
    // Corrupt the candle at the 08:00 London open; the band open falls
    // back to the 08:30 candle while the Asia band stays intact.
    candles[16].open = f64::NAN;

    let bars = market_session_return_bars(&candles, ts(2026, 1, 15, 0, 0));

    assert_eq!(bars[0].kind, MarketSession::Asia);
    crate::helpers::assert_close(bars[0].return_pct, 8.0);
    assert_eq!(bars[1].kind, MarketSession::London);
    crate::helpers::assert_close(bars[1].open, price_at(ts(2026, 1, 14, 8, 30)));
    crate::helpers::assert_close(bars[1].return_pct, (114.5 - 108.5) / 108.5 * 100.0);
}

#[test]
fn market_session_summaries_track_average_count_and_win_rate() {
    let band = |kind, start_ms: u64, return_pct: f64| MarketSessionReturnBar {
        kind,
        start_ms,
        end_ms: start_ms + HALF_HOUR_MS,
        open: 100.0,
        close: 100.0 + return_pct,
        return_pct,
    };
    let bars = vec![
        band(MarketSession::Asia, 0, 2.0),
        band(MarketSession::Asia, 1, -1.0),
        band(MarketSession::London, 2, 1.0),
    ];

    let summaries = market_session_summaries(&bars);

    assert_eq!(summaries.len(), MarketSession::ALL.len());
    for (idx, session) in MarketSession::ALL.into_iter().enumerate() {
        assert_eq!(summaries[idx].session, session);
    }

    let asia = &summaries[0];
    assert_eq!(asia.sample_count, 2);
    crate::helpers::assert_close(asia.average_return_pct, 0.5);
    crate::helpers::assert_close(asia.win_rate_pct, 50.0);

    let london = &summaries[1];
    assert_eq!(london.sample_count, 1);
    crate::helpers::assert_close(london.average_return_pct, 1.0);
    crate::helpers::assert_close(london.win_rate_pct, 100.0);

    let new_york = &summaries[2];
    assert_eq!(new_york.sample_count, 0);
    crate::helpers::assert_close(new_york.average_return_pct, 0.0);
    crate::helpers::assert_close(new_york.win_rate_pct, 0.0);
}

#[test]
fn summary_rates_preserve_accumulation_order_and_nonfinite_returns() {
    for (values, expected_average, expected_win_rate) in [
        (vec![], 0.0, 0.0_f64),
        (vec![-0.0, 0.0], 0.0, 0.0),
        (vec![1e16, 1.0, -1e16], 0.0, 2.0 / 3.0 * 100.0),
        (vec![f64::MAX, f64::MAX], f64::INFINITY, 100.0),
        (vec![f64::INFINITY, f64::NEG_INFINITY], f64::NAN, 50.0),
        (vec![f64::NAN, -1.0, 2.0], f64::NAN, 1.0 / 3.0 * 100.0),
    ] {
        let daily = values
            .iter()
            .map(|&value| sample_bar(SessionWeekday::Tue, value, 0.0, 0))
            .collect::<Vec<_>>();
        let intraday = values
            .iter()
            .map(|&value| MarketSessionReturnBar {
                kind: MarketSession::London,
                start_ms: 0,
                end_ms: 1,
                open: 100.0,
                close: 100.0,
                return_pct: value,
            })
            .collect::<Vec<_>>();
        let weekdays = weekday_summaries(&daily);
        let sessions = market_session_summaries(&intraday);
        let weekday = &weekdays[SessionWeekday::Tue.index()];
        let session = &sessions[1];
        for (count, average, win_rate) in [
            (
                weekday.sample_count,
                weekday.average_return_pct,
                weekday.win_rate_pct,
            ),
            (
                session.sample_count,
                session.average_return_pct,
                session.win_rate_pct,
            ),
        ] {
            assert_eq!(count, values.len());
            if expected_average.is_nan() {
                assert!(average.is_nan());
            } else {
                assert_eq!(average.to_bits(), expected_average.to_bits());
            }
            assert_eq!(win_rate.to_bits(), expected_win_rate.to_bits());
        }
        assert_eq!(weekdays[0].sample_count, 0);
        assert_eq!(sessions[0].sample_count, 0);
    }
}

#[test]
fn market_session_returns_preserve_duplicate_precedence_and_invalid_edge_coverage() {
    let day_start = ts(2026, 1, 14, 0, 0);
    let day_end = ts(2026, 1, 15, 0, 0);
    let price_at = hours_price(day_start);
    let original = half_hour_candles(day_start, day_end, &price_at);
    let mut candles = vec![original[0].clone(), original[0].clone()];
    candles[0].open = f64::NAN;
    candles.extend(original[1..].iter().rev().cloned());
    candles[2].close = f64::NAN;
    let mut duplicate = original[16].clone();
    duplicate.open = 50.0;
    candles.push(duplicate);

    let bars = market_session_return_bars(&candles, day_end);

    assert_eq!(bars.len(), 4);
    assert_eq!(bars[0].kind, MarketSession::Asia);
    assert_eq!(bars[0].start_ms, day_start);
    assert_eq!(bars[0].open, price_at(day_start + HALF_HOUR_MS));
    assert_eq!(bars[1].kind, MarketSession::London);
    assert_eq!(bars[1].open, 108.0);
    assert_eq!(bars[3].kind, MarketSession::Overnight);
    assert_eq!(bars[3].end_ms, day_end);
    assert_eq!(bars[3].close, price_at(day_end - HALF_HOUR_MS));
    assert!(candles[0].open.is_nan());
    assert!(candles[2].close.is_nan());
    assert_eq!(candles[1].open, 100.0);
}
