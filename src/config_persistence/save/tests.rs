use super::{
    CONFIG_SAVE_DEBOUNCE, ConfigSaveCompletionAction, config_save_completion_action,
    config_save_is_due, config_save_should_start,
};
use crate::app_state::TradingTerminal;
use std::time::{Duration, Instant};

#[test]
fn config_save_due_check_waits_until_debounce_deadline() {
    let now = Instant::now();
    let due_at = now + CONFIG_SAVE_DEBOUNCE;

    assert!(!config_save_is_due(None, now));
    assert!(!config_save_is_due(
        Some(due_at),
        now + Duration::from_millis(100)
    ));
    assert!(config_save_is_due(Some(due_at), due_at));
    assert!(config_save_is_due(
        Some(due_at),
        due_at + Duration::from_secs(1)
    ));
}

#[test]
fn config_save_start_waits_for_in_flight_write() {
    let now = Instant::now();
    let due_at = now - Duration::from_millis(1);

    assert!(config_save_should_start(Some(due_at), false, now));
    assert!(!config_save_should_start(Some(due_at), true, now));
    assert!(!config_save_should_start(None, false, now));
}

#[test]
fn config_save_completion_prioritizes_pending_exit_save() {
    // A pending debounced save runs before the exit decision regardless
    // of the just-completed save's success — the user's most-recent
    // changes haven't hit disk yet.
    assert_eq!(
        config_save_completion_action(true, true, true),
        ConfigSaveCompletionAction::SavePending
    );
    assert_eq!(
        config_save_completion_action(true, true, false),
        ConfigSaveCompletionAction::SavePending
    );
}

#[test]
fn config_save_completion_exits_only_after_a_successful_save() {
    assert_eq!(
        config_save_completion_action(true, false, true),
        ConfigSaveCompletionAction::Exit
    );
}

#[test]
fn config_save_completion_blocks_exit_when_final_save_failed() {
    // Exit requested + nothing pending + the just-completed save returned
    // Err → stay open so account layout, muted tickers, hotkeys, presets,
    // etc. aren't silently dropped.
    assert_eq!(
        config_save_completion_action(true, false, false),
        ConfigSaveCompletionAction::BlockExitOnError
    );
}

#[test]
fn failed_exit_save_leaves_immediate_retry_pending() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.config_save_exit_requested = true;
    terminal.config_save_due_at = None;

    let _task = terminal.handle_config_save_result(Err("disk full".to_string()));

    assert!(!terminal.config_save_exit_requested);
    assert!(terminal.config_save_due_at.is_some());
    assert!(config_save_should_start(
        terminal.config_save_due_at,
        terminal.config_save_in_flight,
        Instant::now()
    ));
}

#[test]
fn config_save_completion_does_nothing_when_exit_was_not_requested() {
    assert_eq!(
        config_save_completion_action(false, true, true),
        ConfigSaveCompletionAction::None
    );
    assert_eq!(
        config_save_completion_action(false, false, true),
        ConfigSaveCompletionAction::None
    );
    assert_eq!(
        config_save_completion_action(false, false, false),
        ConfigSaveCompletionAction::None
    );
}
