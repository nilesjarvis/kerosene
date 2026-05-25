use super::*;
use std::collections::HashSet;

#[test]
fn saved_layout_snapshots_exclude_detached_chart_clones() {
    let chart_id = 7;
    let mut terminal = terminal_with_chart(chart_id);
    let _task = terminal.open_detached_chart_window(chart_id);
    let (_, detached_chart_id) = first_detached_window(&terminal);

    let all_chart_ids: HashSet<_> = terminal
        .chart_configs_snapshot()
        .into_iter()
        .map(|chart| chart.id)
        .collect();
    let saved_layout_ids: HashSet<_> = terminal
        .saved_layout_snapshot("test".to_string())
        .charts
        .into_iter()
        .map(|chart| chart.id)
        .collect();
    let detached_window_ids: HashSet<_> = terminal
        .detached_chart_window_configs_snapshot()
        .into_iter()
        .map(|window| window.chart_id)
        .collect();

    assert!(all_chart_ids.contains(&chart_id));
    assert!(all_chart_ids.contains(&detached_chart_id));
    assert_eq!(saved_layout_ids, HashSet::from([chart_id]));
    assert_eq!(detached_window_ids, HashSet::from([detached_chart_id]));
}
