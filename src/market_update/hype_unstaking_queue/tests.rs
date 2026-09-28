use super::*;

use crate::hype_unstaking_state::{HypeUnstakingEvent, HypeUnstakingQueueData};

use iced::widget::pane_grid;

fn open_hype_unstaking_queue_pane(terminal: &mut TradingTerminal) {
    let (panes, pane) = pane_grid::State::new(PaneKind::Chart(0));
    terminal.panes = panes;
    terminal
        .panes
        .split(
            pane_grid::Axis::Vertical,
            pane,
            PaneKind::HypeUnstakingQueue,
        )
        .expect("split should create HYPE unstaking queue pane");
}

#[test]
fn boot_refresh_is_noop_when_pane_is_closed() {
    let (mut terminal, _) = TradingTerminal::boot();
    let (panes, _) = pane_grid::State::new(PaneKind::Chart(0));
    terminal.panes = panes;
    terminal.hype_unstaking_queue.loading = false;

    let _ = terminal.request_hype_unstaking_queue_boot_refresh();
    assert!(
        !terminal.hype_unstaking_queue.loading,
        "should not start refresh when pane is closed"
    );
}

#[test]
fn boot_refresh_starts_refresh_when_pane_is_open() {
    let (mut terminal, _) = TradingTerminal::boot();
    let (panes, pane) = pane_grid::State::new(PaneKind::Chart(0));
    terminal.panes = panes;
    terminal.hype_unstaking_queue.loading = false;

    terminal
        .panes
        .split(
            pane_grid::Axis::Vertical,
            pane,
            PaneKind::HypeUnstakingQueue,
        )
        .expect("split should create HYPE unstaking queue pane");

    let _ = terminal.request_hype_unstaking_queue_boot_refresh();
    assert!(
        terminal.hype_unstaking_queue.loading,
        "should start refresh when pane is open"
    );
    assert_eq!(terminal.hype_unstaking_queue.refresh_request_id, 1);
}

#[test]
fn stale_loaded_message_does_not_clear_active_refresh() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.hype_unstaking_queue.loading = true;
    terminal.hype_unstaking_queue.refresh_request_id = 2;
    terminal.hype_unstaking_queue.error = Some("current error".to_string());

    let _task = terminal.update_hype_unstaking_queue_market(Message::HypeUnstakingQueueLoaded(
        1,
        Box::new(Ok(HypeUnstakingQueueData::default())),
    ));

    assert!(terminal.hype_unstaking_queue.loading);
    assert!(terminal.hype_unstaking_queue.data.is_none());
    assert_eq!(
        terminal.hype_unstaking_queue.error.as_deref(),
        Some("current error")
    );
    assert!(terminal.hype_unstaking_queue.last_fetch.is_none());
}

#[test]
fn duplicate_loaded_message_after_completion_is_ignored() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.hype_unstaking_queue.loading = false;
    terminal.hype_unstaking_queue.refresh_request_id = 7;
    terminal.hype_unstaking_queue.data =
        Some(HypeUnstakingQueueData::new(vec![HypeUnstakingEvent {
            unlock_time_ms: 1_000,
            user: "0xaccepted".to_string(),
            amount_wei: 100,
        }]));

    let _task = terminal.update_hype_unstaking_queue_market(Message::HypeUnstakingQueueLoaded(
        7,
        Box::new(Ok(HypeUnstakingQueueData::new(vec![HypeUnstakingEvent {
            unlock_time_ms: 2_000,
            user: "0xduplicate".to_string(),
            amount_wei: 200,
        }]))),
    ));

    let data = terminal
        .hype_unstaking_queue
        .data
        .as_ref()
        .expect("accepted data should remain cached");
    assert_eq!(data.events.len(), 1);
    assert_eq!(data.events[0].user, "0xaccepted");
}

#[test]
fn loaded_refresh_prunes_unlocked_rows_before_caching() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.hype_unstaking_queue.loading = true;
    terminal.hype_unstaking_queue.refresh_request_id = 1;
    let now_ms = TradingTerminal::now_ms();

    let _task = terminal.update_hype_unstaking_queue_market(Message::HypeUnstakingQueueLoaded(
        1,
        Box::new(Ok(HypeUnstakingQueueData::new(vec![
            HypeUnstakingEvent {
                unlock_time_ms: now_ms.saturating_sub(1),
                user: "0xpast".to_string(),
                amount_wei: 100,
            },
            HypeUnstakingEvent {
                unlock_time_ms: now_ms.saturating_add(60_000),
                user: "0xfuture".to_string(),
                amount_wei: 200,
            },
        ]))),
    ));

    let data = terminal
        .hype_unstaking_queue
        .data
        .as_ref()
        .expect("successful refresh should cache data");
    assert_eq!(data.events.len(), 1);
    assert_eq!(data.events[0].user, "0xfuture");
}

#[test]
fn failed_refresh_does_not_mark_hype_unstaking_queue_fresh() {
    let (mut terminal, _) = TradingTerminal::boot();
    open_hype_unstaking_queue_pane(&mut terminal);
    terminal.hype_unstaking_queue.loading = true;
    terminal.hype_unstaking_queue.refresh_request_id = 1;

    let _task = terminal.update_hype_unstaking_queue_market(Message::HypeUnstakingQueueLoaded(
        1,
        Box::new(Err("network down".to_string())),
    ));

    assert!(!terminal.hype_unstaking_queue.loading);
    assert_eq!(
        terminal.hype_unstaking_queue.error.as_deref(),
        Some("network down")
    );
    assert!(terminal.hype_unstaking_queue.last_fetch.is_none());

    let _task = terminal.request_hype_unstaking_queue_refresh(false);

    assert!(terminal.hype_unstaking_queue.loading);
    assert_eq!(terminal.hype_unstaking_queue.refresh_request_id, 2);
}

#[test]
fn hype_unstaking_queue_error_redacts_state_error() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.hype_unstaking_queue.loading = true;
    terminal.hype_unstaking_queue.refresh_request_id = 1;

    let _task = terminal.update_hype_unstaking_queue_market(Message::HypeUnstakingQueueLoaded(
        1,
        Box::new(Err(
            "unstaking fetch failed: auth_token=unstaking-secret".to_string()
        )),
    ));

    let error = terminal
        .hype_unstaking_queue
        .error
        .as_deref()
        .expect("state error");
    assert!(error.contains("auth_token=<redacted>"));
    assert!(!error.contains("unstaking-secret"));
}

#[test]
fn hype_unstaking_queue_tick_refresh_is_ignored_when_pane_is_closed() {
    let (mut terminal, _) = TradingTerminal::boot();
    let (panes, _) = pane_grid::State::new(PaneKind::Chart(0));
    terminal.panes = panes;

    let _task = terminal.update_hype_unstaking_queue_market(Message::HypeUnstakingQueueRefreshTick);

    assert!(!terminal.hype_unstaking_queue.loading);
    assert_eq!(terminal.hype_unstaking_queue.refresh_request_id, 0);
}
