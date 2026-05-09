use crate::app_state::TradingTerminal;

use crate::sound;

/// How long toasts are visible before auto-dismissing (seconds).
pub(crate) const TOAST_LIFETIME_SECS: u64 = 5;
/// Maximum toasts visible at once.
const MAX_TOASTS: usize = 8;

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
            }
            .to_string();
            std::thread::spawn(move || {
                let _ = notify_rust::Notification::new()
                    .summary(&summary)
                    .body(&message)
                    .timeout(5000)
                    .show();
            });
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
            std::thread::spawn(move || {
                let _ = notify_rust::Notification::new()
                    .summary("Kerosene: Interest")
                    .body(&message)
                    .timeout(5000)
                    .show();
            });
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
            std::thread::spawn(move || {
                let _ = notify_rust::Notification::new()
                    .summary("Kerosene: Tracked Trade")
                    .body(&message)
                    .timeout(5000)
                    .show();
            });
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
