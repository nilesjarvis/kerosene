use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConfigSaveCompletionAction {
    None,
    SavePending,
    Exit,
    /// User asked to close, but the final save returned Err. Stay open
    /// so the failure isn't swallowed silently — the recorded error
    /// status (set by `record_config_save_result`) is already visible
    /// in the UI and the user can retry or accept the loss explicitly.
    BlockExitOnError,
}

pub(super) fn config_save_is_due(due_at: Option<Instant>, now: Instant) -> bool {
    due_at.is_some_and(|due_at| now >= due_at)
}

pub(super) fn config_save_should_start(
    due_at: Option<Instant>,
    in_flight: bool,
    now: Instant,
) -> bool {
    !in_flight && config_save_is_due(due_at, now)
}

pub(super) fn config_save_completion_action(
    exit_requested: bool,
    has_pending_save: bool,
    save_succeeded: bool,
) -> ConfigSaveCompletionAction {
    match (exit_requested, has_pending_save, save_succeeded) {
        // A debounced save is still due — run it before deciding to exit.
        (true, true, _) => ConfigSaveCompletionAction::SavePending,
        // Exit requested + nothing else pending + last save succeeded → exit.
        (true, false, true) => ConfigSaveCompletionAction::Exit,
        // Exit requested + nothing else pending + last save FAILED → block.
        // Persistence carries account layout, muted tickers, hotkeys,
        // order presets, etc.; silently exiting after a failed save would
        // lose those changes without a recovery opportunity.
        (true, false, false) => ConfigSaveCompletionAction::BlockExitOnError,
        (false, _, _) => ConfigSaveCompletionAction::None,
    }
}
