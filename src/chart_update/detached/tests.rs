use super::*;
use crate::chart_state::ChartInstance;
use crate::order_execution::QuickOrderForm;
use crate::pane_state::PaneKind;
use crate::timeframe::Timeframe;
use iced::{widget::pane_grid, window};

mod lifecycle;
mod snapshots;
mod surface_state;

fn terminal_with_chart(chart_id: ChartId) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    let (panes, _) = pane_grid::State::new(PaneKind::Chart(chart_id));
    terminal.panes = panes;
    terminal.charts.clear();
    terminal.detached_chart_windows.clear();
    terminal.chart_surface_active_tools.clear();
    terminal.chart_surface_viewports.clear();
    terminal.chart_quick_order_surface.clear();
    terminal.next_chart_id = chart_id + 1;
    terminal.charts.insert(
        chart_id,
        ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1),
    );
    terminal
}

fn first_detached_window(terminal: &TradingTerminal) -> (window::Id, ChartId) {
    match terminal
        .detached_chart_windows
        .iter()
        .map(|(window_id, state)| (*window_id, state.chart_id))
        .next()
    {
        Some(detached_window) => detached_window,
        None => panic!("detached chart window"),
    }
}

fn chart_instance(terminal: &TradingTerminal, chart_id: ChartId) -> &ChartInstance {
    match terminal.charts.get(&chart_id) {
        Some(instance) => instance,
        None => panic!("chart instance {chart_id}"),
    }
}

fn chart_instance_mut(terminal: &mut TradingTerminal, chart_id: ChartId) -> &mut ChartInstance {
    match terminal.charts.get_mut(&chart_id) {
        Some(instance) => instance,
        None => panic!("chart instance {chart_id}"),
    }
}

fn seed_chart_pending_requests(
    terminal: &mut TradingTerminal,
    chart_id: ChartId,
    other_chart_id: ChartId,
) {
    terminal
        .heatmap_pending_charts
        .insert("heat-shared".to_string(), vec![chart_id, other_chart_id]);
    terminal
        .heatmap_pending_charts
        .insert("heat-only".to_string(), vec![chart_id]);
    terminal
        .liquidation_pending_charts
        .insert("liq-shared".to_string(), vec![chart_id, other_chart_id]);
    terminal
        .liquidation_pending_charts
        .insert("liq-only".to_string(), vec![chart_id]);
    terminal
        .sec_earnings_pending_charts
        .insert("NVDA".to_string(), vec![chart_id, other_chart_id]);
    terminal
        .sec_earnings_pending_charts
        .insert("TSLA".to_string(), vec![chart_id]);
    terminal
        .sec_earnings_pending_request_ids
        .insert("NVDA".to_string(), 7);
    terminal
        .sec_earnings_pending_request_ids
        .insert("TSLA".to_string(), 8);
}

fn assert_chart_pending_requests_pruned(terminal: &TradingTerminal, other_chart_id: ChartId) {
    assert_eq!(
        terminal.heatmap_pending_charts.get("heat-shared"),
        Some(&vec![other_chart_id])
    );
    assert!(!terminal.heatmap_pending_charts.contains_key("heat-only"));
    assert_eq!(
        terminal.liquidation_pending_charts.get("liq-shared"),
        Some(&vec![other_chart_id])
    );
    assert!(!terminal.liquidation_pending_charts.contains_key("liq-only"));
    assert_eq!(
        terminal.sec_earnings_pending_charts.get("NVDA"),
        Some(&vec![other_chart_id])
    );
    assert_eq!(
        terminal.sec_earnings_pending_request_ids.get("NVDA"),
        Some(&7)
    );
    assert!(!terminal.sec_earnings_pending_charts.contains_key("TSLA"));
    assert!(
        !terminal
            .sec_earnings_pending_request_ids
            .contains_key("TSLA")
    );
}
