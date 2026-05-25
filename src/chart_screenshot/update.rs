use crate::app_state::TradingTerminal;
use crate::message::Message;

use self::bounds::FindWidgetBounds;
use super::capture::{
    copy_chart_screenshot_to_clipboard, render_chart_screenshot, save_chart_screenshot_png,
};

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

                self.chart_screenshot_next_request_id =
                    self.chart_screenshot_next_request_id.saturating_add(1);
                let request_id = self.chart_screenshot_next_request_id;
                self.chart_screenshot_pending_request_id = Some(request_id);
                self.chart_screenshot_capture_in_progress = true;
                self.chart_screenshot_error = None;
                self.chart_screenshot = None;

                let target = Self::chart_screenshot_canvas_id(surface_id);
                let bounds_task = iced::advanced::widget::operate(FindWidgetBounds::new(target))
                    .map(move |bounds| {
                        Message::ChartScreenshotBoundsResolved(
                            request_id, chart_id, surface_id, bounds,
                        )
                    });
                return self.open_or_focus_chart_screenshot_window(bounds_task);
            }
            Message::ChartScreenshotBoundsResolved(
                request_id,
                chart_id,
                surface_id,
                Some(bounds),
            ) => {
                if self.chart_screenshot_pending_request_id != Some(request_id) {
                    return Task::none();
                }

                let Some(instance) = self.charts.get(&chart_id) else {
                    self.finish_chart_screenshot_error(
                        request_id,
                        "Chart screenshot unavailable: chart not found".to_string(),
                    );
                    return Task::none();
                };

                let request = self.chart_screenshot_render_request(instance, surface_id, bounds);

                return Task::perform(render_chart_screenshot(request), move |result| {
                    Message::ChartScreenshotCaptured(request_id, chart_id, result)
                });
            }
            Message::ChartScreenshotBoundsResolved(request_id, _, _, None) => {
                self.finish_chart_screenshot_error(
                    request_id,
                    "Chart screenshot unavailable: chart area was not visible".to_string(),
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
                Err(err) => self.push_toast(format!("Chart image copy failed: {err}"), true),
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
                Err(err) => self.push_toast(format!("Chart image save failed: {err}"), true),
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
