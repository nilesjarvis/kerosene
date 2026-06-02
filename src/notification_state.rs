use crate::app_state::TradingTerminal;

use crate::sound;

#[cfg(target_os = "macos")]
use std::ffi::OsStr;
#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::sync::Once;

/// How long toasts are visible before auto-dismissing (seconds).
pub(crate) const TOAST_LIFETIME_SECS: u64 = 5;
/// Maximum toasts visible at once.
const MAX_TOASTS: usize = 8;

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
}

impl TradingTerminal {
    pub(crate) fn push_silent_toast(&mut self, message: String, is_error: bool) {
        self.toasts.push(Toast {
            id: self.next_toast_id,
            message,
            is_error,
            created_at: std::time::Instant::now(),
        });
        self.next_toast_id += 1;
        if self.toasts.len() > MAX_TOASTS {
            self.toasts.remove(0);
        }
    }

    /// Push a toast notification. Also plays sound and sends desktop
    /// notification if enabled.
    pub(crate) fn push_toast(&mut self, message: String, is_error: bool) {
        let _theme = self.theme();
        self.toasts.push(Toast {
            id: self.next_toast_id,
            message: message.clone(),
            is_error,
            created_at: std::time::Instant::now(),
        });
        self.next_toast_id += 1;
        if self.toasts.len() > MAX_TOASTS {
            self.toasts.remove(0);
        }
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
        self.toasts.push(Toast {
            id: self.next_toast_id,
            message: message.clone(),
            is_error: false,
            created_at: std::time::Instant::now(),
        });
        self.next_toast_id += 1;
        if self.toasts.len() > MAX_TOASTS {
            self.toasts.remove(0);
        }

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
        self.toasts.push(Toast {
            id: self.next_toast_id,
            message: message.clone(),
            is_error: false,
            created_at: std::time::Instant::now(),
        });
        self.next_toast_id += 1;
        if self.toasts.len() > MAX_TOASTS {
            self.toasts.remove(0);
        }

        if self.sound_enabled {
            sound::play_fill();
        }

        if self.desktop_notifications {
            show_desktop_notification("Kerosene: Tracked Trade", message);
        }
    }

    pub(crate) fn push_telegram_feed_alert(&mut self, message: String) {
        self.toasts.push(Toast {
            id: self.next_toast_id,
            message: message.clone(),
            is_error: false,
            created_at: std::time::Instant::now(),
        });
        self.next_toast_id += 1;
        if self.toasts.len() > MAX_TOASTS {
            self.toasts.remove(0);
        }

        if self.sound_enabled {
            sound::play_fill();
        }

        if self.desktop_notifications {
            show_desktop_notification("Kerosene: Telegram Feed", message);
        }
    }

    pub(crate) fn push_x_feed_alert(&mut self, message: String) {
        self.toasts.push(Toast {
            id: self.next_toast_id,
            message: message.clone(),
            is_error: false,
            created_at: std::time::Instant::now(),
        });
        self.next_toast_id += 1;
        if self.toasts.len() > MAX_TOASTS {
            self.toasts.remove(0);
        }

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
        self.order_status = Some((message, is_error));
        self.play_notification_sound(is_error);
    }
}
