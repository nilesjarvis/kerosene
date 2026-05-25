use super::*;
use std::collections::HashSet;

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
