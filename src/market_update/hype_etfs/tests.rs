use super::*;

use crate::hype_etf_state::HypeEtfData;

use iced::widget::pane_grid;

fn open_hype_etf_pane(terminal: &mut TradingTerminal) {
    let (panes, pane) = pane_grid::State::new(PaneKind::Chart(0));
    terminal.panes = panes;
    terminal
        .panes
        .split(pane_grid::Axis::Vertical, pane, PaneKind::HypeEtfs)
        .expect("split should create HYPE ETF pane");
}

#[test]
fn boot_refresh_only_starts_when_hype_etf_pane_is_open() {
    let (mut terminal, _) = TradingTerminal::boot();
    let (panes, pane) = pane_grid::State::new(PaneKind::Chart(0));
    terminal.panes = panes;
    terminal.hype_etfs.loading = false;

    let _task = terminal.request_hype_etfs_boot_refresh();
    assert!(!terminal.hype_etfs.loading);

    terminal
        .panes
        .split(pane_grid::Axis::Vertical, pane, PaneKind::HypeEtfs)
        .expect("split should create HYPE ETF pane");

    let _task = terminal.request_hype_etfs_boot_refresh();
    assert!(terminal.hype_etfs.loading);
    assert_eq!(terminal.hype_etfs.refresh_request_id, 1);
}

#[test]
fn stale_loaded_message_does_not_clear_active_refresh() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.hype_etfs.loading = true;
    terminal.hype_etfs.refresh_request_id = 2;
    terminal.hype_etfs.error = Some("current error".to_string());

    let _task = terminal.update_hype_etfs_market(Message::HypeEtfsLoaded(
        1,
        Box::new(Ok(HypeEtfData::default())),
    ));

    assert!(terminal.hype_etfs.loading);
    assert!(terminal.hype_etfs.data.is_none());
    assert_eq!(terminal.hype_etfs.error.as_deref(), Some("current error"));
    assert!(terminal.hype_etfs.last_fetch.is_none());
}

#[test]
fn duplicate_loaded_message_after_completion_is_ignored() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.hype_etfs.loading = false;
    terminal.hype_etfs.refresh_request_id = 7;
    terminal.hype_etfs.data = Some(HypeEtfData {
        funds: Vec::new(),
        warnings: vec!["accepted".to_string()],
    });

    let _task = terminal.update_hype_etfs_market(Message::HypeEtfsLoaded(
        7,
        Box::new(Ok(HypeEtfData {
            funds: Vec::new(),
            warnings: vec!["duplicate".to_string()],
        })),
    ));

    let data = terminal
        .hype_etfs
        .data
        .as_ref()
        .expect("accepted data should remain cached");
    assert_eq!(data.warnings, vec!["accepted"]);
}

#[test]
fn failed_refresh_does_not_mark_hype_etfs_fresh() {
    let (mut terminal, _) = TradingTerminal::boot();
    open_hype_etf_pane(&mut terminal);
    terminal.hype_etfs.loading = true;
    terminal.hype_etfs.refresh_request_id = 1;

    let _task = terminal.update_hype_etfs_market(Message::HypeEtfsLoaded(
        1,
        Box::new(Err("network down".to_string())),
    ));

    assert!(!terminal.hype_etfs.loading);
    assert_eq!(terminal.hype_etfs.error.as_deref(), Some("network down"));
    assert!(terminal.hype_etfs.last_fetch.is_none());

    let _task = terminal.request_hype_etfs_refresh(false);

    assert!(terminal.hype_etfs.loading);
    assert_eq!(terminal.hype_etfs.refresh_request_id, 2);
}

#[test]
fn hype_etf_tick_refresh_is_ignored_when_pane_is_closed() {
    let (mut terminal, _) = TradingTerminal::boot();
    let (panes, _) = pane_grid::State::new(PaneKind::Chart(0));
    terminal.panes = panes;

    let _task = terminal.update_hype_etfs_market(Message::HypeEtfsRefreshTick);

    assert!(!terminal.hype_etfs.loading);
    assert_eq!(terminal.hype_etfs.refresh_request_id, 0);
}
