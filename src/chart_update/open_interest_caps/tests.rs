use super::*;
use crate::chart_state::ChartInstance;
use crate::timeframe::Timeframe;
use iced::widget::pane_grid;

fn terminal_with_charts(symbols: &[&str]) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    let (mut panes, mut pane) = pane_grid::State::new(PaneKind::Chart(0));
    for (id, symbol) in symbols.iter().enumerate() {
        let id = id as u64;
        terminal.charts.insert(
            id,
            ChartInstance::new(id, (*symbol).to_string(), Timeframe::H1),
        );
        if id > 0 {
            pane = panes
                .split(pane_grid::Axis::Horizontal, pane, PaneKind::Chart(id))
                .expect("split chart")
                .0;
        }
    }
    terminal.panes = panes;
    terminal
}

fn caps(symbol: &str, cap: f64) -> HashMap<String, f64> {
    HashMap::from([(symbol.to_string(), cap)])
}

#[test]
fn open_interest_caps_share_requests_and_refresh_without_asset_context() {
    let mut terminal =
        terminal_with_charts(&["xyz:AAPL", "xyz:NVDA", "abc:AAPL", "BTC", "@107", "#1"]);
    assert_eq!(terminal.queue_chart_open_interest_caps(0).len(), 2);
    assert!(terminal.queue_chart_open_interest_caps(1).is_empty());
    terminal.apply_chart_open_interest_caps("xyz", Ok(caps("xyz:AAPL", 25_000_000.0)), 10);
    assert_eq!(
        terminal.chart_open_interest_cap("xyz:AAPL", 10),
        Some(25_000_000.0)
    );
    assert_eq!(terminal.chart_open_interest_cap("xyz:NVDA", 10), None);
    assert_eq!(terminal.chart_open_interest_cap("abc:AAPL", 10), None);
    assert_eq!(terminal.chart_open_interest_cap("BTC", 10), None);
    assert!(terminal.queue_chart_open_interest_caps(60_009).is_empty());
    assert_eq!(terminal.queue_chart_open_interest_caps(60_010).len(), 1);
    // A hung refresh must not leave a stale cap visible indefinitely.
    assert_eq!(terminal.chart_open_interest_cap("xyz:AAPL", 120_010), None);
}

#[test]
fn open_interest_caps_clear_failures_back_off_and_replace_removed_limits() {
    let mut terminal = terminal_with_charts(&["xyz:AAPL"]);
    let _ = terminal.queue_chart_open_interest_caps(0);
    terminal.apply_chart_open_interest_caps("xyz", Ok(caps("xyz:AAPL", 1_000.0)), 0);
    let _ = terminal.queue_chart_open_interest_caps(60_000);
    terminal.apply_chart_open_interest_caps("xyz", Err("429".to_string()), 60_000);
    assert_eq!(terminal.chart_open_interest_cap("xyz:AAPL", 60_000), None);
    assert!(terminal.queue_chart_open_interest_caps(119_999).is_empty());
    assert_eq!(terminal.queue_chart_open_interest_caps(120_000).len(), 1);
    terminal.apply_chart_open_interest_caps("xyz", Err("offline".to_string()), 120_000);
    assert!(terminal.queue_chart_open_interest_caps(239_999).is_empty());
    assert_eq!(terminal.queue_chart_open_interest_caps(240_000).len(), 1);
    terminal.apply_chart_open_interest_caps("xyz", Ok(caps("xyz:AAPL", 2_000.0)), 240_000);
    assert_eq!(
        terminal.chart_open_interest_cap("xyz:AAPL", 240_000),
        Some(2_000.0)
    );
    assert_eq!(terminal.queue_chart_open_interest_caps(300_000).len(), 1);
    terminal.apply_chart_open_interest_caps("xyz", Ok(HashMap::new()), 300_000);
    assert_eq!(terminal.chart_open_interest_cap("xyz:AAPL", 300_000), None);
}

#[test]
fn open_interest_caps_keep_in_flight_requests_across_chart_symbol_changes() {
    let mut terminal = terminal_with_charts(&["xyz:AAPL"]);
    let _ = terminal.queue_chart_open_interest_caps(0);
    terminal.charts.get_mut(&0).expect("chart").symbol = "abc:AAPL".to_string();
    assert_eq!(terminal.queue_chart_open_interest_caps(1).len(), 1);
    terminal.apply_chart_open_interest_caps("xyz", Ok(caps("xyz:AAPL", 1_000.0)), 2);
    assert_eq!(terminal.chart_open_interest_cap("abc:AAPL", 2), None);
    let _ = terminal.queue_chart_open_interest_caps(3);
    assert!(!terminal.chart_open_interest_caps.dexes.contains_key("xyz"));
    terminal.apply_chart_open_interest_caps("xyz", Ok(caps("xyz:AAPL", 2_000.0)), 4);
    assert!(!terminal.chart_open_interest_caps.dexes.contains_key("xyz"));
    terminal.charts.get_mut(&0).expect("chart").symbol = "xyz:AAPL".to_string();
    assert_eq!(terminal.queue_chart_open_interest_caps(5).len(), 1);
}

#[test]
fn open_interest_caps_skip_unused_charts_and_collapsed_headers() {
    let mut terminal = terminal_with_charts(&["BTC"]);
    terminal.charts.insert(
        99,
        ChartInstance::new(99, "xyz:AAPL".to_string(), Timeframe::H1),
    );
    assert!(terminal.queue_chart_open_interest_caps(0).is_empty());
    let chart = terminal.charts.get_mut(&0).expect("chart");
    chart.symbol = "xyz:AAPL".to_string();
    chart.header_collapsed = true;
    assert!(terminal.queue_chart_open_interest_caps(1).is_empty());
    terminal.charts.get_mut(&0).expect("chart").header_collapsed = false;
    assert_eq!(terminal.queue_chart_open_interest_caps(2).len(), 1);
}
