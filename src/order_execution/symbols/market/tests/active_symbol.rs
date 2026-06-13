use super::*;
use crate::annotations::DrawingTool;
use crate::chart::ChartViewport;
use crate::chart_state::{ChartInstance, ChartSurfaceId};
use crate::order_execution::QuickOrderForm;
use crate::timeframe::Timeframe;

fn quick_order_form() -> QuickOrderForm {
    QuickOrderForm {
        price: 100.0,
        quantity: "2.5".to_string(),
        quantity_is_usd: false,
        percentage: 25.0,
        quantity_provenance: None,
        is_limit: true,
        click_x: 10.0,
        click_y: 20.0,
        chart_w: 300.0,
        chart_h: 200.0,
    }
}

fn chart_viewport() -> ChartViewport {
    ChartViewport {
        start_time_ms: 1,
        end_time_ms: 2,
        price_lo: 90.0,
        price_hi: 110.0,
        chart_width: 500.0,
        candle_width: 10.0,
        scroll_offset: 0.0,
        y_auto: true,
        y_scale: 1.0,
        y_offset: 0.0,
        funding_y_scale: 1.0,
        funding_y_offset: 0.0,
    }
}

#[test]
fn switch_active_symbol_clears_order_sizing_inputs() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![
        symbol("BTC", MarketType::Perp),
        symbol("ETH", MarketType::Perp),
    ];
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.order_quantity = "1500".to_string();
    terminal.order_percentage = 75.0;
    terminal.order_quantity_is_usd = true;

    let _task = terminal.switch_active_symbol_internal("ETH".to_string());

    assert_eq!(terminal.active_symbol, "ETH");
    assert_eq!(terminal.active_symbol_display, "ETH");
    assert!(terminal.order_quantity.is_empty());
    assert_eq!(terminal.order_percentage, 0.0);
    assert!(terminal.order_quantity_is_usd);
}

#[test]
fn switch_active_symbol_does_not_remember_old_quick_order_size_under_new_symbol() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![
        symbol("BTC", MarketType::Perp),
        symbol("ETH", MarketType::Perp),
    ];
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.primary_chart_id = Some(7);
    terminal.charts.clear();
    terminal
        .charts
        .insert(7, ChartInstance::new(7, "BTC".to_string(), Timeframe::H1));
    terminal
        .charts
        .get_mut(&7)
        .expect("chart")
        .set_quick_order(quick_order_form());
    terminal
        .charts
        .get_mut(&7)
        .expect("chart")
        .chart
        .set_current_spread_at(Some(0.25), 1_000);
    terminal
        .chart_quick_order_surface
        .insert(7, ChartSurfaceId::Docked(7));
    terminal
        .chart_surface_active_tools
        .insert(ChartSurfaceId::Docked(7), DrawingTool::TrendLine);
    terminal
        .chart_surface_viewports
        .insert(ChartSurfaceId::Docked(7), chart_viewport());
    terminal.chart_screenshot_menu_open = Some(ChartSurfaceId::Docked(7));

    let _task = terminal.switch_active_symbol_internal("ETH".to_string());

    let instance = terminal.charts.get(&7).expect("chart");
    assert_eq!(instance.symbol, "ETH");
    assert_eq!(instance.chart.symbol_key, "ETH");
    assert!(instance.quick_order.is_none());
    assert_eq!(instance.chart.current_spread, None);
    assert_eq!(instance.chart.spread_history_bounds(), None);
    assert!(instance.chart.active_tool.is_none());
    assert!(!terminal.chart_quick_order_surface.contains_key(&7));
    assert!(
        !terminal
            .chart_surface_active_tools
            .contains_key(&ChartSurfaceId::Docked(7))
    );
    assert!(
        !terminal
            .chart_surface_viewports
            .contains_key(&ChartSurfaceId::Docked(7))
    );
    assert_eq!(terminal.chart_screenshot_menu_open, None);
    assert_eq!(
        instance.quick_order_reopen_values(true),
        (String::new(), true, 0.0, true)
    );
}

#[test]
fn switch_to_non_tradable_outcome_reports_display_label_not_raw_key() {
    let mut terminal = TradingTerminal::boot().0;
    let fallback = outcome_symbol("#660", true);
    let label = TradingTerminal::exchange_symbol_display_name(&fallback);
    terminal.exchange_symbols = vec![fallback, symbol("HYPE", MarketType::Perp)];

    let _task = terminal.switch_active_symbol_internal("#660".to_string());

    let (message, is_error) = match &terminal.order_status {
        Some((message, is_error)) => (message.clone(), *is_error),
        None => panic!("status should be set"),
    };
    assert!(is_error);
    assert_eq!(message, format!("{label} is not a tradable market"));
    assert!(!message.contains("#660"));
    match &terminal.symbol_search_status {
        Some((search_message, _)) => assert_eq!(search_message, &message),
        None => panic!("search status should be set"),
    }
}

#[test]
fn restored_active_symbol_key_replaces_non_tradable_fallback_outcome() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![
        outcome_symbol("#660", true),
        outcome_symbol("#670", false),
        symbol("HYPE", MarketType::Perp),
    ];

    assert_eq!(
        terminal.restored_active_symbol_key("#660"),
        Some("HYPE".to_string())
    );
}
