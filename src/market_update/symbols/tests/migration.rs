use super::*;

#[test]
fn spot_alias_migration_updates_presets_snapshots_and_caches_only_once() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.order_books.clear();
    terminal.spaghetti_charts.clear();
    terminal.exchange_symbols = vec![canonical_purr_symbol()];
    let legacy = ["BTC", "@0", "PURR/USDC", "BTC", "@107", "@0"]
        .map(str::to_string)
        .to_vec();
    terminal.watchlist_presets = vec![
        crate::config::WatchlistPresetConfig {
            id: 11,
            name: "Changed".into(),
            symbols: legacy.clone(),
        },
        crate::config::WatchlistPresetConfig {
            id: 12,
            name: "Unchanged".into(),
            symbols: vec!["BTC".into(), "BTC".into()],
        },
    ];
    terminal.live_watchlists = HashMap::from([
        (
            9,
            LiveWatchlistInstance {
                id: 9,
                preset_id: Some(11),
                symbols: legacy,
                search_query: String::new(),
                sort_column: Default::default(),
                sort_direction: Default::default(),
                visible_columns: crate::config::default_live_watchlist_columns(),
                ema: Default::default(),
                ema_period_input: "20".to_string(),
                row_cache: Vec::new(),
            },
        ),
        (
            10,
            LiveWatchlistInstance {
                id: 10,
                preset_id: Some(12),
                symbols: vec!["BTC".into(), "BTC".into()],
                search_query: String::new(),
                sort_column: Default::default(),
                sort_direction: Default::default(),
                visible_columns: crate::config::default_live_watchlist_columns(),
                ema: Default::default(),
                ema_period_input: "20".to_string(),
                row_cache: Vec::new(),
            },
        ),
    ]);
    terminal.saved_layouts = vec![
        serde_json::from_value(serde_json::json!({
            "name": "Saved",
            "live_watchlists": [
                {"id": 9, "preset_id": 11, "symbols": ["@0"]},
                {"id": 10, "preset_id": 12, "symbols": ["BTC", "BTC"]}
            ],
            "spaghetti_charts": [{"id": 8, "watchlist_preset_id": 11, "symbols": ["@0"]}]
        }))
        .expect("saved layout fixture"),
    ];
    for key in ["@0", "PURR/USDC"] {
        terminal
            .live_watchlist_ctxs
            .insert(key.into(), context(2.0));
        terminal
            .live_watchlist_history
            .insert(key.into(), (1.0, 2.0, 3.0));
        terminal
            .live_watchlist_history_loaded_at
            .insert(key.into(), 42);
    }
    terminal.live_watchlist_contexts_request_id = 7;
    terminal.live_watchlist_history_request_id = 8;

    let _tasks = terminal.migrate_legacy_spot_widget_keys();

    let canonical = ["BTC", "PURR/USDC", "@107"];
    assert_eq!(terminal.watchlist_presets[0].symbols, canonical);
    assert_eq!(terminal.live_watchlists[&9].symbols, canonical);
    assert_eq!(
        terminal.saved_layouts[0].live_watchlists[0].symbols,
        canonical
    );
    assert_eq!(
        terminal.saved_layouts[0].spaghetti_charts[0].symbols,
        canonical
    );
    assert_eq!(terminal.watchlist_presets[1].symbols, ["BTC", "BTC"]);
    assert_eq!(terminal.live_watchlists[&10].symbols, ["BTC", "BTC"]);
    assert_eq!(
        terminal.saved_layouts[0].live_watchlists[1].symbols,
        ["BTC", "BTC"]
    );
    assert!(!terminal.live_watchlist_ctxs.contains_key("@0"));
    assert!(!terminal.live_watchlist_history.contains_key("@0"));
    assert!(!terminal.live_watchlist_history_loaded_at.contains_key("@0"));
    assert!(terminal.live_watchlist_ctxs.contains_key("PURR/USDC"));
    assert!(terminal.live_watchlist_history.contains_key("PURR/USDC"));
    assert!(
        terminal
            .live_watchlist_history_loaded_at
            .contains_key("PURR/USDC")
    );
    assert_eq!(terminal.live_watchlist_contexts_request_id, 8);
    assert_eq!(terminal.live_watchlist_history_request_id, 9);
    assert!(terminal.config_save_due_at.is_some());

    terminal.config_save_due_at = None;
    assert!(terminal.migrate_legacy_spot_widget_keys().is_empty());
    assert!(terminal.config_save_due_at.is_none());
    assert_eq!(terminal.live_watchlist_contexts_request_id, 8);
    assert_eq!(terminal.live_watchlist_history_request_id, 9);
}

#[test]
fn successful_spot_metadata_migrates_legacy_widget_keys_atomically() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.order_books.clear();
    let mut book = OrderBookInstance::new(7, OrderBookSymbolMode::Fixed("@0".to_string()), 0.01);
    book.book_error = Some("legacy fetch failed".to_string());
    terminal.order_books.insert(7, book);

    terminal.spaghetti_charts.clear();
    let mut spaghetti = SpaghettiChartInstance::new_empty(8);
    spaghetti.canvas.series.push(Series {
        symbol: "@0".to_string(),
        display: "@0".to_string(),
        candles: vec![Candle::test_flat(1_000, 0.2)],
        color: iced::Color::BLACK,
        loaded: true,
    });
    terminal.spaghetti_charts.insert(8, spaghetti);

    terminal.live_watchlists.clear();
    terminal.live_watchlists.insert(
        9,
        LiveWatchlistInstance {
            id: 9,
            preset_id: None,
            symbols: vec!["@0".to_string(), "PURR/USDC".to_string()],
            search_query: String::new(),
            sort_column: crate::config::LiveWatchlistSortColumn::default(),
            sort_direction: crate::config::SortDirection::default(),
            visible_columns: crate::config::default_live_watchlist_columns(),
            ema: Default::default(),
            ema_period_input: "20".to_string(),
            row_cache: Vec::new(),
        },
    );
    terminal.live_watchlist_contexts_loading = true;
    terminal.live_watchlist_contexts_request_id = 3;
    terminal.live_watchlist_contexts_request_symbols = vec!["@0".to_string()];
    terminal.live_watchlist_history_loading = true;
    terminal.live_watchlist_history_request_id = 4;
    terminal.live_watchlist_history_request_symbols = vec!["@0".to_string()];

    let task = terminal.apply_symbols_loaded(Ok(payload(vec![
        perp_symbol("HYPE"),
        canonical_purr_symbol(),
    ])));

    assert_eq!(
        terminal.order_books[&7].mode,
        OrderBookSymbolMode::Fixed("PURR/USDC".to_string())
    );
    assert!(terminal.order_books[&7].book_loading);
    assert!(terminal.order_books[&7].book_error.is_none());
    let series = &terminal.spaghetti_charts[&8].canvas.series;
    assert_eq!(series.len(), 1);
    assert_eq!(series[0].symbol, "PURR/USDC");
    assert_eq!(series[0].display, "PURR/USDC");
    assert!(series[0].candles.is_empty());
    assert!(!series[0].loaded);
    assert_eq!(
        terminal.live_watchlists[&9].symbols,
        vec!["PURR/USDC".to_string()]
    );
    assert!(!terminal.live_watchlist_contexts_loading);
    assert!(terminal.live_watchlist_contexts_request_symbols.is_empty());
    assert_eq!(terminal.live_watchlist_contexts_request_id, 4);
    assert!(!terminal.live_watchlist_history_loading);
    assert!(terminal.live_watchlist_history_request_symbols.is_empty());
    assert_eq!(terminal.live_watchlist_history_request_id, 5);
    assert!(terminal.config_save_due_at.is_some());
    assert!(
        task.units() >= 2,
        "book and spaghetti data must be refetched"
    );
}

#[test]
fn successful_spot_metadata_migrates_regular_chart_aliases_before_refetch() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();

    let context = crate::chart_state::ChartBackfillRequestContext::new(
        crate::config::ChartBackfillSource::Hyperliquid,
        0,
        0,
    );
    let mut duplicate = ChartInstance::new(10, "@0".to_string(), Timeframe::H1);
    duplicate.set_secondary_symbol_identity("@0".to_string(), "@0".to_string());
    duplicate.candle_fetch_request = Some(TradingTerminal::build_candle_fetch_request(
        10,
        "@0",
        Timeframe::H1,
        context,
        None,
        0,
    ));
    duplicate.secondary_candle_fetch_request = Some(TradingTerminal::build_candle_fetch_request(
        10,
        "@0",
        Timeframe::H1,
        context,
        None,
        0,
    ));
    terminal.charts.insert(10, duplicate);

    let mut secondary = ChartInstance::new(11, "BTC".to_string(), Timeframe::H1);
    secondary.set_secondary_symbol_identity("@0".to_string(), "@0".to_string());
    secondary.secondary_candle_fetch_request = Some(TradingTerminal::build_candle_fetch_request(
        11,
        "@0",
        Timeframe::H1,
        context,
        None,
        0,
    ));
    terminal.charts.insert(11, secondary);

    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![
        perp_symbol("BTC"),
        canonical_purr_symbol(),
    ])));

    let duplicate = &terminal.charts[&10];
    assert_eq!(duplicate.symbol, "PURR/USDC");
    assert_eq!(duplicate.symbol_display, "PURR/USDC");
    assert_eq!(
        duplicate
            .candle_fetch_request
            .as_ref()
            .map(|request| request.symbol.as_str()),
        Some("PURR/USDC")
    );
    assert!(duplicate.secondary_symbol.is_none());
    assert!(duplicate.secondary_candle_fetch_request.is_none());

    let secondary = &terminal.charts[&11];
    assert_eq!(secondary.symbol, "BTC");
    assert_eq!(secondary.secondary_symbol.as_deref(), Some("PURR/USDC"));
    assert_eq!(
        secondary
            .secondary_candle_fetch_request
            .as_ref()
            .map(|request| request.symbol.as_str()),
        Some("PURR/USDC")
    );
    assert!(terminal.config_save_due_at.is_some());
}

#[test]
fn symbols_loaded_refreshes_existing_outcome_chart_display_without_key_change() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = "#950".to_string();
    terminal.active_symbol_display = "#950".to_string();
    terminal
        .charts
        .insert(7, ChartInstance::new(7, "#950".to_string(), Timeframe::H1));

    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![outcome_symbol("#950")])));

    let expected_display = "YES: Will BTC close green?";
    assert_eq!(terminal.active_symbol, "#950");
    assert_eq!(terminal.active_symbol_display, expected_display);
    let chart = terminal.charts.get(&7).expect("chart");
    assert_eq!(chart.symbol, "#950");
    assert_eq!(chart.symbol_display, expected_display);
}

#[test]
fn symbols_loaded_clears_stale_macro_candles_when_chart_key_is_canonicalized() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();

    let mut canonical = perp_symbol("xyz:BTC");
    canonical.ticker = "BTC".to_string();

    let mut chart = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
    chart.chart.hourly_candles = vec![Candle::test_flat(500, 50.0)];
    chart.chart.daily_candles = vec![Candle::test_flat(1_000, 100.0)];
    chart.chart.weekly_candles = vec![Candle::test_flat(2_000, 200.0)];
    chart.chart.monthly_candles = vec![Candle::test_flat(3_000, 300.0)];
    terminal.charts.insert(7, chart);

    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![canonical])));

    let chart = terminal.charts.get(&7).expect("chart");
    assert_eq!(chart.symbol, "xyz:BTC");
    assert!(chart.chart.hourly_candles.is_empty());
    assert!(chart.chart.daily_candles.is_empty());
    assert!(chart.chart.weekly_candles.is_empty());
    assert!(chart.chart.monthly_candles.is_empty());
    assert_eq!(chart.macro_candles_request_id, 1);
}

#[test]
fn symbols_loaded_disarms_hud_when_chart_symbol_key_rewrites() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();

    let mut canonical = perp_symbol("xyz:BTC");
    canonical.ticker = "BTC".to_string();

    let mut chart = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
    chart
        .chart
        .set_crosshair_style(crate::config::ChartCrosshairStyle::Hud);
    chart.chart.set_hud_armed_at(true, 1_000);
    assert!(chart.chart.hud_armed());
    terminal.charts.insert(7, chart);

    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![canonical])));

    let chart = terminal.charts.get(&7).expect("chart");
    assert_eq!(chart.symbol, "xyz:BTC");
    assert_eq!(chart.chart.symbol_key, "xyz:BTC");
    assert!(!chart.chart.hud_armed());
}

#[test]
fn symbols_loaded_resets_quick_order_when_chart_symbol_key_rewrites() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();

    let mut canonical = perp_symbol("xyz:BTC");
    canonical.ticker = "BTC".to_string();

    let mut chart = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
    chart.set_quick_order(quick_order_form());
    chart.track_last_price_update(Some(100.0), 101.0, 1_000);
    chart.heatmap_last_fetch = Some(HeatmapFetchParams {
        coin: "BTC".to_string(),
        min_price: 90.0,
        max_price: 110.0,
        start_time: 1,
        end_time: 2,
    });
    chart.heatmap_status = Some(("stale heatmap".to_string(), true));
    chart.heatmap_fetching = true;
    chart.heatmap_data = Some(LiquidationHeatmap {
        rects: Vec::new(),
        max_abs_usd: 1.0,
    });
    chart.liquidation_status = Some(("stale liquidations".to_string(), true));
    chart.liquidation_fetching = true;
    chart.liquidation_pending_key = Some("BTC".to_string());
    chart.liquidation_data = Some(LiquidationLevel {
        coin: "BTC".to_string(),
        min: 90.0,
        max: 110.0,
        liquidations: Vec::new(),
    });
    chart.chart.liquidation_buckets = vec![LiquidationBucket {
        price_center: 100.0,
        long_coins: 1.0,
        short_coins: 0.0,
        long_usd: 100.0,
        short_usd: 0.0,
    }];
    chart.chart.funding_rates = vec![FundingRatePoint {
        time_ms: 1,
        rate: 0.01,
    }];
    chart.chart.funding_status = Some(("stale funding".to_string(), false));
    terminal.charts.insert(7, chart);
    terminal
        .chart_quick_order_surface
        .insert(7, ChartSurfaceId::Docked(7));

    let _task = terminal.apply_symbols_loaded(Ok(payload(vec![canonical])));

    let chart = terminal.charts.get(&7).expect("chart");
    assert_eq!(chart.symbol, "xyz:BTC");
    assert_eq!(chart.chart.symbol_key, "xyz:BTC");
    assert!(chart.quick_order.is_none());
    assert!(!chart.chart.quick_order_open);
    assert_eq!(chart.last_quick_order_symbol, "");
    assert!(chart.last_price_flash.is_none());
    assert!(chart.heatmap_last_fetch.is_none());
    assert!(chart.heatmap_status.is_none());
    assert!(!chart.heatmap_fetching);
    assert!(chart.heatmap_data.is_none());
    assert!(chart.liquidation_status.is_none());
    assert!(!chart.liquidation_fetching);
    assert!(chart.liquidation_pending_key.is_none());
    assert!(chart.liquidation_data.is_none());
    assert!(chart.chart.liquidation_buckets.is_empty());
    assert!(chart.chart.funding_rates.is_empty());
    assert!(chart.chart.funding_status.is_none());
    assert!(!terminal.chart_quick_order_surface.contains_key(&7));
}
