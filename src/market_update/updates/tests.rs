use super::*;
use crate::market_state::updates::UpdatesSnapshot;
use iced::widget::pane_grid;

#[test]
fn updates_refresh_requires_pane_and_coalesces_manual_and_timer_requests() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.panes = pane_grid::State::new(PaneKind::Chart(0)).0;
    let _ = terminal.request_updates_refresh(true);
    assert!(!terminal.updates.loading);
    terminal.panes = pane_grid::State::new(PaneKind::Updates).0;
    let _ = terminal.request_updates_refresh(false);
    assert!(terminal.updates.loading);
    let id = terminal.updates.request_id;
    let _ = terminal.request_updates_refresh(true);
    assert_eq!(terminal.updates.request_id, id);
    terminal.updates.loading = false;
    let _ = terminal.request_updates_refresh(true);
    assert!(!terminal.updates.loading);
}

#[test]
fn updates_stale_results_do_not_finish_current_requests() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.updates.loading = true;
    terminal.updates.request_id = 2;
    let _ = terminal.update_parameters_market(Message::UpdatesLoaded(
        1,
        Box::new(UpdatesSnapshot {
            parameters: Err("old failure".into()),
            caps: Err("old failure".into()),
        }),
    ));
    assert!(terminal.updates.loading);
    assert!(terminal.updates.error.is_none());
}

#[test]
fn updates_storage_failures_retry_without_blocking_market_refresh() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.panes = pane_grid::State::new(PaneKind::Updates).0;
    terminal.updates.dirty = true;
    let _ = terminal.save_updates_history();
    assert!(terminal.updates.saving);
    let _ = terminal.request_updates_refresh(false);
    assert!(
        terminal.updates.loading,
        "persistence must not block polling"
    );
    let _ = terminal.update_parameters_market(Message::UpdatesSaved(Err("disk full".into())));
    assert!(terminal.updates.dirty && terminal.updates.storage_error);
    let _ = terminal.save_updates_history();
    assert!(!terminal.updates.saving, "failed saves are rate limited");
    terminal.updates.last_save_attempt = None;
    let _ = terminal.save_updates_history();
    assert!(terminal.updates.saving);
    terminal.updates.dirty = true;
    let _ = terminal.update_parameters_market(Message::UpdatesSaved(Ok(())));
    assert!(terminal.updates.saving, "changes during save are flushed");
    assert!(!terminal.updates.dirty);
}

#[test]
fn updates_add_pane_is_singleton_and_view_renders() {
    let (mut terminal, _) = TradingTerminal::boot();
    let _ = terminal.update_panes(Message::AddUpdatesPane);
    let _ = terminal.update_panes(Message::AddUpdatesPane);
    assert_eq!(
        terminal
            .workspace_pane_kinds()
            .filter(|(_, _, kind)| matches!(kind, PaneKind::Updates))
            .count(),
        1
    );
    let _view = terminal.view_updates();
}
