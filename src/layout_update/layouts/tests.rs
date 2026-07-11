use super::normalized_layout_name;
use crate::app_state::TradingTerminal;
use crate::config::{
    ChartConfig, KeroseneConfig, OrderPreset, SavedLayout, WidgetPaddingTargetConfig,
};
use crate::liquidations_distribution_state::{
    LiquidationDistributionData, LiquidationDistributionRequest,
};
use crate::message::Message;

#[test]
fn normalized_layout_name_trims_and_rejects_empty_names() {
    assert_eq!(
        normalized_layout_name("  Trading  "),
        Some("Trading".to_string())
    );
    assert_eq!(normalized_layout_name("   "), None);
}

#[test]
fn saving_a_named_layout_appends_activates_and_clears_input() {
    let (mut terminal, _) = TradingTerminal::boot();
    let saved_before = terminal.saved_layouts.len();
    terminal.layout_input = "Scalp".to_string();

    let _task = terminal.update_saved_layouts(Message::SaveLayout("Scalp".to_string()));

    assert_eq!(terminal.saved_layouts.len(), saved_before + 1);
    assert!(
        terminal
            .saved_layouts
            .iter()
            .any(|layout| layout.name == "Scalp")
    );
    assert_eq!(terminal.active_layout_name.as_deref(), Some("Scalp"));
    assert!(terminal.layout_input.is_empty());
}

#[test]
fn saving_a_layout_with_an_existing_name_overwrites_in_place() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.saved_layouts.push(saved_layout("Scalp"));
    let len_before = terminal.saved_layouts.len();
    // Seeded "Scalp" has no liquidation distribution symbol; the fresh snapshot
    // will, so a successful in-place overwrite is observable.
    terminal.liquidation_distribution.symbol = "BTC".to_string();
    terminal.layout_input = "Scalp".to_string();

    let _task = terminal.update_saved_layouts(Message::SaveLayout("Scalp".to_string()));

    assert_eq!(terminal.saved_layouts.len(), len_before);
    let scalp: Vec<_> = terminal
        .saved_layouts
        .iter()
        .filter(|layout| layout.name == "Scalp")
        .collect();
    assert_eq!(scalp.len(), 1);
    assert_eq!(
        scalp[0].liquidation_distribution_symbol.as_deref(),
        Some("BTC")
    );
    assert_eq!(terminal.active_layout_name.as_deref(), Some("Scalp"));
    assert!(terminal.layout_input.is_empty());
}

#[test]
fn saving_a_blank_layout_name_is_a_no_op() {
    let (mut terminal, _) = TradingTerminal::boot();
    let saved_before = terminal.saved_layouts.clone();
    terminal.layout_input = "   ".to_string();

    let _task = terminal.update_saved_layouts(Message::SaveLayout("   ".to_string()));

    assert_eq!(terminal.saved_layouts, saved_before);
    assert_eq!(terminal.layout_input, "   ");
}

fn saved_layout(name: &str) -> SavedLayout {
    let cfg = KeroseneConfig::default();
    SavedLayout {
        name: name.to_string(),
        pane_layout: cfg.pane_layout,
        layout_ratios: cfg.layout_ratios,
        charts: cfg.charts,
        order_books: cfg.order_books,
        live_watchlists: cfg.live_watchlists,
        positioning_infos: cfg.positioning_infos,
        session_data: cfg.session_data,
        x_feeds: cfg.x_feeds,
        spaghetti_charts: cfg.spaghetti_charts,
        widget_padding: cfg.widget_padding,
        active_symbol: cfg.active_symbol,
        liquidation_distribution_symbol: None,
        active_timeframe: cfg.active_timeframe,
        order_kind: cfg.order_kind,
        reduce_only: cfg.reduce_only,
        book_tick_size: cfg.book_tick_size,
        favourite_symbols: cfg.favourite_symbols,
        ticker_tape_enabled: cfg.ticker_tape_enabled,
        active_theme: cfg.active_theme,
        custom_themes: cfg.custom_themes,
        sound_enabled: cfg.sound_enabled,
        desktop_notifications: cfg.desktop_notifications,
        income_alerts_enabled: cfg.income_alerts_enabled,
        liquidation_alerts_enabled: cfg.liquidation_alerts_enabled,
        liquidation_alert_threshold: cfg.liquidation_alert_threshold,
        market_slippage_pct: cfg.market_slippage_pct,
        tracked_trade_alerts_enabled: cfg.tracked_trade_alerts_enabled,
        tracked_trade_aggregation_enabled: cfg.tracked_trade_aggregation_enabled,
        liquidation_feed_aggregation_enabled: cfg.liquidation_feed_aggregation_enabled,
        preset_is_usd: cfg.preset_is_usd,
        order_presets: cfg.order_presets,
    }
}

#[test]
fn saved_layout_debug_redacts_order_configuration_without_changing_serde() {
    const NAME: &str = "private-layout-name-sentinel";
    const SYMBOL: &str = "private-layout-symbol-sentinel";
    const ORDER_KIND: &str = "private-layout-order-kind-sentinel";
    const PRESET_LABEL: &str = "private-layout-preset-sentinel";
    const SLIPPAGE: f64 = 7.654_321;
    const PRESET_SIZE: f64 = 87_654.321;
    let mut layout = saved_layout(NAME);
    layout.active_symbol = SYMBOL.to_string();
    layout.liquidation_distribution_symbol = Some(SYMBOL.to_string());
    layout.order_kind = ORDER_KIND.to_string();
    layout.favourite_symbols = vec![SYMBOL.to_string()];
    layout.market_slippage_pct = SLIPPAGE;
    layout.order_presets.market_usd = vec![OrderPreset {
        label: PRESET_LABEL.to_string(),
        size: PRESET_SIZE,
        price_offset_pct: Some(1.234_567),
    }];
    let wire_before = serde_json::to_value(&layout).expect("serialize saved layout");

    let rendered = format!("{layout:?}");
    let wire_after = serde_json::to_value(&layout).expect("serialize layout after formatting");
    let restored: SavedLayout =
        serde_json::from_value(wire_after.clone()).expect("restore saved layout");

    assert!(rendered.contains("<redacted>"), "{rendered}");
    for sentinel in [NAME, SYMBOL, ORDER_KIND, PRESET_LABEL] {
        assert!(!rendered.contains(sentinel), "{rendered}");
    }
    assert!(!rendered.contains(&format!("{SLIPPAGE:?}")), "{rendered}");
    assert!(
        !rendered.contains(&format!("{PRESET_SIZE:?}")),
        "{rendered}"
    );
    assert!(rendered.contains("favourite_symbols_len: 1"), "{rendered}");
    assert!(
        rendered.contains("order_presets: OrderPresetsConfig"),
        "{rendered}"
    );
    assert_eq!(wire_after, wire_before);
    assert_eq!(restored, layout);
    assert_eq!(restored.market_slippage_pct.to_bits(), SLIPPAGE.to_bits());
    assert_eq!(
        restored.order_presets.market_usd[0].size.to_bits(),
        PRESET_SIZE.to_bits()
    );
}

fn liquidation_distribution_data(symbol: &str) -> LiquidationDistributionData {
    LiquidationDistributionData {
        request: LiquidationDistributionRequest::new(
            symbol.to_string(),
            symbol.to_string(),
            symbol.to_string(),
            100.0,
            0.0,
            200.0,
            1_778_357_590,
        ),
        points: Vec::new(),
        raw_count: 0,
        total_long_usd: 0.0,
        total_short_usd: 0.0,
        max_bucket_usd: 0.0,
        max_cumulative_usd: 0.0,
        fetched_at_ms: 1_778_357_590_000,
    }
}

#[test]
fn completed_layout_import_is_discarded_after_config_clear() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.config_cleared_this_session = true;
    let saved_layouts_before = terminal.saved_layouts.clone();

    let _task =
        terminal.update_saved_layouts(Message::LayoutImported(Ok(saved_layout("Imported")).into()));

    assert_eq!(terminal.saved_layouts, saved_layouts_before);
    assert!(terminal.config_save_due_at.is_none());
    assert!(
        terminal
            .toasts
            .last()
            .is_some_and(|toast| toast.is_error && toast.message.contains("import was discarded"))
    );
}

#[test]
fn layout_import_drops_unknown_widget_padding_targets() {
    let imported: SavedLayout = serde_json::from_value(serde_json::json!({
        "name": "Future Padding",
        "widget_padding": {
            "default_px": 5.0,
            "overrides": [
                {
                    "target": "Watchlist",
                    "padding_px": 12.0
                },
                {
                    "target": "RemovedPane",
                    "padding_px": 14.0
                }
            ]
        }
    }))
    .expect("saved layout with unknown padding target should deserialize");

    let (mut terminal, _) = TradingTerminal::boot();
    let _task = terminal.update_saved_layouts(Message::LayoutImported(Ok(imported).into()));

    let imported_layout = terminal
        .saved_layouts
        .iter()
        .find(|layout| layout.name == "Future Padding")
        .expect("imported layout should be saved");
    assert_eq!(imported_layout.widget_padding.default_px, 5.0);
    assert_eq!(imported_layout.widget_padding.overrides.len(), 1);
    assert_eq!(
        imported_layout.widget_padding.overrides[0].target,
        WidgetPaddingTargetConfig::Watchlist
    );
    assert_eq!(imported_layout.widget_padding.overrides[0].padding_px, 12.0);
}

#[test]
fn layout_import_preserves_unknown_future_panes() {
    let raw_layout = serde_json::json!({
        "Split": {
            "axis": "Vertical",
            "ratio": 0.5,
            "a": { "Leaf": { "Chart": { "chart_id": 7 } } },
            "b": {
                "Leaf": {
                    "FuturePane": {
                        "id": 9,
                        "label": "newer-version"
                    }
                }
            }
        }
    });
    let imported: SavedLayout = serde_json::from_value(serde_json::json!({
        "name": "Future Pane",
        "pane_layout": raw_layout.clone()
    }))
    .expect("saved layout with unknown future pane should deserialize");

    let (mut terminal, _) = TradingTerminal::boot();
    let _task = terminal.update_saved_layouts(Message::LayoutImported(Ok(imported).into()));

    let imported_layout = terminal
        .saved_layouts
        .iter()
        .find(|layout| layout.name == "Future Pane")
        .expect("imported layout should be saved");
    let pane_layout = imported_layout
        .pane_layout
        .as_ref()
        .expect("future pane layout should be retained");
    assert_eq!(
        serde_json::to_value(pane_layout).expect("future pane layout should serialize"),
        raw_layout
    );
}

#[test]
fn saved_layout_snapshot_includes_liquidation_distribution_symbol() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.liquidation_distribution.symbol = " BTC ".to_string();

    let layout = terminal.saved_layout_snapshot("Distribution".to_string());

    assert_eq!(
        layout.liquidation_distribution_symbol.as_deref(),
        Some("BTC")
    );
}

#[test]
fn applying_layout_restores_liquidation_distribution_symbol_when_present() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.liquidation_distribution.symbol = "ETH".to_string();
    terminal.liquidation_distribution.symbol_search_query = "ETH".to_string();
    terminal.liquidation_distribution.loading = true;
    terminal.liquidation_distribution.error = Some("stale".to_string());
    terminal.liquidation_distribution.data = Some(liquidation_distribution_data("ETH"));
    let expected_display = terminal.liquidation_distribution_symbol_display("BTC");

    let mut layout = saved_layout("Distribution");
    layout.liquidation_distribution_symbol = Some(" BTC ".to_string());
    let _task = terminal.apply_layout(layout);

    assert_eq!(terminal.liquidation_distribution.symbol, "BTC");
    assert_eq!(
        terminal.liquidation_distribution.symbol_search_query,
        expected_display
    );
    assert!(!terminal.liquidation_distribution.loading);
    assert!(terminal.liquidation_distribution.pending_request.is_none());
    assert!(terminal.liquidation_distribution.error.is_none());
    assert!(terminal.liquidation_distribution.data.is_none());
}

#[test]
fn applying_layout_clears_liquidation_distribution_symbol_when_present_empty() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.liquidation_distribution.symbol = "BTC".to_string();
    terminal.liquidation_distribution.symbol_search_query = "BTC".to_string();
    terminal.liquidation_distribution.loading = true;
    terminal.liquidation_distribution.error = Some("stale".to_string());

    let mut layout = saved_layout("Distribution");
    layout.liquidation_distribution_symbol = Some(String::new());
    let _task = terminal.apply_layout(layout);

    assert!(terminal.liquidation_distribution.symbol.is_empty());
    assert!(
        terminal
            .liquidation_distribution
            .symbol_search_query
            .is_empty()
    );
    assert!(!terminal.liquidation_distribution.loading);
    assert!(terminal.liquidation_distribution.pending_request.is_none());
    assert!(terminal.liquidation_distribution.error.is_none());
}

#[test]
fn applying_layout_clears_invalid_liquidation_distribution_symbol_when_present() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.liquidation_distribution.symbol = "BTC".to_string();
    terminal.liquidation_distribution.symbol_search_query = "BTC".to_string();
    terminal.liquidation_distribution.loading = true;
    terminal.liquidation_distribution.error = Some("stale".to_string());
    terminal.liquidation_distribution.data = Some(liquidation_distribution_data("BTC"));

    let mut layout = saved_layout("Distribution");
    layout.liquidation_distribution_symbol = Some("@107".to_string());
    let _task = terminal.apply_layout(layout);

    assert!(terminal.liquidation_distribution.symbol.is_empty());
    assert!(
        terminal
            .liquidation_distribution
            .symbol_search_query
            .is_empty()
    );
    assert!(!terminal.liquidation_distribution.loading);
    assert!(terminal.liquidation_distribution.pending_request.is_none());
    assert!(terminal.liquidation_distribution.error.is_none());
    assert!(terminal.liquidation_distribution.data.is_none());
}

#[test]
fn legacy_layout_without_liquidation_distribution_symbol_preserves_current_selection() {
    let legacy_layout: SavedLayout = serde_json::from_value(serde_json::json!({
        "name": "Legacy"
    }))
    .expect("legacy saved layout without distribution symbol should deserialize");
    assert!(legacy_layout.liquidation_distribution_symbol.is_none());

    let (mut terminal, _) = TradingTerminal::boot();
    terminal.liquidation_distribution.symbol = "ETH".to_string();
    terminal.liquidation_distribution.symbol_search_query = "ETH".to_string();

    let _task = terminal.apply_layout(legacy_layout);

    assert_eq!(terminal.liquidation_distribution.symbol, "ETH");
    assert_eq!(terminal.liquidation_distribution.symbol_search_query, "ETH");
}

#[test]
fn completed_layout_import_is_discarded_while_config_clear_is_pending() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.config_clear_requested = true;
    let saved_layouts_before = terminal.saved_layouts.clone();

    let _task =
        terminal.update_saved_layouts(Message::LayoutImported(Ok(saved_layout("Imported")).into()));

    assert_eq!(terminal.saved_layouts, saved_layouts_before);
    assert!(terminal.config_save_due_at.is_none());
    assert!(
        terminal
            .toasts
            .last()
            .is_some_and(|toast| toast.is_error && toast.message.contains("import was discarded"))
    );
}

#[test]
fn layout_import_start_is_blocked_after_config_clear() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.config_cleared_this_session = true;

    let _task = terminal.update_saved_layouts(Message::ImportLayout);

    assert!(
        terminal
            .toasts
            .last()
            .is_some_and(|toast| toast.is_error && toast.message.contains("import is disabled"))
    );
}

#[test]
fn cancelled_layout_io_results_remain_silent() {
    let (mut terminal, _) = TradingTerminal::boot();
    let toast_count = terminal.toasts.len();

    let _task = terminal.update_saved_layouts(Message::LayoutExported(
        Err("Export cancelled".to_string()).into(),
    ));
    let _task = terminal.update_saved_layouts(Message::LayoutImported(
        Err("Import cancelled".to_string()).into(),
    ));

    assert_eq!(terminal.toasts.len(), toast_count);
}

#[test]
fn layout_export_error_redacts_toast_detail() {
    let (mut terminal, _) = TradingTerminal::boot();

    let _task = terminal.update_saved_layouts(Message::LayoutExported(
        Err("write failed: api_key=layout-secret".to_string()).into(),
    ));

    let toast = terminal.toasts.last().expect("toast");
    assert!(toast.is_error);
    assert!(toast.message.contains("api_key=<redacted>"));
    assert!(!toast.message.contains("layout-secret"));
}

#[test]
fn layout_import_error_redacts_toast_detail() {
    let (mut terminal, _) = TradingTerminal::boot();

    let _task = terminal.update_saved_layouts(Message::LayoutImported(
        Err("parse failed: signature=sig-secret".to_string()).into(),
    ));

    let toast = terminal.toasts.last().expect("toast");
    assert!(toast.is_error);
    assert!(toast.message.contains("signature=<redacted>"));
    assert!(!toast.message.contains("sig-secret"));
}

#[test]
fn applying_layout_clears_pending_requests_for_replaced_charts() {
    let (mut terminal, _) = TradingTerminal::boot();
    let old_chart_id = terminal.charts.keys().copied().next().expect("chart");
    let replacement_chart_id = old_chart_id.saturating_add(100);
    terminal
        .heatmap_pending_charts
        .insert("heat-old".to_string(), vec![old_chart_id]);
    terminal
        .liquidation_pending_charts
        .insert("liq-old".to_string(), vec![old_chart_id]);
    terminal
        .sec_earnings_pending_charts
        .insert("NVDA".to_string(), vec![old_chart_id]);
    terminal
        .sec_earnings_pending_request_ids
        .insert("NVDA".to_string(), 7);
    terminal
        .sec_filing_summary_pending_charts
        .insert("filing-old".to_string(), vec![old_chart_id]);
    terminal
        .sec_filing_summary_pending_request_ids
        .insert("filing-old".to_string(), 8);

    let mut layout = saved_layout("Replacement");
    layout.charts = vec![ChartConfig::empty(replacement_chart_id, "BTC", "H1")];
    let _task = terminal.apply_layout(layout);

    assert!(!terminal.charts.contains_key(&old_chart_id));
    assert!(terminal.charts.contains_key(&replacement_chart_id));
    assert!(terminal.heatmap_pending_charts.is_empty());
    assert!(terminal.liquidation_pending_charts.is_empty());
    assert!(terminal.sec_earnings_pending_charts.is_empty());
    assert!(terminal.sec_earnings_pending_request_ids.is_empty());
    assert!(terminal.sec_filing_summary_pending_charts.is_empty());
    assert!(terminal.sec_filing_summary_pending_request_ids.is_empty());
}
