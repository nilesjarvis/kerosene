use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use self::bounds::FindWidgetBounds;
use super::capture::{
    copy_chart_screenshot_to_clipboard, render_chart_screenshot, save_chart_screenshot_png,
};
use super::{ChartScreenshotCaptureRequest, ChartScreenshotPendingCapture};

use iced::{Task, window};

mod bounds;
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
                if self.chart_screenshot_capture_in_progress
                    || self.chart_screenshot_pending_capture.is_some()
                {
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

                self.chart_screenshot_next_request_id =
                    self.chart_screenshot_next_request_id.wrapping_add(1);
                let request_id = self.chart_screenshot_next_request_id;
                let request = ChartScreenshotCaptureRequest::new(
                    request_id,
                    chart_id,
                    self.chart_instance_generation,
                    surface_id,
                );
                self.chart_screenshot_pending_capture =
                    Some(ChartScreenshotPendingCapture::awaiting_bounds(request));
                self.chart_screenshot_capture_in_progress = true;
                self.chart_screenshot_error = None;
                self.chart_screenshot = None;

                let target = Self::chart_screenshot_canvas_id(surface_id);
                let bounds_task = iced::advanced::widget::operate(FindWidgetBounds::new(target))
                    .map(move |bounds| Message::ChartScreenshotBoundsResolved(request, bounds));
                return self.open_or_focus_chart_screenshot_window(bounds_task);
            }
            Message::ChartScreenshotBoundsResolved(request, Some(bounds)) => {
                if !self
                    .chart_screenshot_pending_capture
                    .as_ref()
                    .is_some_and(|pending| pending.is_awaiting_bounds(request))
                {
                    return Task::none();
                }
                if request.chart_instance_generation() != self.chart_instance_generation {
                    self.finish_chart_screenshot_error(
                        request,
                        "Chart screenshot unavailable: chart not found".to_string(),
                    );
                    return Task::none();
                }

                let Some(instance) = self.charts.get(&request.chart_id()) else {
                    self.finish_chart_screenshot_error(
                        request,
                        "Chart screenshot unavailable: chart not found".to_string(),
                    );
                    return Task::none();
                };

                let render_request =
                    self.chart_screenshot_render_request(instance, request.surface_id(), bounds);
                let render_started = self
                    .chart_screenshot_pending_capture
                    .as_mut()
                    .is_some_and(|pending| pending.begin_rendering(request));
                if !render_started {
                    return Task::none();
                }

                return Task::perform(render_chart_screenshot(render_request), move |result| {
                    Message::ChartScreenshotCaptured(request, result.into())
                });
            }
            Message::ChartScreenshotBoundsResolved(request, None) => {
                if self
                    .chart_screenshot_pending_capture
                    .as_ref()
                    .is_some_and(|pending| pending.is_awaiting_bounds(request))
                {
                    self.finish_chart_screenshot_error(
                        request,
                        "Chart screenshot unavailable: chart area was not visible".to_string(),
                    );
                }
            }
            Message::ChartScreenshotCaptured(request, result) => {
                if !self
                    .chart_screenshot_pending_capture
                    .as_ref()
                    .is_some_and(|pending| pending.is_rendering(request))
                {
                    return Task::none();
                }

                self.chart_screenshot_pending_capture = None;
                self.chart_screenshot_capture_in_progress = false;
                match result.into_result() {
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
                    |result| Message::ChartScreenshotCopied(result.into()),
                );
            }
            Message::ChartScreenshotCopied(result) => match result.into_result() {
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

                return Task::perform(save_chart_screenshot_png(state), |result| {
                    Message::ChartScreenshotSaved(result.into())
                });
            }
            Message::ChartScreenshotSaved(result) => match result.into_result() {
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
    use crate::chart_screenshot::ChartScreenshotState;
    use crate::config::KeroseneConfig;
    use crate::timeframe::Timeframe;
    use chrono::Local;
    use iced::widget::image::Handle as ImageHandle;
    use std::sync::Arc;

    fn screenshot_state(symbol: &str) -> ChartScreenshotState {
        ChartScreenshotState {
            symbol: symbol.to_string(),
            timeframe: "1H".to_string(),
            width: 1,
            height: 1,
            rgba: Arc::from(vec![1, 2, 3, 255]),
            png: Arc::from(vec![9, 8, 7]),
            preview_handle: ImageHandle::from_rgba(1, 1, vec![1, 2, 3, 255]),
            captured_at: Local::now(),
            default_filename: format!("{symbol}.png"),
        }
    }

    fn capture_request(
        terminal: &TradingTerminal,
        request_id: u64,
        chart_id: u64,
    ) -> ChartScreenshotCaptureRequest {
        ChartScreenshotCaptureRequest::new(
            request_id,
            chart_id,
            terminal.chart_instance_generation,
            crate::chart_state::ChartSurfaceId::Docked(chart_id),
        )
    }

    fn rendering_capture(request: ChartScreenshotCaptureRequest) -> ChartScreenshotPendingCapture {
        let mut pending = ChartScreenshotPendingCapture::awaiting_bounds(request);
        assert!(pending.begin_rendering(request));
        pending
    }

    #[test]
    fn prior_max_id_capture_cannot_settle_reopened_capture() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let chart_id = terminal.charts.keys().copied().next().expect("chart");
        let instance = terminal.charts.get_mut(&chart_id).expect("chart instance");
        instance.interval = Timeframe::H1;
        instance.chart.candles = vec![Candle::test_ohlcv(
            1_000,
            3_600_999,
            [10.0, 12.0, 9.0, 11.0],
            5.0,
        )];
        terminal.chart_screenshot_next_request_id = u64::MAX;
        let prior_request = ChartScreenshotCaptureRequest::new(
            u64::MAX,
            chart_id,
            terminal.chart_instance_generation,
            crate::chart_state::ChartSurfaceId::Docked(chart_id),
        );
        let screenshot_window_id = iced::window::Id::unique();
        terminal.chart_screenshot_window_id = Some(screenshot_window_id);
        terminal.chart_screenshot_pending_capture = Some(rendering_capture(prior_request));
        terminal.chart_screenshot_capture_in_progress = true;

        let _task = terminal.update_window(Message::WindowClosed(screenshot_window_id));
        assert!(terminal.chart_screenshot_pending_capture.is_none());
        assert!(!terminal.chart_screenshot_capture_in_progress);

        let _task = terminal.update_chart_screenshot(Message::OpenChartScreenshot(
            chart_id,
            crate::chart_state::ChartSurfaceId::Docked(chart_id),
        ));
        let current_request = terminal
            .chart_screenshot_pending_capture
            .expect("reopened capture owner")
            .request();
        assert_eq!(current_request.request_id(), 0);
        assert!(
            terminal
                .chart_screenshot_pending_capture
                .as_mut()
                .is_some_and(|pending| pending.begin_rendering(current_request))
        );

        let _task = terminal.update_chart_screenshot(Message::ChartScreenshotCaptured(
            prior_request,
            Ok(screenshot_state("prior-capture")).into(),
        ));

        assert_eq!(
            terminal
                .chart_screenshot_pending_capture
                .map(|pending| pending.request()),
            Some(current_request),
            "a result from the closed MAX request must not settle the reopened capture"
        );
        assert!(terminal.chart_screenshot_capture_in_progress);
        assert!(terminal.chart_screenshot.is_none());
    }

    #[test]
    fn capture_result_cannot_settle_request_still_awaiting_bounds() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let request = capture_request(&terminal, 7, 1);
        terminal.chart_screenshot_pending_capture =
            Some(ChartScreenshotPendingCapture::awaiting_bounds(request));
        terminal.chart_screenshot_capture_in_progress = true;

        let _task = terminal.update_chart_screenshot(Message::ChartScreenshotCaptured(
            request,
            Ok(screenshot_state("premature-capture")).into(),
        ));

        let pending = terminal
            .chart_screenshot_pending_capture
            .expect("capture still awaits bounds");
        assert!(pending.is_awaiting_bounds(request));
        assert!(terminal.chart_screenshot_capture_in_progress);
        assert!(terminal.chart_screenshot.is_none());
    }

    #[test]
    fn prior_layout_bounds_cannot_snapshot_reconstructed_chart() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.chart_instance_generation = 2;
        let request = ChartScreenshotCaptureRequest::new(
            7,
            1,
            1,
            crate::chart_state::ChartSurfaceId::Docked(1),
        );
        terminal.chart_screenshot_pending_capture =
            Some(ChartScreenshotPendingCapture::awaiting_bounds(request));
        terminal.chart_screenshot_capture_in_progress = true;

        let _task = terminal.update_chart_screenshot(Message::ChartScreenshotBoundsResolved(
            request,
            Some(iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 640.0,
                height: 360.0,
            }),
        ));

        assert!(terminal.chart_screenshot_pending_capture.is_none());
        assert!(!terminal.chart_screenshot_capture_in_progress);
        assert_eq!(
            terminal.chart_screenshot_error.as_deref(),
            Some("Chart screenshot unavailable: chart not found")
        );
    }

    #[test]
    fn duplicate_bounds_cannot_dispatch_or_cancel_active_render() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let chart_id = terminal.charts.keys().copied().next().expect("chart");
        let request = capture_request(&terminal, 7, chart_id);
        terminal.chart_screenshot_pending_capture =
            Some(ChartScreenshotPendingCapture::awaiting_bounds(request));
        terminal.chart_screenshot_capture_in_progress = true;

        let _task = terminal.update_chart_screenshot(Message::ChartScreenshotBoundsResolved(
            request,
            Some(iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 640.0,
                height: 360.0,
            }),
        ));
        assert!(
            terminal
                .chart_screenshot_pending_capture
                .is_some_and(|pending| pending.is_rendering(request))
        );

        let _task =
            terminal.update_chart_screenshot(Message::ChartScreenshotBoundsResolved(request, None));

        assert!(
            terminal
                .chart_screenshot_pending_capture
                .is_some_and(|pending| pending.is_rendering(request))
        );
        assert!(terminal.chart_screenshot_error.is_none());
    }

    #[test]
    fn owned_render_completion_survives_later_layout_change() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let request = ChartScreenshotCaptureRequest::new(
            7,
            1,
            1,
            crate::chart_state::ChartSurfaceId::Docked(1),
        );
        terminal.chart_instance_generation = 2;
        terminal.chart_screenshot_pending_capture = Some(rendering_capture(request));
        terminal.chart_screenshot_capture_in_progress = true;

        let _task = terminal.update_chart_screenshot(Message::ChartScreenshotCaptured(
            request,
            Ok(screenshot_state("owned-render")).into(),
        ));

        assert!(terminal.chart_screenshot_pending_capture.is_none());
        assert!(!terminal.chart_screenshot_capture_in_progress);
        assert_eq!(
            terminal
                .chart_screenshot
                .as_ref()
                .map(|state| state.symbol.as_str()),
            Some("owned-render")
        );
    }

    #[test]
    fn render_request_snapshots_current_privacy_settings() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let chart_id = terminal.charts.keys().copied().next().expect("chart");
        terminal.chart_screenshot_settings.obscure_position_entry = true;
        terminal.chart_screenshot_settings.hide_positions_and_orders = true;

        let render_request = terminal.chart_screenshot_render_request(
            terminal.charts.get(&chart_id).expect("chart instance"),
            crate::chart_state::ChartSurfaceId::Docked(chart_id),
            iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 640.0,
                height: 360.0,
            },
        );
        terminal.chart_screenshot_settings.obscure_position_entry = false;
        terminal.chart_screenshot_settings.hide_positions_and_orders = false;

        assert!(render_request.chart.obscure_position_prices);
        assert!(render_request.chart.hide_positions_and_orders);
        assert!(!terminal.charts[&chart_id].chart.obscure_position_prices);
        assert!(!terminal.charts[&chart_id].chart.hide_positions_and_orders);
    }

    #[test]
    fn chart_screenshot_capture_error_redacts_window_error_and_toast() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let request = capture_request(&terminal, 7, 1);
        terminal.chart_screenshot_pending_capture = Some(rendering_capture(request));
        terminal.chart_screenshot_capture_in_progress = true;

        let _task = terminal.update_chart_screenshot(Message::ChartScreenshotCaptured(
            request,
            Err("render failed: api_key=key-secret signature=sig-secret".to_string()).into(),
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

        let _task = terminal.update_chart_screenshot(Message::ChartScreenshotCopied(
            Err("copy failed: auth_token=token-secret".to_string()).into(),
        ));

        let toast = terminal.toasts.last().expect("toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("auth_token=<redacted>"));
        assert!(!toast.message.contains("token-secret"));
    }

    #[test]
    fn chart_screenshot_copy_success_preserves_exact_toast() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());

        let _task = terminal.update_chart_screenshot(Message::ChartScreenshotCopied(Ok(()).into()));

        let toast = terminal.toasts.last().expect("toast");
        assert!(!toast.is_error);
        assert_eq!(toast.message, "Chart image copied to clipboard");
    }

    #[test]
    fn chart_screenshot_save_error_redacts_toast() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());

        let _task = terminal.update_chart_screenshot(Message::ChartScreenshotSaved(
            Err("save failed: client_secret=secret-value".to_string()).into(),
        ));

        let toast = terminal.toasts.last().expect("toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("client_secret=<redacted>"));
        assert!(!toast.message.contains("secret-value"));
    }

    #[test]
    fn chart_screenshot_save_success_and_cancel_preserve_exact_feedback() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let path = std::path::PathBuf::from("synthetic-screenshot-dir/chart.png");

        let _task = terminal
            .update_chart_screenshot(Message::ChartScreenshotSaved(Ok(Some(path.clone())).into()));

        let toast = terminal.toasts.last().expect("save toast");
        assert!(!toast.is_error);
        assert_eq!(
            toast.message,
            format!("Chart image saved to {}", path.display())
        );
        let toast_count = terminal.toasts.len();

        let _task =
            terminal.update_chart_screenshot(Message::ChartScreenshotSaved(Ok(None).into()));

        assert_eq!(terminal.toasts.len(), toast_count);
    }
}
