use super::*;

fn key(symbol: &str, period: usize) -> LiveWatchlistEmaKey {
    LiveWatchlistEmaKey {
        symbol: symbol.to_string(),
        settings: LiveWatchlistEmaConfig {
            period,
            ..Default::default()
        },
    }
}

#[test]
fn ema_requests_deduplicate_and_obey_concurrency_and_retry_cooldown() {
    let mut state = LiveWatchlistEmaState::default();
    let needed = HashSet::from([key("BTC", 20), key("BTC", 20), key("ETH", 20)]);
    let requests = state.plan(&needed, 1_000);
    assert_eq!(requests.len(), 2);
    assert!(state.plan(&needed, 2_000).is_empty());
    let (key, id) = &requests[0];
    assert!(state.apply(key, *id, Err("network".to_string())));
    assert!(state.plan(&needed, 60_999).is_empty());
    assert_eq!(state.plan(&needed, 61_000).len(), 1);
    assert!(state.value(key, Some(100.0), 61_000).0.is_none());
}

#[test]
fn ema_setting_changes_count_obsolete_in_flight_requests_and_reject_old_results() {
    let mut state = LiveWatchlistEmaState::default();
    let old = (1..=6).map(|period| key("BTC", period)).collect();
    let requests = state.plan(&old, 1_000);
    assert_eq!(requests.len(), MAX_IN_FLIGHT);
    let needed = HashSet::from([key("BTC", 50)]);
    assert!(state.plan(&needed, 2_000).is_empty());
    let (old_key, id) = &requests[0];
    assert!(!state.apply(old_key, id + 1, Err("stale".to_string())));
    assert!(state.apply(old_key, *id, Err("old setting".to_string())));
    let next = state.plan(&needed, 3_000);
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].0.settings.period, 50);
    assert!(!state.entries.contains_key(old_key));
    assert!(!state.apply(old_key, *id, Err("duplicate".to_string())));
    assert_eq!(
        state
            .value(&key("BTC", 50), Some(100.0), 3_000)
            .1
            .as_deref(),
        Some("Loading EMA…")
    );
}

#[test]
fn disabling_ema_discards_completed_data_and_preserves_request_identity() {
    let mut state = LiveWatchlistEmaState::default();
    let needed = HashSet::from([key("BTC", 20)]);
    let requests = state.plan(&needed, 0);
    let (key, old_id) = &requests[0];
    assert!(state.apply(key, *old_id, Err("unavailable".to_string())));
    assert!(state.plan(&HashSet::new(), 1).is_empty());
    assert!(state.entries.is_empty());
    let next = state.plan(&needed, 2);
    assert!(next[0].1 > *old_id);
    assert!(!state.apply(key, *old_id, Err("old".to_string())));
}

#[test]
fn large_watchlists_load_untouched_symbols_before_refreshing_previous_ones() {
    let mut state = LiveWatchlistEmaState::default();
    let needed = (1..=5).map(|period| key("BTC", period)).collect();
    let requests = state.plan(&needed, 0);
    for (key, id) in requests {
        assert!(state.apply(&key, id, Err("unavailable".to_string())));
    }
    let requests = state.plan(&needed, REFRESH_MS);
    assert_eq!(requests[0].0.settings.period, 5);
}

fn watchlist(
    id: u64,
    symbols: &[&str],
    enabled: bool,
) -> crate::market_state::LiveWatchlistInstance {
    crate::market_state::LiveWatchlistInstance {
        id,
        preset_id: None,
        symbols: symbols.iter().map(|symbol| symbol.to_string()).collect(),
        search_query: String::new(),
        sort_column: Default::default(),
        sort_direction: Default::default(),
        visible_columns: if enabled {
            vec![LiveWatchlistColumn::EmaDistance]
        } else {
            Vec::new()
        },
        ema: Default::default(),
        ema_period_input: "20".to_string(),
        row_cache: Vec::new(),
    }
}

#[test]
fn ema_requests_only_cover_open_enabled_unhidden_widgets_and_deduplicate_keys() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.live_watchlists.clear();
    let (mut panes, first) =
        iced::widget::pane_grid::State::new(crate::pane_state::PaneKind::LiveWatchlist(1));
    panes.split(
        iced::widget::pane_grid::Axis::Horizontal,
        first,
        crate::pane_state::PaneKind::LiveWatchlist(2),
    );
    terminal.panes = panes;
    terminal
        .live_watchlists
        .insert(1, watchlist(1, &["BTC", "ETH"], true));
    terminal
        .live_watchlists
        .insert(2, watchlist(2, &["BTC"], true));
    terminal
        .live_watchlists
        .insert(3, watchlist(3, &["SOL"], true));
    assert_eq!(
        terminal.live_watchlist_ema_keys(),
        HashSet::from([key("BTC", 20), key("ETH", 20)])
    );
    terminal
        .live_watchlists
        .get_mut(&1)
        .expect("watchlist")
        .visible_columns
        .clear();
    assert_eq!(
        terminal.live_watchlist_ema_keys(),
        HashSet::from([key("BTC", 20)])
    );
    terminal
        .live_watchlists
        .get_mut(&2)
        .expect("watchlist")
        .ema
        .period = 50;
    assert_eq!(
        terminal.live_watchlist_ema_keys(),
        HashSet::from([key("BTC", 50)])
    );
    terminal.muted_tickers.insert("BTC".to_string());
    assert!(terminal.live_watchlist_ema_keys().is_empty());
}

#[test]
fn successful_ema_results_update_rows_using_each_widgets_settings_and_live_mid() {
    let (mut terminal, _) = TradingTerminal::boot();
    let (mut panes, first) =
        iced::widget::pane_grid::State::new(crate::pane_state::PaneKind::LiveWatchlist(1));
    panes.split(
        iced::widget::pane_grid::Axis::Horizontal,
        first,
        crate::pane_state::PaneKind::LiveWatchlist(2),
    );
    terminal.panes = panes;
    terminal.live_watchlists.clear();
    terminal
        .live_watchlists
        .insert(1, watchlist(1, &["BTC"], true));
    let mut second = watchlist(2, &["BTC"], true);
    second.ema.period = 50;
    terminal.live_watchlists.insert(2, second);
    let now = TradingTerminal::now_ms();
    let requests = terminal
        .live_watchlist_ema
        .plan(&terminal.live_watchlist_ema_keys(), now);
    let (key, request_id) = requests
        .iter()
        .find(|(key, _)| key.settings.period == 20)
        .expect("20-period request");
    let candles = (0..21)
        .map(|index| crate::api::Candle::test_flat(now - (20 - index) * 60_000, 100.0))
        .collect();
    let sample = WatchlistEmaSample::from_candles(candles, &key.settings, now).expect("EMA sample");
    let _ = terminal.apply_live_watchlist_ema_loaded(key.clone(), *request_id, Ok(sample));
    let _ = terminal.handle_mids_update(HashMap::from([("BTC".to_string(), 110.0)]));
    let row = &terminal.live_watchlists[&1].row_cache[0];
    let expected_ema = 100.0 + 10.0 * 2.0 / 21.0;
    let expected = (110.0 - expected_ema) / expected_ema * 100.0;
    assert!((row.ema_distance.expect("distance") - expected).abs() < 1e-10);
    assert!(row.ema_status.is_none());
    assert!(
        terminal.live_watchlists[&2].row_cache[0]
            .ema_distance
            .is_none()
    );
    let _ = terminal.apply_live_watchlist_ema_loaded(
        key.clone(),
        *request_id,
        Err("duplicate".to_string()),
    );
    assert!(
        terminal.live_watchlists[&1].row_cache[0]
            .ema_distance
            .is_some()
    );
    terminal
        .live_watchlists
        .get_mut(&1)
        .expect("watchlist")
        .symbols
        .clear();
    let _ = terminal.request_live_watchlist_ema_refresh();
    assert!(!terminal.live_watchlist_ema.entries.contains_key(key));
}

#[test]
fn candle_rollover_hides_unfinished_snapshot_and_refreshes_before_minute_cooldown() {
    let mut state = LiveWatchlistEmaState::default();
    let key = key("BTC", 3);
    let needed = HashSet::from([key.clone()]);
    let requests = state.plan(&needed, 238_000);
    let candles = [60_000, 120_000, 180_000]
        .into_iter()
        .map(|time| crate::api::Candle::test_flat(time, 100.0))
        .collect();
    let sample = WatchlistEmaSample::from_candles(candles, &key.settings, 238_000).expect("EMA");
    assert!(state.apply(&key, requests[0].1, Ok(sample)));
    assert_eq!(state.value(&key, Some(100.0), 239_000).0, Some(0.0));
    assert_eq!(state.value(&key, Some(100.0), 240_000).0, None);
    assert!(state.plan(&needed, 240_000).is_empty());
    assert_eq!(state.plan(&needed, 253_000).len(), 1);
}
