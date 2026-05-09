use super::{
    CONFIG_SAVE_DEBOUNCE, ConfigSaveCompletionAction, config_save_completion_action,
    config_save_is_due, config_save_should_start,
};
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
    assert_eq!(
        config_save_completion_action(true, true),
        ConfigSaveCompletionAction::SavePending
    );
    assert_eq!(
        config_save_completion_action(true, false),
        ConfigSaveCompletionAction::Exit
    );
    assert_eq!(
        config_save_completion_action(false, true),
        ConfigSaveCompletionAction::None
    );
}
