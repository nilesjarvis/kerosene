use super::min_size::clamp_split_ratio;
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use crate::pane_state::PaneKind;

#[test]
fn clamp_split_ratio_recovers_from_non_finite_ratio() {
    let ratio = clamp_split_ratio(f32::NAN, 500.0, 50.0, 50.0, false, false, 4.0);

    assert_eq!(ratio, 0.5);
}

#[test]
fn clamp_split_ratio_handles_invalid_axis_length() {
    let ratio = clamp_split_ratio(0.25, f32::NAN, 50.0, 50.0, false, false, 4.0);

    assert_eq!(ratio, 0.25);
}

#[test]
fn closing_chart_pane_prunes_pending_request_registries() {
    let (mut terminal, _) = TradingTerminal::boot();
    let chart_pane = terminal
        .find_pane_matching(|kind| matches!(kind, PaneKind::Chart(_)))
        .expect("chart pane");
    let chart_id = match terminal.panes.get(chart_pane).expect("chart pane kind") {
        PaneKind::Chart(chart_id) => *chart_id,
        _ => panic!("expected chart pane"),
    };
    let other_chart_id = chart_id.saturating_add(100);
    seed_pending_chart_requests(&mut terminal, chart_id, other_chart_id);

    let _task = terminal.update_pane_interactions(Message::ClosePane(chart_pane));

    assert!(!terminal.charts.contains_key(&chart_id));
    assert_pending_chart_requests_pruned(&terminal, other_chart_id);
}

fn seed_pending_chart_requests(
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

fn assert_pending_chart_requests_pruned(terminal: &TradingTerminal, other_chart_id: ChartId) {
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
