use crate::app_state::TradingTerminal;

use crate::sound;

#[cfg(target_os = "macos")]
use std::ffi::OsStr;
#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::sync::Once;

/// How long non-error toasts are visible before auto-dismissing (seconds).
pub(crate) const TOAST_LIFETIME_SECS: u64 = 5;
/// Maximum toasts visible at once.
const MAX_TOASTS: usize = 8;
/// Duration of the toast enter/exit slide-and-fade animation.
pub(crate) const TOAST_ANIMATION: std::time::Duration = std::time::Duration::from_millis(260);

#[cfg(target_os = "macos")]
const MACOS_DEV_NOTIFICATION_BUNDLE_ID: &str = "com.apple.Terminal";

fn show_desktop_notification(summary: impl Into<String>, message: String) {
    let summary = summary.into();
    std::thread::spawn(move || {
        prepare_desktop_notifications();
        let _ = notify_rust::Notification::new()
            .summary(&summary)
            .body(&message)
            .timeout(5000)
            .show();
    });
}

#[cfg(not(target_os = "macos"))]
fn prepare_desktop_notifications() {}

#[cfg(target_os = "macos")]
fn prepare_desktop_notifications() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let bundle_id = current_macos_app_bundle_id()
            .unwrap_or_else(|| MACOS_DEV_NOTIFICATION_BUNDLE_ID.into());
        let _ = notify_rust::set_application(&bundle_id);
    });
}

#[cfg(target_os = "macos")]
fn current_macos_app_bundle_id() -> Option<String> {
    let app_bundle = std::env::current_exe()
        .ok()?
        .ancestors()
        .find(|path| path.extension() == Some(OsStr::new("app")))?
        .to_path_buf();
    read_bundle_identifier(app_bundle.join("Contents").join("Info.plist"))
}

#[cfg(target_os = "macos")]
fn read_bundle_identifier(plist_path: PathBuf) -> Option<String> {
    let output = std::process::Command::new("/usr/bin/plutil")
        .arg("-extract")
        .arg("CFBundleIdentifier")
        .arg("raw")
        .arg("-o")
        .arg("-")
        .arg(plist_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let bundle_id = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!bundle_id.is_empty()).then_some(bundle_id)
}

pub(crate) struct Toast {
    pub(crate) id: u64,
    pub(crate) message: String,
    pub(crate) is_error: bool,
    pub(crate) created_at: std::time::Instant,
    /// Set once the toast begins its exit animation.
    pub(crate) dismissing_at: Option<std::time::Instant>,
}

impl Toast {
    /// Entrance animation progress in `0.0..=1.0` (1.0 once fully shown).
    pub(crate) fn enter_progress(&self, now: std::time::Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.created_at).as_secs_f32();
        (elapsed / TOAST_ANIMATION.as_secs_f32()).clamp(0.0, 1.0)
    }

    /// Exit animation progress in `0.0..=1.0` (1.0 once fully gone).
    pub(crate) fn exit_progress(&self, now: std::time::Instant) -> f32 {
        match self.dismissing_at {
            Some(started) => {
                let elapsed = now.saturating_duration_since(started).as_secs_f32();
                (elapsed / TOAST_ANIMATION.as_secs_f32()).clamp(0.0, 1.0)
            }
            None => 0.0,
        }
    }

    /// Whether this toast is mid enter or exit transition.
    pub(crate) fn is_animating(&self, now: std::time::Instant) -> bool {
        self.enter_progress(now) < 1.0 || self.dismissing_at.is_some()
    }
}

pub(crate) fn toast_auto_dismiss_due(toast: &Toast, now: std::time::Instant) -> bool {
    if toast.is_error {
        return false;
    }

    now.duration_since(toast.created_at).as_secs() >= TOAST_LIFETIME_SECS
}

fn push_toast_entry(
    toasts: &mut Vec<Toast>,
    next_toast_id: &mut u64,
    message: String,
    is_error: bool,
) {
    toasts.push(Toast {
        id: *next_toast_id,
        message,
        is_error,
        created_at: std::time::Instant::now(),
        dismissing_at: None,
    });
    *next_toast_id += 1;
    prune_toast_queue(toasts);
}

fn prune_toast_queue(toasts: &mut Vec<Toast>) {
    while toasts.len() > MAX_TOASTS {
        let remove_index = toasts.iter().position(|toast| !toast.is_error).unwrap_or(0);
        toasts.remove(remove_index);
    }
}

impl TradingTerminal {
    pub(crate) fn push_silent_toast(&mut self, message: String, is_error: bool) {
        push_toast_entry(&mut self.toasts, &mut self.next_toast_id, message, is_error);
    }

    /// Push a toast notification. Also plays sound and sends desktop
    /// notification if enabled.
    pub(crate) fn push_toast(&mut self, message: String, is_error: bool) {
        let _theme = self.theme();
        push_toast_entry(
            &mut self.toasts,
            &mut self.next_toast_id,
            message.clone(),
            is_error,
        );
        // Sound
        self.play_notification_sound(is_error);
        // Desktop notification
        if self.desktop_notifications {
            let summary = if is_error {
                "Kerosene: Error"
            } else {
                "Kerosene: Trade"
            };
            show_desktop_notification(summary, message);
        }
    }

    /// Push a positive interest alert with dedicated sound and summary.
    pub(crate) fn push_interest_alert(&mut self, message: String) {
        let _theme = self.theme();
        push_toast_entry(
            &mut self.toasts,
            &mut self.next_toast_id,
            message.clone(),
            false,
        );

        if self.sound_enabled {
            sound::play_interest();
        }

        if self.desktop_notifications {
            show_desktop_notification("Kerosene: Interest", message);
        }
    }

    /// Push a tracked-trade alert. This alert is controlled by the Tracked
    /// Trades pane button and intentionally emits both sound and desktop
    /// notification when enabled.
    pub(crate) fn push_tracked_trade_alert(&mut self, message: String) {
        push_toast_entry(
            &mut self.toasts,
            &mut self.next_toast_id,
            message.clone(),
            false,
        );

        if self.sound_enabled {
            sound::play_fill();
        }

        if self.desktop_notifications {
            show_desktop_notification("Kerosene: Tracked Trade", message);
        }
    }

    pub(crate) fn push_telegram_feed_alert(&mut self, message: String) {
        push_toast_entry(
            &mut self.toasts,
            &mut self.next_toast_id,
            message.clone(),
            false,
        );

        if self.sound_enabled {
            sound::play_fill();
        }

        if self.desktop_notifications {
            show_desktop_notification("Kerosene: Telegram Feed", message);
        }
    }

    pub(crate) fn push_x_feed_alert(&mut self, message: String) {
        push_toast_entry(
            &mut self.toasts,
            &mut self.next_toast_id,
            message.clone(),
            false,
        );

        if self.sound_enabled {
            sound::play_fill();
        }

        if self.desktop_notifications {
            show_desktop_notification("Kerosene: X Feed", message);
        }
    }

    pub(crate) fn play_notification_sound(&self, is_error: bool) {
        let _theme = self.theme();
        if self.sound_enabled {
            if is_error {
                sound::play_error();
            } else {
                sound::play_fill();
            }
        }
    }

    pub(crate) fn set_order_status(&mut self, message: String, is_error: bool) {
        let _theme = self.theme();
        self.order_status = Some((message.clone(), is_error));
        if is_error {
            // Execution failures must stay visible when the order ticket pane
            // is closed (HUD/quick-order trading from a chart); push_toast
            // also covers the error sound and desktop notification.
            self.push_toast(message, true);
        } else {
            self.play_notification_sound(is_error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toast(id: u64, is_error: bool) -> Toast {
        Toast {
            id,
            message: format!("toast {id}"),
            is_error,
            created_at: std::time::Instant::now(),
            dismissing_at: None,
        }
    }

    #[test]
    fn error_toasts_do_not_auto_dismiss() {
        let mut error = toast(1, true);
        error.created_at = error
            .created_at
            .checked_sub(std::time::Duration::from_secs(TOAST_LIFETIME_SECS + 60))
            .expect("test timestamp should be representable");

        assert!(!toast_auto_dismiss_due(&error, std::time::Instant::now()));
    }

    #[test]
    fn stale_info_toasts_auto_dismiss() {
        let mut info = toast(1, false);
        info.created_at = info
            .created_at
            .checked_sub(std::time::Duration::from_secs(TOAST_LIFETIME_SECS))
            .expect("test timestamp should be representable");

        assert!(toast_auto_dismiss_due(&info, std::time::Instant::now()));
    }

    #[test]
    fn pruning_removes_oldest_non_error_before_error() {
        let mut toasts = vec![toast(0, true)];
        for id in 1..=MAX_TOASTS as u64 {
            toasts.push(toast(id, false));
        }

        prune_toast_queue(&mut toasts);

        assert_eq!(toasts.len(), MAX_TOASTS);
        assert!(toasts.iter().any(|toast| toast.id == 0 && toast.is_error));
        assert!(!toasts.iter().any(|toast| toast.id == 1));
    }

    #[test]
    fn info_toast_does_not_evict_full_error_queue() {
        let mut toasts = (0..MAX_TOASTS as u64)
            .map(|id| toast(id, true))
            .collect::<Vec<_>>();
        toasts.push(toast(MAX_TOASTS as u64, false));

        prune_toast_queue(&mut toasts);

        assert_eq!(toasts.len(), MAX_TOASTS);
        assert!(toasts.iter().all(|toast| toast.is_error));
        assert!(!toasts.iter().any(|toast| toast.id == MAX_TOASTS as u64));
    }
}
