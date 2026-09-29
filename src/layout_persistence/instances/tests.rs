use super::*;
use crate::api::{ExchangeSymbol, MarketType, USDC_TOKEN_INDEX};
use crate::config::{
    MAX_QUICK_TRADE_ACTIONS, QuickTradeActionConfig, QuickTradeDenomination, QuickTradeSide,
};

#[test]
fn chart_restoration_preserves_settings_and_filters_invalid_entries_in_both_paths() {
    let mut saved = config::ChartConfig::empty(7, "@0", "M5");
    saved.secondary_symbol = Some("@0".to_string());
    saved.inverted = true;
    saved.show_trade_markers = true;
    saved.show_earnings_markers = true;
    saved.header_collapsed = true;
    saved.drawing_toolbar_collapsed = true;
    saved.funding_panel_height = 80;
    saved.session_panel_height = 96;
    saved.macro_indicators.show_funding_rate = true;
    saved.macro_indicators.show_session_indicator = true;
    saved.macro_indicators.sma_50h = true;
    saved.open_interest_as_notional = true;
    saved.asset_volume_as_notional = false;
    saved.outcome_volume_as_notional = true;
    saved.annotations = serde_json::from_value(serde_json::json!([
        {"type": "level", "price": 100.0, "color": [0.2, 0.5, 0.8]},
        {"type": "unknown", "color": [0.2, 0.5, 0.8]},
        {"type": "level", "price": 200.0, "color": [0.8, 0.5, 0.2]}
    ]))
    .expect("annotation fixtures");
    saved.quick_trade_actions = (0..MAX_QUICK_TRADE_ACTIONS + 3)
        .map(|index| QuickTradeActionConfig {
            side: QuickTradeSide::Sell,
            quantity: index as f64,
            denomination: QuickTradeDenomination::Coin,
        })
        .collect();

    let mut expected = saved.clone();
    expected.annotations.remove(1);
    // Invalid entries do not consume the limit or annotation IDs.
    expected.quick_trade_actions = saved.quick_trade_actions[1..=MAX_QUICK_TRADE_ACTIONS].to_vec();
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols.clear();
    terminal.muted_tickers.clear();
    for boot in [true, false] {
        if boot {
            let (charts, tasks) = TradingTerminal::boot_chart_instances(
                std::slice::from_ref(&saved),
                &terminal.muted_tickers,
                config::ChartBackfillSource::Hyperliquid,
                &zeroize::Zeroizing::new(String::new()),
            );
            assert!(tasks.is_empty());
            terminal.charts = charts;
        } else {
            let _tasks =
                terminal.restore_layout_chart_instances(std::slice::from_ref(&saved), &[], 8, 0);
        }

        assert_eq!(terminal.chart_configs_snapshot(), [expected.clone()]);
        let instance = &terminal.charts[&7];
        assert_eq!(instance.next_annotation_id, 2);
        assert_eq!(
            instance
                .annotations
                .iter()
                .map(|ann| ann.id)
                .collect::<Vec<_>>(),
            [0, 1]
        );
        assert_eq!(
            instance
                .chart
                .annotations
                .iter()
                .map(|ann| ann.to_config())
                .collect::<Vec<_>>(),
            expected.annotations
        );
        assert_eq!(instance.chart.macro_indicators, expected.macro_indicators);
    }
}

#[test]
fn chart_restoration_preserves_comparison_settings_in_both_paths() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols.clear();
    terminal.muted_tickers.clear();
    for pair_mode in [false, true] {
        let mut saved = config::SpaghettiChartConfig::empty(7);
        saved.symbols = vec!["@0".to_string()];
        saved.watchlist_preset_id = Some(42);
        saved.timeframe = "M15".to_string();
        saved.pair_mode = pair_mode;
        saved.pair_candle_mode = true;
        saved.color_mode = spaghetti::ComparisonColorMode::Single;
        saved.show_labels = true;
        saved.anchor = Some("utc_day".to_string());
        saved.anchor_granularity = Some("H1".to_string());
        let mut expected = saved.clone();
        if pair_mode {
            expected.watchlist_preset_id = None;
        }
        for boot in [true, false] {
            if boot {
                let (charts, tasks) = TradingTerminal::boot_spaghetti_instances(
                    std::slice::from_ref(&saved),
                    &terminal.muted_tickers,
                    config::ChartBackfillSource::Hyperliquid,
                    &zeroize::Zeroizing::new(String::new()),
                );
                assert!(tasks.is_empty());
                terminal.spaghetti_charts = charts;
            } else {
                let _tasks = terminal.restore_layout_chart_instances(
                    &[],
                    std::slice::from_ref(&saved),
                    0,
                    8,
                );
            }
            assert_eq!(
                terminal.spaghetti_chart_configs_snapshot(),
                [expected.clone()]
            );
            let instance = &terminal.spaghetti_charts[&7];
            assert_eq!(instance.canvas.pair_ratio_mode, pair_mode);
            assert!(instance.canvas.pair_candle_mode);
            assert!(!instance.editor_open);
        }
    }
}

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "test".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 4,
        max_leverage: 1,
        only_isolated: false,
        growth_mode: false,
        market_type,
        outcome: None,
    }
}

fn canonical_purr_symbol() -> ExchangeSymbol {
    ExchangeSymbol {
        ticker: "PURR".to_string(),
        category: "spot".to_string(),
        display_name: Some("PURR/USDC".to_string()),
        asset_index: 10_000,
        collateral_token: Some(USDC_TOKEN_INDEX),
        sz_decimals: 0,
        ..symbol("PURR/USDC", MarketType::Spot)
    }
}

#[test]
fn runtime_layout_restore_canonicalizes_regular_chart_spot_aliases_before_fetch() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![
        symbol("BTC", MarketType::Perp),
        symbol("ETH", MarketType::Perp),
        canonical_purr_symbol(),
    ];

    let mut legacy_primary = config::ChartConfig::empty(1, "@0", "H1");
    legacy_primary.secondary_symbol = Some("ETH".to_string());
    let mut legacy_secondary = config::ChartConfig::empty(2, "BTC", "H1");
    legacy_secondary.secondary_symbol = Some("@0".to_string());
    let mut alias_collision = config::ChartConfig::empty(3, "PURR/USDC", "H1");
    alias_collision.secondary_symbol = Some("@0".to_string());
    let mut unchanged_perps = config::ChartConfig::empty(4, "BTC", "H1");
    unchanged_perps.secondary_symbol = Some("ETH".to_string());

    let _tasks = terminal.restore_layout_chart_instances(
        &[
            legacy_primary,
            legacy_secondary,
            alias_collision,
            unchanged_perps,
        ],
        &[],
        5,
        0,
    );

    let primary = &terminal.charts[&1];
    assert_eq!(primary.symbol, "PURR/USDC");
    assert_eq!(primary.symbol_display, "PURR/USDC");
    assert_eq!(
        primary
            .candle_fetch_request
            .as_ref()
            .map(|request| request.symbol.as_str()),
        Some("PURR/USDC")
    );
    assert_eq!(primary.secondary_symbol.as_deref(), Some("ETH"));

    let secondary = &terminal.charts[&2];
    assert_eq!(secondary.symbol, "BTC");
    assert_eq!(secondary.secondary_symbol.as_deref(), Some("PURR/USDC"));
    assert_eq!(
        secondary
            .secondary_candle_fetch_request
            .as_ref()
            .map(|request| request.symbol.as_str()),
        Some("PURR/USDC")
    );

    let deduplicated = &terminal.charts[&3];
    assert_eq!(deduplicated.symbol, "PURR/USDC");
    assert!(deduplicated.secondary_symbol.is_none());
    assert!(deduplicated.secondary_candle_fetch_request.is_none());

    let perps = &terminal.charts[&4];
    assert_eq!(perps.symbol, "BTC");
    assert_eq!(perps.secondary_symbol.as_deref(), Some("ETH"));
    assert_eq!(
        perps
            .candle_fetch_request
            .as_ref()
            .map(|request| request.symbol.as_str()),
        Some("BTC")
    );
    assert_eq!(
        perps
            .secondary_candle_fetch_request
            .as_ref()
            .map(|request| request.symbol.as_str()),
        Some("ETH")
    );

    assert!(terminal.charts.values().all(|chart| {
        chart
            .candle_fetch_request
            .as_ref()
            .map(|request| request.symbol.as_str())
            != Some("@0")
            && chart
                .secondary_candle_fetch_request
                .as_ref()
                .map(|request| request.symbol.as_str())
                != Some("@0")
    }));
}

#[test]
fn runtime_layout_restore_defers_unresolved_legacy_spot_aliases() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols.clear();
    let mut config = config::ChartConfig::empty(7, "@0", "H1");
    config.secondary_symbol = Some("@0".to_string());

    let _tasks = terminal.restore_layout_chart_instances(&[config], &[], 8, 0);

    let chart = &terminal.charts[&7];
    assert_eq!(chart.symbol, "@0");
    assert_eq!(chart.secondary_symbol.as_deref(), Some("@0"));
    assert!(chart.candle_fetch_request.is_none());
    assert!(chart.secondary_candle_fetch_request.is_none());
    assert_eq!(chart.macro_candles_request_id, 0);
}
