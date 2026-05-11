use super::super::state::DragKind;
use super::super::{CandlestickChart, ChartState};
use crate::message::Message;
use iced::widget::canvas;
use iced::{Point, Rectangle};

// ---------------------------------------------------------------------------
// Mouse Press Handling
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn handle_left_press(
        &self,
        state: &mut ChartState,
        pos: Point,
        chart_w: f32,
        chart_h: f32,
        bounds_height: f32,
    ) -> Option<canvas::Action<Message>> {
        if self.funding_mode_button_contains(bounds_height, pos, chart_w) {
            return Some(
                canvas::Action::publish(Message::ToggleFundingRateDisplayMode(self.id))
                    .and_capture(),
            );
        }

        if pos.x < chart_w
            && self
                .funding_panel_resize_target_y(bounds_height, pos.y)
                .is_some()
        {
            state.drag = Some(DragKind::ResizeFundingPanel);
            state.drag_start = Some(pos);
            state.drag_start_funding_panel_height = self.funding_panel_height;
            state.drag_funding_panel_height = Some(self.funding_panel_height);
            return Some(canvas::Action::capture());
        }

        if pos.x < chart_w && pos.y < chart_h {
            if state.range_anchor_price.is_some() {
                state.range_anchor_price = None;
                return Some(canvas::Action::request_redraw());
            }

            if state.shift_down
                && let Some((price_hi, price_range, price_h)) =
                    self.visible_price_params(state, chart_w, chart_h)
            {
                let clamped_y = pos.y.clamp(0.0, price_h);
                let price = self.y_to_price_with(clamped_y, price_hi, price_range, price_h);
                state.range_anchor_price = Some(price);
                return Some(canvas::Action::request_redraw());
            }
        }

        if let Some(hit) = self.hit_test_order_line(state, pos, chart_w, chart_h) {
            let (cancel_x, cancel_end) = Self::order_cancel_x_range(hit.order);
            if pos.x >= cancel_x && pos.x <= cancel_end {
                return Some(
                    canvas::Action::publish(Message::CancelOrder {
                        coin: hit.order.coin.clone(),
                        oid: hit.order.oid,
                    })
                    .and_capture(),
                );
            }

            state.drag = Some(DragKind::MoveOrder { oid: hit.order.oid });
            state.drag_start = Some(pos);
            state.drag_order_new_price = Some(hit.order.limit_px);
            return Some(canvas::Action::capture());
        }

        if let Some(tool) = self.active_tool
            && pos.x < chart_w
            && pos.y < chart_h
        {
            return self.handle_drawing_tool_press(state, pos, chart_w, chart_h, tool);
        }

        let (_, funding_panel_h) = self.chart_area_heights(bounds_height);
        if pos.x >= chart_w && pos.y < chart_h {
            state.drag = Some(DragKind::PanY);
            state.drag_start = Some(pos);
            state.drag_start_y_offset = state.y_offset;
            if state.y_auto {
                state.y_auto = false;
                state.y_offset = 0.0;
                state.y_scale = 1.0;
                state.drag_start_y_offset = 0.0;
            }
        } else if funding_panel_h > 0.0 && pos.y >= chart_h && pos.y < chart_h + funding_panel_h {
            state.drag = Some(DragKind::PanFundingY);
            state.drag_start = Some(pos);
            state.drag_start_y_offset = state.funding_y_offset;
        } else if pos.x < chart_w && pos.y < chart_h {
            state.drag = Some(DragKind::PanX);
            state.drag_start = Some(pos);
            state.drag_start_scroll = state.scroll_offset;
        }
        None
    }

    pub(super) fn handle_right_press(
        &self,
        state: &mut ChartState,
        bounds: Rectangle,
        pos: Point,
        chart_w: f32,
        chart_h: f32,
    ) -> Option<canvas::Action<Message>> {
        if pos.x < chart_w && pos.y < chart_h && state.range_anchor_price.is_some() {
            state.range_anchor_price = None;
            return Some(canvas::Action::request_redraw());
        }

        if let Some(hit) = self.hit_test_order_line(state, pos, chart_w, chart_h) {
            return Some(
                canvas::Action::publish(Message::CancelOrder {
                    coin: hit.order.coin.clone(),
                    oid: hit.order.oid,
                })
                .and_capture(),
            );
        }

        if self.active_tool.is_some() && pos.x < chart_w && pos.y < chart_h {
            return Some(canvas::Action::publish(Message::ClearDrawingTool).and_capture());
        }

        if pos.x >= chart_w && pos.y < chart_h {
            state.y_auto = true;
            state.y_offset = 0.0;
            state.y_scale = 1.0;
            self.candle_cache.clear();
            if let Some(action) = self.viewport_action(state, bounds) {
                return Some(action);
            }
            return Some(canvas::Action::request_redraw());
        }

        let (_, funding_panel_h) = self.chart_area_heights(bounds.height);
        if funding_panel_h > 0.0
            && pos.x >= chart_w
            && pos.y >= chart_h
            && pos.y < chart_h + funding_panel_h
        {
            state.funding_y_scale = 1.0;
            state.funding_y_offset = 0.0;
            self.candle_cache.clear();
            return Some(canvas::Action::request_redraw());
        }

        if pos.x < chart_w
            && pos.y < chart_h
            && let Some((price_hi, price_range, price_h)) =
                self.visible_price_params(state, chart_w, chart_h)
        {
            let clamped_y = pos.y.clamp(0.0, price_h);
            let price = self.y_to_price_with(clamped_y, price_hi, price_range, price_h);
            return Some(
                canvas::Action::publish(Message::OpenQuickOrder(
                    self.id, price, pos.x, pos.y, chart_w, chart_h,
                ))
                .and_capture(),
            );
        }
        None
    }
}
