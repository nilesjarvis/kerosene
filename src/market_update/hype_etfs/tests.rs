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
fn hype_etf_refresh_wraps_request_id_without_replacing_active_owner() {
    let (mut terminal, _) = TradingTerminal::boot();
    open_hype_etf_pane(&mut terminal);
    terminal.hype_etfs.refresh_request_id = u64::MAX;

    let _task = terminal.request_hype_etfs_refresh(false);

    assert!(terminal.hype_etfs.loading);
    assert_eq!(terminal.hype_etfs.refresh_request_id, 0);

    let _forced_task = terminal.request_hype_etfs_refresh(true);

    assert!(terminal.hype_etfs.loading);
    assert_eq!(terminal.hype_etfs.refresh_request_id, 0);
}

#[test]
fn active_hype_etf_owner_survives_pane_reconstruction_and_accepts_exact_data() {
    let (mut terminal, _) = TradingTerminal::boot();
    open_hype_etf_pane(&mut terminal);
    let _task = terminal.request_hype_etfs_refresh(false);
    let request_id = terminal.hype_etfs.refresh_request_id;

    let (panes, _) = pane_grid::State::new(PaneKind::Chart(0));
    terminal.panes = panes;
    let _closed_task = terminal.request_hype_etfs_refresh(false);
    open_hype_etf_pane(&mut terminal);
    let _reopened_task = terminal.request_hype_etfs_refresh(false);

    assert!(terminal.hype_etfs.loading);
    assert_eq!(terminal.hype_etfs.refresh_request_id, request_id);

    let _completion_task = terminal.update_hype_etfs_market(Message::HypeEtfsLoaded(
        request_id,
        Ok(HypeEtfData {
            funds: Vec::new(),
            warnings: vec!["exact warning".to_string()],
        })
        .into(),
    ));

    assert!(!terminal.hype_etfs.loading);
    let data = terminal
        .hype_etfs
        .data
        .as_ref()
        .expect("current completion should install data");
    assert_eq!(data.warnings, vec!["exact warning"]);
    assert!(terminal.hype_etfs.error.is_none());
    assert!(terminal.hype_etfs.last_fetch.is_some());
}

#[test]
fn stale_loaded_message_does_not_clear_active_refresh() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.hype_etfs.loading = true;
    terminal.hype_etfs.refresh_request_id = 2;
    terminal.hype_etfs.error = Some("current error".to_string());

    let _task = terminal.update_hype_etfs_market(Message::HypeEtfsLoaded(
        1,
        Ok(HypeEtfData::default()).into(),
    ));

    assert!(terminal.hype_etfs.loading);
    assert!(terminal.hype_etfs.data.is_none());
    assert_eq!(terminal.hype_etfs.error.as_deref(), Some("current error"));
    assert!(terminal.hype_etfs.last_fetch.is_none());
}

#[test]
fn stale_error_does_not_change_current_hype_etf_cache_or_owner() {
    let (mut terminal, _) = TradingTerminal::boot();
    let last_fetch = std::time::Instant::now();
    terminal.hype_etfs.loading = true;
    terminal.hype_etfs.refresh_request_id = 9;
    terminal.hype_etfs.data = Some(HypeEtfData {
        funds: Vec::new(),
        warnings: vec!["current".to_string()],
    });
    terminal.hype_etfs.error = Some("current error".to_string());
    terminal.hype_etfs.last_fetch = Some(last_fetch);

    let _task = terminal.update_hype_etfs_market(Message::HypeEtfsLoaded(
        8,
        Err("stale error".to_string()).into(),
    ));

    assert!(terminal.hype_etfs.loading);
    assert_eq!(terminal.hype_etfs.refresh_request_id, 9);
    assert_eq!(
        terminal
            .hype_etfs
            .data
            .as_ref()
            .expect("current data")
            .warnings,
        vec!["current"]
    );
    assert_eq!(terminal.hype_etfs.error.as_deref(), Some("current error"));
    assert_eq!(terminal.hype_etfs.last_fetch, Some(last_fetch));
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
        Ok(HypeEtfData {
            funds: Vec::new(),
            warnings: vec!["duplicate".to_string()],
        })
        .into(),
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
        Err("network down".to_string()).into(),
    ));

    assert!(!terminal.hype_etfs.loading);
    assert_eq!(terminal.hype_etfs.error.as_deref(), Some("network down"));
    assert!(terminal.hype_etfs.last_fetch.is_none());

    let _task = terminal.request_hype_etfs_refresh(false);

    assert!(terminal.hype_etfs.loading);
    assert_eq!(terminal.hype_etfs.refresh_request_id, 2);
}

#[test]
fn hype_etf_error_redacts_state_error() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.hype_etfs.loading = true;
    terminal.hype_etfs.refresh_request_id = 1;

    let _task = terminal.update_hype_etfs_market(Message::HypeEtfsLoaded(
        1,
        Err("ETF fetch failed: api_key=etf-secret signature=sig-secret".to_string()).into(),
    ));

    let error = terminal.hype_etfs.error.as_deref().expect("state error");
    assert!(error.contains("api_key=<redacted>"));
    assert!(error.contains("signature=<redacted>"));
    assert!(!error.contains("etf-secret"));
    assert!(!error.contains("sig-secret"));
}

#[test]
fn hype_etf_success_redacts_partial_warnings_before_state() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.hype_etfs.loading = true;
    terminal.hype_etfs.refresh_request_id = 1;

    let _task = terminal.update_hype_etfs_market(Message::HypeEtfsLoaded(
        1,
        Ok(HypeEtfData {
            funds: Vec::new(),
            warnings: vec![
                "safe partial warning".to_string(),
                "partial failure: auth_token=partial-warning-secret".to_string(),
            ],
        })
        .into(),
    ));

    let warnings = &terminal
        .hype_etfs
        .data
        .as_ref()
        .expect("successful partial result should remain available")
        .warnings;
    assert_eq!(warnings[0], "safe partial warning");
    assert!(warnings[1].contains("auth_token=<redacted>"));
    assert!(!warnings[1].contains("partial-warning-secret"));
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
