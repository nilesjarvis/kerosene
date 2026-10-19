use super::*;
use crate::chart_state::ChartInstance;
use crate::chart_state::OpenInterestCapSnapshot;
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

fn live_snapshot(coins: &[&str], time_ms: u64) -> OpenInterestCapStreamEvent {
    OpenInterestCapStreamEvent::Snapshot(OpenInterestCapSnapshot {
        coins: coins.iter().map(|coin| (*coin).to_string()).collect(),
        server_time_ms: time_ms,
        received_at_ms: time_ms,
    })
}

#[test]
fn open_interest_cap_stream_updates_all_dexes_and_clears_removed_warnings() {
    let mut terminal = terminal_with_charts(&["BTC", "xyz:AAPL", "abc:AAPL"]);
    terminal.apply_chart_open_interest_cap_stream(live_snapshot(&["BTC", "xyz:AAPL"], 1), 1);
    assert_eq!(terminal.chart_is_at_open_interest_cap("BTC", 1), Some(true));
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("xyz:AAPL", 1),
        Some(true)
    );
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("abc:AAPL", 1),
        Some(false)
    );
    assert_eq!(terminal.chart_is_at_open_interest_cap("@107", 1), None);
    // Live status suppresses per-DEX HTTP status reads; numeric caps still load.
    assert_eq!(terminal.queue_chart_open_interest_caps(1).len(), 2);
    terminal.apply_chart_open_interest_cap_stream(live_snapshot(&["abc:AAPL"], 5_000), 5_000);
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("xyz:AAPL", 5_000),
        Some(false)
    );
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("BTC", 5_000),
        Some(false)
    );
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("abc:AAPL", 5_000),
        Some(true)
    );
    terminal.apply_chart_open_interest_cap_stream(live_snapshot(&[], 10_000), 10_000);
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("abc:AAPL", 10_000),
        Some(false)
    );
}

#[test]
fn open_interest_cap_stream_cannot_be_overwritten_by_an_older_http_request() {
    let mut terminal = terminal_with_charts(&["BTC"]);
    assert_eq!(terminal.queue_chart_open_interest_caps(0).len(), 1);
    terminal.apply_chart_open_interest_cap_stream(live_snapshot(&["BTC"], 10), 10);
    terminal.apply_chart_open_interest_cap_status("", Ok(HashSet::new()), 20);
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("BTC", 20),
        Some(true)
    );
    terminal.apply_chart_open_interest_cap_stream(OpenInterestCapStreamEvent::Unavailable, 30);
    // The old HTTP result must not reappear as a confirmed below-cap status.
    assert_eq!(terminal.chart_is_at_open_interest_cap("BTC", 30), None);
    assert_eq!(terminal.queue_chart_open_interest_caps(30).len(), 1);
    terminal.apply_chart_open_interest_cap_status("", Ok(HashSet::new()), 40);
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("BTC", 40),
        Some(false)
    );
    terminal.apply_chart_open_interest_cap_stream(live_snapshot(&["BTC"], 50), 50);
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("BTC", 50),
        Some(true)
    );
}

#[test]
fn open_interest_cap_stream_disconnect_does_not_accept_late_pre_stream_http_data() {
    let mut terminal = terminal_with_charts(&["BTC"]);
    let _ = terminal.queue_chart_open_interest_caps(0);
    terminal.apply_chart_open_interest_cap_stream(live_snapshot(&[], 10), 10);
    terminal.apply_chart_open_interest_cap_stream(OpenInterestCapStreamEvent::Unavailable, 20);
    terminal.apply_chart_open_interest_cap_status("", Ok(HashSet::from(["BTC".to_string()])), 30);
    assert_eq!(terminal.chart_is_at_open_interest_cap("BTC", 30), None);
    assert_eq!(terminal.queue_chart_open_interest_caps(30).len(), 1);
}

#[test]
fn open_interest_cap_stream_expires_and_retries_without_waiting_a_minute() {
    let mut terminal = terminal_with_charts(&["BTC"]);
    let _ = terminal.queue_chart_open_interest_caps(0);
    terminal.apply_chart_open_interest_cap_status("", Ok(HashSet::new()), 1);
    terminal.apply_chart_open_interest_cap_stream(live_snapshot(&["BTC"], 10), 10);
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("BTC", 15_009),
        Some(true)
    );
    assert_eq!(terminal.chart_is_at_open_interest_cap("BTC", 15_010), None);
    assert_eq!(terminal.queue_chart_open_interest_caps(15_010).len(), 1);
    terminal.apply_chart_open_interest_cap_status("", Ok(HashSet::new()), 15_020);
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("BTC", 15_020),
        Some(false)
    );
}

#[test]
fn open_interest_cap_stream_rejects_old_snapshots_and_releases_unused_state() {
    let mut terminal = terminal_with_charts(&["BTC"]);
    terminal.apply_chart_open_interest_cap_stream(live_snapshot(&["BTC"], 20_000), 20_000);
    terminal.apply_chart_open_interest_cap_stream(live_snapshot(&[], 19_000), 20_001);
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("BTC", 20_001),
        Some(true)
    );
    terminal.apply_chart_open_interest_cap_stream(live_snapshot(&[], 1), 20_002);
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("BTC", 20_002),
        Some(true)
    );
    terminal.charts.get_mut(&0).expect("chart").header_collapsed = true;
    assert!(terminal.queue_chart_open_interest_caps(20_003).is_empty());
    assert_eq!(terminal.chart_is_at_open_interest_cap("BTC", 20_003), None);
    terminal.apply_chart_open_interest_cap_stream(live_snapshot(&["BTC"], 20_004), 20_004);
    assert_eq!(terminal.chart_is_at_open_interest_cap("BTC", 20_004), None);
}

#[test]
fn open_interest_caps_share_requests_and_refresh_without_asset_context() {
    let mut terminal =
        terminal_with_charts(&["xyz:AAPL", "xyz:NVDA", "abc:AAPL", "BTC", "@107", "#1"]);
    assert_eq!(terminal.queue_chart_open_interest_caps(0).len(), 5);
    assert!(!terminal.chart_open_interest_caps.dexes[""].in_flight);
    assert!(terminal.chart_open_interest_caps.dexes[""].status.in_flight);
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
    assert_eq!(terminal.queue_chart_open_interest_caps(1).len(), 2);
    terminal.apply_chart_open_interest_caps("xyz", Ok(caps("xyz:AAPL", 1_000.0)), 2);
    terminal.apply_chart_open_interest_cap_status("xyz", Ok(HashSet::new()), 2);
    assert_eq!(terminal.chart_open_interest_cap("abc:AAPL", 2), None);
    let _ = terminal.queue_chart_open_interest_caps(3);
    assert!(!terminal.chart_open_interest_caps.dexes.contains_key("xyz"));
    terminal.apply_chart_open_interest_caps("xyz", Ok(caps("xyz:AAPL", 2_000.0)), 4);
    assert!(!terminal.chart_open_interest_caps.dexes.contains_key("xyz"));
    terminal.charts.get_mut(&0).expect("chart").symbol = "xyz:AAPL".to_string();
    assert_eq!(terminal.queue_chart_open_interest_caps(5).len(), 2);
}

#[test]
fn open_interest_caps_skip_unused_charts_and_collapsed_headers() {
    let mut terminal = terminal_with_charts(&["@107"]);
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
    assert_eq!(terminal.queue_chart_open_interest_caps(2).len(), 2);
}

#[test]
fn open_interest_cap_status_refreshes_independently_of_numeric_limits() {
    let mut terminal = terminal_with_charts(&["xyz:AAPL", "xyz:NVDA", "BTC"]);
    assert_eq!(terminal.queue_chart_open_interest_caps(0).len(), 3);
    assert_eq!(terminal.chart_is_at_open_interest_cap("xyz:AAPL", 0), None);
    terminal.apply_chart_open_interest_caps("xyz", Err("unavailable".to_string()), 10);
    terminal.apply_chart_open_interest_cap_status(
        "xyz",
        Ok(HashSet::from(["xyz:AAPL".to_string()])),
        10,
    );
    terminal.apply_chart_open_interest_cap_status("", Ok(HashSet::from(["BTC".to_string()])), 10);
    for symbol in ["xyz:AAPL", "BTC"] {
        assert_eq!(terminal.chart_open_interest_cap(symbol, 10), None);
        assert_eq!(
            terminal.chart_is_at_open_interest_cap(symbol, 10),
            Some(true)
        );
    }
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("xyz:NVDA", 10),
        Some(false)
    );
    assert_eq!(terminal.chart_is_at_open_interest_cap("abc:AAPL", 10), None);
    assert_eq!(terminal.chart_is_at_open_interest_cap("@107", 10), None);
    assert_eq!(terminal.chart_is_at_open_interest_cap("#1", 10), None);
    assert!(terminal.queue_chart_open_interest_caps(60_009).is_empty());
    assert_eq!(terminal.queue_chart_open_interest_caps(60_010).len(), 3);
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("xyz:AAPL", 120_010),
        None
    );
    assert_eq!(terminal.chart_is_at_open_interest_cap("BTC", 120_010), None);
    terminal.apply_chart_open_interest_cap_status("xyz", Ok(HashSet::new()), 120_011);
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("xyz:AAPL", 120_011),
        Some(false)
    );
}

#[test]
fn open_interest_cap_status_clears_on_failure_and_backs_off_without_clearing_limits() {
    let mut terminal = terminal_with_charts(&["xyz:AAPL"]);
    let _ = terminal.queue_chart_open_interest_caps(0);
    terminal.apply_chart_open_interest_caps("xyz", Ok(caps("xyz:AAPL", 1_000.0)), 0);
    terminal.apply_chart_open_interest_cap_status(
        "xyz",
        Ok(HashSet::from(["xyz:AAPL".to_string()])),
        0,
    );
    assert_eq!(terminal.queue_chart_open_interest_caps(60_000).len(), 2);
    terminal.apply_chart_open_interest_cap_status("xyz", Err("offline".to_string()), 60_000);
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("xyz:AAPL", 60_000),
        None
    );
    assert_eq!(
        terminal.chart_open_interest_cap("xyz:AAPL", 60_000),
        Some(1_000.0)
    );
    assert!(terminal.queue_chart_open_interest_caps(119_999).is_empty());
    assert_eq!(terminal.queue_chart_open_interest_caps(120_000).len(), 1);
    terminal.apply_chart_open_interest_cap_status("xyz", Err("offline".to_string()), 120_000);
    assert!(terminal.queue_chart_open_interest_caps(239_999).is_empty());
    assert_eq!(terminal.queue_chart_open_interest_caps(240_000).len(), 1);
    terminal.apply_chart_open_interest_cap_status("xyz", Ok(HashSet::new()), 240_000);
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("xyz:AAPL", 240_000),
        Some(false)
    );
    assert_eq!(terminal.queue_chart_open_interest_caps(300_000).len(), 1);
}

#[test]
fn open_interest_cap_status_retains_pending_requests_and_ignores_late_results() {
    let mut terminal = terminal_with_charts(&["BTC"]);
    assert_eq!(terminal.queue_chart_open_interest_caps(0).len(), 1);
    terminal.charts.get_mut(&0).expect("chart").symbol = "xyz:AAPL".to_string();
    assert_eq!(terminal.queue_chart_open_interest_caps(1).len(), 2);
    assert!(terminal.chart_open_interest_caps.dexes.contains_key(""));
    terminal.charts.get_mut(&0).expect("chart").symbol = "BTC".to_string();
    assert!(terminal.queue_chart_open_interest_caps(2).is_empty());
    terminal.apply_chart_open_interest_cap_status("", Ok(HashSet::new()), 3);
    // A duplicate completion cannot overwrite a settled snapshot.
    terminal.apply_chart_open_interest_cap_status("", Ok(HashSet::from(["BTC".to_string()])), 4);
    assert_eq!(
        terminal.chart_is_at_open_interest_cap("BTC", 4),
        Some(false)
    );
    terminal.charts.get_mut(&0).expect("chart").symbol = "xyz:AAPL".to_string();
    let _ = terminal.queue_chart_open_interest_caps(5);
    assert!(!terminal.chart_open_interest_caps.dexes.contains_key(""));
    terminal.apply_chart_open_interest_cap_status("", Ok(HashSet::from(["BTC".to_string()])), 6);
    assert_eq!(terminal.chart_is_at_open_interest_cap("BTC", 6), None);
}
