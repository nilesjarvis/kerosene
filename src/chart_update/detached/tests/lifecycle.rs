use super::*;
use crate::annotations::DrawingTool;
use crate::api::{ExchangeSymbol, MarketType};
use crate::chart::{ChartViewport, EarningsMarker};
use std::collections::HashSet;

fn exchange_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "test".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

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
fn open_detached_chart_window_clones_source_chart() {
    let chart_id = 7;
    let mut terminal = terminal_with_chart(chart_id);

    let _task = terminal.open_detached_chart_window(chart_id);
    let (window_id, detached_chart_id) = first_detached_window(&terminal);

    assert_ne!(detached_chart_id, chart_id);
    assert_eq!(terminal.detached_chart_windows.len(), 1);
    assert_eq!(
        chart_instance(&terminal, chart_id).chart.surface_id(),
        ChartSurfaceId::Docked(chart_id)
    );
    assert_eq!(chart_instance(&terminal, chart_id).symbol, "BTC");
    assert_eq!(chart_instance(&terminal, detached_chart_id).symbol, "BTC");
    assert_eq!(
        chart_instance(&terminal, detached_chart_id)
            .chart
            .surface_id(),
        ChartSurfaceId::Detached(window_id)
    );

    let _task = terminal.update_chart(Message::ChartSymbolSelected(
        detached_chart_id,
        "ETH".into(),
    ));

    assert_eq!(chart_instance(&terminal, chart_id).symbol, "BTC");
    assert_eq!(chart_instance(&terminal, detached_chart_id).symbol, "ETH");
}

#[test]
fn detached_chart_clone_joins_active_earnings_request() {
    let chart_id = 7;
    let mut terminal = terminal_with_chart(chart_id);
    {
        let source = chart_instance_mut(&mut terminal, chart_id);
        source.show_earnings_markers = true;
        source.earnings_fetching = true;
        source.earnings_pending_ticker = Some("NVDA".to_string());
        source.earnings_status = Some(("EARN loading".to_string(), false));
    }
    terminal
        .sec_earnings_pending_request_ids
        .insert("NVDA".to_string(), 11);
    terminal
        .sec_earnings_pending_charts
        .insert("NVDA".to_string(), vec![chart_id]);

    let _task = terminal.open_detached_chart_window(chart_id);
    let (_, detached_chart_id) = first_detached_window(&terminal);

    assert_eq!(
        terminal.sec_earnings_pending_charts.get("NVDA"),
        Some(&vec![chart_id, detached_chart_id])
    );
    let detached = chart_instance(&terminal, detached_chart_id);
    assert!(detached.earnings_fetching);
    assert_eq!(detached.earnings_pending_ticker.as_deref(), Some("NVDA"));
    assert_eq!(
        detached.earnings_status.as_ref(),
        Some(&("EARN loading".to_string(), false))
    );
}

#[test]
fn detached_chart_clone_joins_active_filing_summary_request() {
    let chart_id = 7;
    let mut terminal = terminal_with_chart(chart_id);
    let filing_key = "1045819:0001045819-26-000001:nvda-20260528.htm".to_string();
    {
        let source = chart_instance_mut(&mut terminal, chart_id);
        source.show_earnings_markers = true;
        source.chart.set_earnings_markers(vec![EarningsMarker {
            time_ms: 1_779_926_400_000,
            cik: 1_045_819,
            form: "8-K".to_string(),
            filing_date: "2026-05-28".to_string(),
            accession_number: "0001045819-26-000001".to_string(),
            primary_document: "nvda-20260528.htm".to_string(),
            quarter_label: Some("Q1 2026".to_string()),
            filing_summary: None,
            filing_summary_status: None,
            filing_summary_loading: true,
        }]);
    }
    terminal
        .sec_filing_summary_pending_request_ids
        .insert(filing_key.clone(), 13);
    terminal
        .sec_filing_summary_pending_charts
        .insert(filing_key.clone(), vec![chart_id]);

    let _task = terminal.open_detached_chart_window(chart_id);
    let (_, detached_chart_id) = first_detached_window(&terminal);

    assert_eq!(
        terminal.sec_filing_summary_pending_charts.get(&filing_key),
        Some(&vec![chart_id, detached_chart_id])
    );
    let detached = chart_instance(&terminal, detached_chart_id);
    assert!(detached.chart.earnings_markers[0].filing_summary_loading);
}

#[test]
fn open_detached_chart_window_can_create_multiple_independent_windows() {
    let chart_id = 7;
    let mut terminal = terminal_with_chart(chart_id);

    let _task = terminal.open_detached_chart_window(chart_id);
    let _task = terminal.open_detached_chart_window(chart_id);

    let detached_ids: HashSet<_> = terminal
        .detached_chart_windows
        .values()
        .map(|state| state.chart_id)
        .collect();

    assert_eq!(terminal.detached_chart_windows.len(), 2);
    assert_eq!(detached_ids.len(), 2);
    assert!(!detached_ids.contains(&chart_id));
    assert!(
        detached_ids
            .iter()
            .all(|id| terminal.charts.contains_key(id))
    );
    assert!(terminal.charts.contains_key(&chart_id));
}

#[test]
fn closing_detached_chart_window_removes_only_detached_chart_clone() {
    let chart_id = 7;
    let mut terminal = terminal_with_chart(chart_id);
    let _task = terminal.open_detached_chart_window(chart_id);
    let (window_id, detached_chart_id) = first_detached_window(&terminal);

    terminal.remove_detached_chart_window_state(window_id);

    assert!(terminal.charts.contains_key(&chart_id));
    assert!(!terminal.charts.contains_key(&detached_chart_id));
}

#[test]
fn active_symbol_switch_retargets_focused_detached_clone_without_mutating_source_chart() {
    let chart_id = 7;
    let mut terminal = terminal_with_chart(chart_id);
    terminal.exchange_symbols = vec![exchange_symbol("BTC"), exchange_symbol("ETH")];

    let _task = terminal.open_detached_chart_window(chart_id);
    let (window_id, detached_chart_id) = first_detached_window(&terminal);
    let detached_surface_id = ChartSurfaceId::Detached(window_id);
    terminal.primary_chart_id = Some(detached_chart_id);
    chart_instance_mut(&mut terminal, detached_chart_id).set_quick_order(quick_order_form());
    chart_instance_mut(&mut terminal, detached_chart_id)
        .chart
        .active_tool = Some(DrawingTool::TrendLine);
    terminal
        .chart_quick_order_surface
        .insert(detached_chart_id, detached_surface_id);
    terminal
        .chart_surface_active_tools
        .insert(detached_surface_id, DrawingTool::TrendLine);
    terminal
        .chart_surface_viewports
        .insert(detached_surface_id, chart_viewport());
    terminal.chart_screenshot_menu_open = Some(detached_surface_id);

    let _task = terminal.switch_active_symbol_internal("ETH".to_string());

    assert_eq!(chart_instance(&terminal, chart_id).symbol, "BTC");
    assert_eq!(chart_instance(&terminal, detached_chart_id).symbol, "ETH");
    assert_eq!(
        chart_instance(&terminal, detached_chart_id)
            .chart
            .surface_id(),
        detached_surface_id
    );
    assert!(
        chart_instance(&terminal, detached_chart_id)
            .quick_order
            .is_none()
    );
    assert!(
        chart_instance(&terminal, detached_chart_id)
            .chart
            .active_tool
            .is_none()
    );
    assert!(
        !terminal
            .chart_quick_order_surface
            .contains_key(&detached_chart_id)
    );
    assert!(
        !terminal
            .chart_surface_active_tools
            .contains_key(&detached_surface_id)
    );
    assert!(
        !terminal
            .chart_surface_viewports
            .contains_key(&detached_surface_id)
    );
    assert_eq!(terminal.chart_screenshot_menu_open, None);
}

#[test]
fn closing_focused_detached_clone_repairs_primary_chart_before_next_active_symbol_switch() {
    let chart_id = 7;
    let mut terminal = terminal_with_chart(chart_id);
    terminal.exchange_symbols = vec![exchange_symbol("BTC"), exchange_symbol("ETH")];

    let _task = terminal.open_detached_chart_window(chart_id);
    let (window_id, detached_chart_id) = first_detached_window(&terminal);
    terminal.primary_chart_id = Some(detached_chart_id);

    assert!(terminal.remove_detached_chart_window_state(window_id));

    assert!(!terminal.charts.contains_key(&detached_chart_id));
    assert_eq!(terminal.primary_chart_id, Some(chart_id));

    let _task = terminal.switch_active_symbol_internal("ETH".to_string());

    assert_eq!(terminal.primary_chart_id, Some(chart_id));
    assert_eq!(chart_instance(&terminal, chart_id).symbol, "ETH");
    assert_eq!(terminal.active_symbol, "ETH");
}

#[test]
fn closing_detached_chart_window_prunes_pending_request_registries() {
    let chart_id = 7;
    let other_chart_id = 99;
    let mut terminal = terminal_with_chart(chart_id);
    let _task = terminal.open_detached_chart_window(chart_id);
    let (window_id, detached_chart_id) = first_detached_window(&terminal);
    seed_chart_pending_requests(&mut terminal, detached_chart_id, other_chart_id);

    terminal.remove_detached_chart_window_state(window_id);

    assert!(!terminal.charts.contains_key(&detached_chart_id));
    assert_chart_pending_requests_pruned(&terminal, other_chart_id);
}
