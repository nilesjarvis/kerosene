use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use super::capture::{
    copy_chart_screenshot_to_clipboard, render_chart_screenshot, save_chart_screenshot_png,
};
use super::widget::CaptureChartCanvas;

use iced::{Task, window};

mod lifecycle;
mod request;
#[cfg(test)]
pub(super) use request::chart_for_screenshot_export;

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn update_chart_screenshot(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleChartScreenshotMenu(_chart_id, surface_id) => {
                if self.chart_screenshot_menu_open == Some(surface_id) {
                    self.chart_screenshot_menu_open = None;
                } else {
                    self.close_chart_header_menus();
                    self.chart_screenshot_menu_open = Some(surface_id);
                }
            }
            Message::ToggleChartScreenshotObscurePositionEntry(obscure)
                if self.chart_screenshot_settings.obscure_position_entry != obscure =>
            {
                self.chart_screenshot_settings.obscure_position_entry = obscure;
                self.persist_config();
            }
            Message::ToggleChartScreenshotHidePositionsAndOrders(hide)
                if self.chart_screenshot_settings.hide_positions_and_orders != hide =>
            {
                self.chart_screenshot_settings.hide_positions_and_orders = hide;
                self.persist_config();
            }
            Message::OpenChartScreenshot(chart_id, surface_id) => {
                self.chart_screenshot_menu_open = None;
                if self.chart_screenshot_capture_in_progress {
                    self.push_toast("Chart screenshot already in progress".to_string(), false);
                    return self.open_or_focus_chart_screenshot_window(Task::none());
                }

                let Some(instance) = self.charts.get(&chart_id) else {
                    self.push_toast(
                        "Chart screenshot unavailable: chart not found".to_string(),
                        true,
                    );
                    return Task::none();
                };

                if instance.chart.candles.is_empty() {
                    self.push_toast(
                        "Chart screenshot unavailable: no visible candles".to_string(),
                        true,
                    );
                    return Task::none();
                }

                // Freeze model, privacy settings, theme and timestamp at the click.
                let mut request = Some(self.chart_screenshot_render_request(instance));
                self.chart_screenshot_next_request_id =
                    self.chart_screenshot_next_request_id.saturating_add(1);
                let request_id = self.chart_screenshot_next_request_id;
                self.chart_screenshot_pending_request_id = Some(request_id);
                self.chart_screenshot_capture_in_progress = true;
                self.chart_screenshot_error = None;
                self.chart_screenshot = None;

                let target = Self::chart_screenshot_canvas_id(surface_id);
                let mut open_preview =
                    Some(self.open_or_focus_chart_screenshot_window(Task::none()));
                return iced::advanced::widget::operate(CaptureChartCanvas::new(target)).then(
                    move |snapshot| {
                        // A widget operation resolves once. Secure the canvas state
                        // before opening/focusing another window or awaiting rendering.
                        let Some(request) = request.take() else {
                            return Task::none();
                        };
                        let capture = Task::perform(
                            async move {
                                let snapshot = snapshot
                                    .ok_or_else(|| "chart area was not visible".to_string())?;
                                render_chart_screenshot(request, snapshot).await
                            },
                            move |result| {
                                Message::ChartScreenshotCaptured(request_id, chart_id, result)
                            },
                        );
                        Task::batch([open_preview.take().unwrap_or_else(Task::none), capture])
                    },
                );
            }
            Message::ChartScreenshotCaptured(request_id, _chart_id, result) => {
                if self.chart_screenshot_pending_request_id != Some(request_id) {
                    return Task::none();
                }

                self.chart_screenshot_pending_request_id = None;
                self.chart_screenshot_capture_in_progress = false;
                match result {
                    Ok(state) => {
                        self.chart_screenshot = Some(state);
                        self.chart_screenshot_error = None;
                        if let Some(id) = self.chart_screenshot_window_id {
                            return window::gain_focus(id);
                        }

                        return self.open_or_focus_chart_screenshot_window(Task::none());
                    }
                    Err(err) => {
                        let err = redact_sensitive_response_text(&err);
                        self.chart_screenshot_error = Some(err.clone());
                        self.push_toast(format!("Chart screenshot failed: {err}"), true);
                    }
                }
            }
            Message::CopyChartScreenshot => {
                let Some(state) = self.chart_screenshot.clone() else {
                    self.push_toast("No chart screenshot to copy".to_string(), true);
                    return Task::none();
                };

                return Task::perform(
                    async move {
                        let result = copy_chart_screenshot_to_clipboard(state);
                        result.map_err(|e| e.to_string())
                    },
                    Message::ChartScreenshotCopied,
                );
            }
            Message::ChartScreenshotCopied(result) => match result {
                Ok(()) => self.push_toast("Chart image copied to clipboard".to_string(), false),
                Err(err) => self.push_toast(
                    format!(
                        "Chart image copy failed: {}",
                        redact_sensitive_response_text(&err)
                    ),
                    true,
                ),
            },
            Message::SaveChartScreenshot => {
                let Some(state) = self.chart_screenshot.clone() else {
                    self.push_toast("No chart screenshot to save".to_string(), true);
                    return Task::none();
                };

                return Task::perform(
                    save_chart_screenshot_png(state),
                    Message::ChartScreenshotSaved,
                );
            }
            Message::ChartScreenshotSaved(result) => match result {
                Ok(Some(path)) => {
                    self.push_toast(format!("Chart image saved to {}", path.display()), false)
                }
                Ok(None) => {}
                Err(err) => self.push_toast(
                    format!(
                        "Chart image save failed: {}",
                        redact_sensitive_response_text(&err)
                    ),
                    true,
                ),
            },
            Message::CloseChartScreenshotWindow => {
                if let Some(id) = self.chart_screenshot_window_id {
                    return window::close(id);
                }
            }
            _ => {}
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Candle;
    use crate::chart_state::ChartInstance;
    use crate::config::KeroseneConfig;
    use crate::timeframe::Timeframe;

    #[test]
    fn chart_screenshot_ignores_cancelled_and_superseded_results() {
        for pending in [None, Some(9)] {
            let (mut terminal, _task) =
                TradingTerminal::boot_from_config(KeroseneConfig::default());
            terminal.chart_screenshot_pending_request_id = pending;
            terminal.chart_screenshot_capture_in_progress = pending.is_some();
            let toast_count = terminal.toasts.len();

            let _task = terminal.update_chart_screenshot(Message::ChartScreenshotCaptured(
                8,
                1,
                Err("stale render failure".to_string()),
            ));

            assert_eq!(terminal.chart_screenshot_pending_request_id, pending);
            assert_eq!(
                terminal.chart_screenshot_capture_in_progress,
                pending.is_some()
            );
            assert!(terminal.chart_screenshot.is_none());
            assert!(terminal.chart_screenshot_error.is_none());
            assert_eq!(terminal.toasts.len(), toast_count);
        }
    }

    #[test]
    fn chart_screenshot_request_freezes_data_and_privacy_at_click() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let mut instance = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
        instance.chart.candles.push(Candle::test_ohlcv(
            0,
            59_999,
            [100.0, 110.0, 90.0, 105.0],
            1000.0,
        ));
        instance.chart.request_view_reset();
        terminal.chart_screenshot_settings.hide_positions_and_orders = true;
        let request = terminal.chart_screenshot_render_request(&instance);

        instance.chart.candles[0].close = 106.0;
        instance.chart.request_view_reset();
        instance.symbol_display = "ETH".to_string();
        terminal.chart_screenshot_settings.hide_positions_and_orders = false;

        assert_eq!(request.symbol, "BTC");
        assert_eq!(request.chart.candles[0].close, 105.0);
        assert!(request.chart.hide_positions_and_orders);
        assert!(!instance.chart.hide_positions_and_orders);
    }

    #[test]
    fn chart_screenshot_capture_error_redacts_window_error_and_toast() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.chart_screenshot_pending_request_id = Some(7);
        terminal.chart_screenshot_capture_in_progress = true;

        let _task = terminal.update_chart_screenshot(Message::ChartScreenshotCaptured(
            7,
            1,
            Err("render failed: api_key=key-secret signature=sig-secret".to_string()),
        ));

        let error = terminal
            .chart_screenshot_error
            .as_ref()
            .expect("screenshot error");
        assert!(error.contains("api_key=<redacted>"));
        assert!(error.contains("signature=<redacted>"));
        assert!(!error.contains("key-secret"));
        assert!(!error.contains("sig-secret"));

        let toast = terminal.toasts.last().expect("toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("api_key=<redacted>"));
        assert!(toast.message.contains("signature=<redacted>"));
        assert!(!toast.message.contains("key-secret"));
        assert!(!toast.message.contains("sig-secret"));
    }

    #[test]
    fn chart_screenshot_copy_error_redacts_toast() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());

        let _task = terminal.update_chart_screenshot(Message::ChartScreenshotCopied(Err(
            "copy failed: auth_token=token-secret".to_string(),
        )));

        let toast = terminal.toasts.last().expect("toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("auth_token=<redacted>"));
        assert!(!toast.message.contains("token-secret"));
    }

    #[test]
    fn chart_screenshot_save_error_redacts_toast() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());

        let _task = terminal.update_chart_screenshot(Message::ChartScreenshotSaved(Err(
            "save failed: client_secret=secret-value".to_string(),
        )));

        let toast = terminal.toasts.last().expect("toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("client_secret=<redacted>"));
        assert!(!toast.message.contains("secret-value"));
    }
}
