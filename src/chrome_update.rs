use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::notification_state::toast_auto_dismiss_due;
use iced::{Task, clipboard};
use std::time::Instant;

mod menus;
mod status_tick;

impl TradingTerminal {
    pub(crate) fn update_chrome(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleIncomeAlerts => {
                self.income_alerts_enabled = !self.income_alerts_enabled;
                self.persist_config();
            }
            Message::ToggleLiquidationAlerts => {
                self.liquidation_alerts_enabled = !self.liquidation_alerts_enabled;
                self.persist_config();
            }
            Message::ToggleTrackedTradeAlerts => {
                self.tracked_trade_alerts_enabled = !self.tracked_trade_alerts_enabled;
                self.persist_config();
            }
            Message::ToggleTrackedTradeAggregation => {
                self.tracked_trade_aggregation_enabled = !self.tracked_trade_aggregation_enabled;
                self.persist_config();
            }
            Message::ToggleTrackedTradeSettingsMenu => {
                let opening = !self.tracked_trade_settings_menu_open;
                if opening {
                    self.close_chart_header_menus();
                }
                self.tracked_trade_settings_menu_open = opening;
            }
            Message::ToggleLiquidationFeedAggregation => {
                self.liquidation_feed_aggregation_enabled =
                    !self.liquidation_feed_aggregation_enabled;
                self.persist_config();
            }
            Message::ToggleLiquidationChart => {
                self.liquidation_chart_enabled = !self.liquidation_chart_enabled;
            }
            Message::ToggleLiquidationSummary => {
                self.liquidation_summary_enabled = !self.liquidation_summary_enabled;
            }
            Message::ToggleLiquidationFollow => {
                self.liquidation_feed_following = !self.liquidation_feed_following;
                if self.liquidation_feed_following {
                    return self.snap_liquidation_feed_to_latest();
                }
            }
            Message::ToggleLiquidationSettingsMenu => {
                let opening = !self.liquidation_settings_menu_open;
                if opening {
                    self.close_chart_header_menus();
                }
                self.liquidation_settings_menu_open = opening;
            }
            Message::LiquidationAlertThresholdChanged(val) => {
                self.liquidation_alert_input = val.clone();
                if let Ok(num) = val.parse::<f64>() {
                    self.liquidation_alert_threshold = num;
                    self.persist_config();
                }
            }
            Message::SaveLiquidationAlertThreshold => {
                if let Ok(val) = self.liquidation_alert_input.parse::<f64>() {
                    self.liquidation_alert_threshold = val;
                } else {
                    self.liquidation_alert_input = self.liquidation_alert_threshold.to_string();
                }
                self.persist_config();
            }
            Message::DismissToast(id) => {
                if self.toast_animations_enabled {
                    let now = Instant::now();
                    if let Some(toast) = self.toasts.iter_mut().find(|t| t.id == id) {
                        toast.dismissing_at.get_or_insert(now);
                    }
                } else {
                    self.toasts.retain(|t| t.id != id);
                }
            }
            Message::ToastAnimationTick => {
                let now = Instant::now();
                self.toasts.retain(|t| t.exit_progress(now) < 1.0);
            }
            Message::CopyToClipboard(text) => {
                self.push_toast("Copied to clipboard".to_string(), false);
                return clipboard::write(text.into_string()).map(|()| Message::NoOp);
            }
            Message::WalletAddressActionsHovered(key) => {
                self.hovered_wallet_address_actions = Some(key.into_string());
            }
            Message::WalletAddressActionsExited(key)
                if self.hovered_wallet_address_actions.as_deref() == Some(key.as_str()) =>
            {
                self.hovered_wallet_address_actions = None;
            }
            Message::NoOp => {}
            Message::TickToastCleanup => {
                let now = Instant::now();
                if self.toast_animations_enabled {
                    for toast in &mut self.toasts {
                        if toast.dismissing_at.is_none() && toast_auto_dismiss_due(toast, now) {
                            toast.dismissing_at = Some(now);
                        }
                    }
                    self.toasts.retain(|t| t.exit_progress(now) < 1.0);
                } else {
                    self.toasts.retain(|t| !toast_auto_dismiss_due(t, now));
                }
                if self.nuke_confirmation.as_ref().is_some_and(|confirmation| {
                    !crate::order_update::nuke_confirmation_is_armed(Some(confirmation), now)
                }) {
                    self.nuke_confirmation = None;
                }
                return self.stop_chase_if_limits_reached(now);
            }
            Message::SpinnerTick => {
                self.spinner_phase = (self.spinner_phase + 0.35).rem_euclid(std::f32::consts::TAU);
                self.advance_onboarding_phase();
                for instance in self.charts.values_mut() {
                    instance.advance_quick_order_limit_line();
                    instance.advance_order_line_animation();
                }
            }
            Message::TickerTapeTick => {
                self.ticker_tape_scroll_px =
                    (self.ticker_tape_scroll_px + 1.2).rem_euclid(100_000.0);
            }
            Message::StatusBarTick => {
                return self.handle_status_bar_tick();
            }
            Message::ConfigSaved(result) => {
                return self.handle_config_save_result(result.into_result());
            }
            Message::CalendarImpactFilterChanged(filter) => {
                self.calendar_impact_filter = filter;
            }
            Message::CalendarWindowFilterChanged(filter) => {
                self.calendar_window_filter = filter;
            }
            Message::ToggleSound => {
                self.sound_enabled = !self.sound_enabled;
                self.persist_config();
            }
            Message::ToggleDesktopNotifications => {
                self.desktop_notifications = !self.desktop_notifications;
                self.persist_config();
            }
            Message::ToggleHidePnl => {
                self.hide_pnl = !self.hide_pnl;
                self.persist_config();
            }
            Message::EnterApplication if !self.app_onboarding_dismissed => {
                self.app_onboarding_dismissed = true;
                self.persist_config();
            }
            _ => {}
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_application_dismisses_onboarding_and_persists() {
        let (mut terminal, _) = TradingTerminal::boot();

        assert!(!terminal.app_onboarding_dismissed);

        let _task = terminal.update_chrome(Message::EnterApplication);

        assert!(terminal.app_onboarding_dismissed);
        assert!(terminal.config_save_due_at.is_some());
    }
}
