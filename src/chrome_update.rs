use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::notification_state::TOAST_LIFETIME_SECS;
use crate::sound;
use iced::{Task, clipboard};
use std::time::Instant;

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
                self.toasts.retain(|t| t.id != id);
            }
            Message::CopyToClipboard(text) => {
                self.push_toast("Copied address to clipboard".to_string(), false);
                return clipboard::write(text).map(|()| Message::NoOp);
            }
            Message::NoOp => {}
            Message::TickToastCleanup => {
                let now = Instant::now();
                self.toasts
                    .retain(|t| now.duration_since(t.created_at).as_secs() < TOAST_LIFETIME_SECS);
                if self.nuke_confirmation.is_some_and(|armed_at| {
                    !crate::order_update::nuke_confirmation_is_armed(Some(armed_at), now)
                }) {
                    self.nuke_confirmation = None;
                }
                return self.stop_chase_if_limits_reached(now);
            }
            Message::SpinnerTick => {
                self.spinner_phase = (self.spinner_phase + 0.35).rem_euclid(std::f32::consts::TAU);
                for instance in self.charts.values_mut() {
                    instance.advance_quick_order_limit_line();
                    instance.advance_order_line_animation();
                }
            }
            Message::StatusBarTick => {
                let now = Instant::now();
                let config_save_task = self.flush_config_save_if_due(now);
                for status in sound::take_status_messages() {
                    self.push_silent_toast(status.message, status.is_error);
                }
                if self.is_calendar_open()
                    && !self.calendar_loading
                    && self
                        .calendar_next_retry
                        .is_some_and(|retry_at| now >= retry_at)
                {
                    self.calendar_next_retry = None;
                    return Task::batch([config_save_task, self.request_calendar_refresh(false)]);
                }
                return config_save_task;
            }
            Message::ConfigSaved(result) => {
                return self.handle_config_save_result(result);
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
            _ => {}
        }

        Task::none()
    }
}
