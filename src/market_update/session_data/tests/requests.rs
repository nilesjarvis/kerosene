use super::*;

#[test]
fn session_data_error_redacts_state_error() {
    let mut terminal = TradingTerminal::boot().0;
    let request = SessionDataRequest {
        id: 7,
        symbol: "BTC".to_string(),
        lookback: SessionDataLookback::FourWeeks,
        requested_at_ms: 123,
    };
    terminal.session_data.insert(
        7,
        SessionDataInstance::new(7, "BTC".to_string(), SessionDataLookback::FourWeeks),
    );
    {
        let instance = terminal.session_data.get_mut(&7).expect("session data");
        instance.loading = true;
        instance.pending_request = Some(request.clone());
    }

    let _task = terminal.apply_session_data_candles_loaded(
        request,
        Err("session fetch failed: api_key=session-secret".to_string()),
    );

    let error = terminal
        .session_data
        .get(&7)
        .and_then(|instance| instance.error.as_deref())
        .expect("state error");
    assert!(error.contains("api_key=<redacted>"));
    assert!(!error.contains("session-secret"));
}

#[test]
fn forced_refresh_coalesces_identical_pending_request() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.session_data.insert(
        7,
        SessionDataInstance::new(7, "HYPE".to_string(), SessionDataLookback::FourWeeks),
    );
    let pending = SessionDataRequest {
        id: 7,
        symbol: "HYPE".to_string(),
        lookback: SessionDataLookback::FourWeeks,
        requested_at_ms: 123,
    };
    {
        let instance = terminal.session_data.get_mut(&7).expect("session data");
        instance.loading = true;
        instance.pending_request = Some(pending.clone());
    }

    let _task = terminal.request_session_data_refresh(7, true);

    let instance = terminal.session_data.get(&7).expect("session data");
    assert!(instance.loading);
    assert_eq!(instance.pending_request.as_ref(), Some(&pending));
}

#[test]
fn forced_refresh_replaces_pending_request_when_target_changes() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.session_data.insert(
        7,
        SessionDataInstance::new(7, "BTC".to_string(), SessionDataLookback::FourWeeks),
    );
    let pending = SessionDataRequest {
        id: 7,
        symbol: "HYPE".to_string(),
        lookback: SessionDataLookback::FourWeeks,
        requested_at_ms: 123,
    };
    {
        let instance = terminal.session_data.get_mut(&7).expect("session data");
        instance.loading = true;
        instance.pending_request = Some(pending);
    }

    let _task = terminal.request_session_data_refresh(7, true);

    let request = terminal
        .session_data
        .get(&7)
        .and_then(|instance| instance.pending_request.as_ref())
        .expect("replacement request");
    assert_eq!(request.id, 7);
    assert_eq!(request.symbol, "BTC");
    assert_eq!(request.lookback, SessionDataLookback::FourWeeks);
    assert_ne!(request.requested_at_ms, 123);
}

#[test]
fn forced_refresh_replaces_pending_request_when_lookback_changes() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.session_data.insert(
        7,
        SessionDataInstance::new(7, "HYPE".to_string(), SessionDataLookback::EightWeeks),
    );
    let pending = SessionDataRequest {
        id: 7,
        symbol: "HYPE".to_string(),
        lookback: SessionDataLookback::FourWeeks,
        requested_at_ms: 123,
    };
    {
        let instance = terminal.session_data.get_mut(&7).expect("session data");
        instance.loading = true;
        instance.pending_request = Some(pending);
    }

    let _task = terminal.request_session_data_refresh(7, true);

    let request = terminal
        .session_data
        .get(&7)
        .and_then(|instance| instance.pending_request.as_ref())
        .expect("replacement request");
    assert_eq!(request.id, 7);
    assert_eq!(request.symbol, "HYPE");
    assert_eq!(request.lookback, SessionDataLookback::EightWeeks);
    assert_ne!(request.requested_at_ms, 123);
}

#[test]
fn intraday_chunk_ranges_tile_long_lookbacks_without_gaps() {
    let start = 1_704_067_200_000;
    let end = start + 365 * DAY_MS;
    let chunk_ms = INTRADAY_CANDLE_MS * INTRADAY_MAX_CANDLES_PER_REQUEST;

    let ranges = intraday_chunk_ranges(start, end);

    assert_eq!(ranges.first().map(|range| range.0), Some(start));
    assert_eq!(ranges.last().map(|range| range.1), Some(end));
    for pair in ranges.windows(2) {
        assert_eq!(pair[0].1, pair[1].0);
    }
    for (chunk_start, chunk_end) in &ranges {
        assert!(chunk_end - chunk_start <= chunk_ms);
    }
    assert_eq!(ranges.len(), 5);
}

#[test]
fn intraday_chunk_ranges_use_single_request_for_short_lookbacks() {
    let start = 1_704_067_200_000;
    let end = start + 28 * DAY_MS;

    assert_eq!(intraday_chunk_ranges(start, end), vec![(start, end)]);
    assert!(intraday_chunk_ranges(end, start).is_empty());
}

#[test]
fn refresh_admission_preserves_history_pending_state_and_error_precedence() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![exchange_symbol("HYPE", "HYPE", MarketType::Perp)];
    terminal.muted_tickers.insert("HIDDEN".to_string());
    for (symbol, force, error, loading) in [
        ("", false, "previous error", true),
        ("  ", true, "Select a symbol", false),
        ("HIDDEN", true, "Ticker is hidden in Settings > Risk", false),
        (
            "MISSING",
            true,
            "Session Data is available for perp and spot candle symbols",
            false,
        ),
        ("HYPE", true, "previous error", true),
    ] {
        let original = history_instance(7, symbol);
        terminal.session_data.insert(7, original.clone());
        let _task = terminal.request_session_data_refresh(7, force);
        let instance = terminal.session_data.get(&7).expect("session data");
        assert_eq!(instance.loading, loading);
        assert_eq!(instance.error.as_deref(), Some(error));
        assert_eq!(instance.pending_request, original.pending_request);
        assert_eq!(instance.bars, original.bars);
        assert_eq!(instance.last_fetch_ms, original.last_fetch_ms);
    }
}
