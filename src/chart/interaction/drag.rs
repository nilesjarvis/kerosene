use super::super::state::DragKind;
use super::super::{
    CANDLE_GAP_RATIO, CandlestickChart, ChartState, PAN_SPEED, VOLUME_REGION_RATIO,
    model::{
        FUNDING_PLOT_BOTTOM_PADDING, FUNDING_PLOT_TOP_PADDING, FUNDING_RATE_ANNUALIZATION_FACTOR,
    },
};
use crate::message::Message;
use iced::widget::canvas;
use iced::{Point, Rectangle};

// ---------------------------------------------------------------------------
// Drag And Hover Handling
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn handle_cursor_moved(
        &self,
        state: &mut ChartState,
        pos: Option<Point>,
        chart_w: f32,
        chart_h: f32,
        funding_panel_h: f32,
        needs_redraw_for_cursor: bool,
    ) -> Option<canvas::Action<Message>> {
        if let (Some(kind), Some(start), Some(pos)) = (state.drag, state.drag_start, pos) {
            match kind {
                DragKind::PanX => {
                    let step = state.candle_width * (1.0 + CANDLE_GAP_RATIO);
                    let dx = pos.x - start.x;
                    let candle_delta = dx / step * PAN_SPEED;
                    state.scroll_offset = self.clamp_scroll_offset_for(
                        state.drag_start_scroll + candle_delta,
                        chart_w,
                        state.candle_width,
                    );
                    self.candle_cache.clear();
                }
                DragKind::PanY => {
                    let price_h = chart_h * (1.0 - VOLUME_REGION_RATIO);
                    let dy = pos.y - start.y;
                    let visible_range = self.visible_price_range(state, chart_w);
                    let price_per_px = visible_range / price_h as f64;
                    state.y_offset = state.drag_start_y_offset + (dy as f64) * price_per_px;
                    self.candle_cache.clear();
                }
                DragKind::PanFundingY => {
                    let plot_top = chart_h + FUNDING_PLOT_TOP_PADDING;
                    let plot_bottom = chart_h + funding_panel_h - FUNDING_PLOT_BOTTOM_PADDING;
                    let step = state.candle_width * (1.0 + CANDLE_GAP_RATIO);
                    if plot_bottom > plot_top
                        && let Some(range) = self.funding_display_range(state, chart_w, step)
                    {
                        let display_delta = range.y_to_rate(start.y, plot_top, plot_bottom)
                            - range.y_to_rate(pos.y, plot_top, plot_bottom);
                        let raw_delta = if self.funding_annualized {
                            display_delta / FUNDING_RATE_ANNUALIZATION_FACTOR
                        } else {
                            display_delta
                        };
                        state.funding_y_offset = state.drag_start_y_offset + raw_delta;
                        self.candle_cache.clear();
                    }
                }
                DragKind::MoveOrder { .. } => {
                    if let Some((price_hi, price_range, price_h)) =
                        self.visible_price_params(state, chart_w, chart_h)
                    {
                        let clamped_y = pos.y.clamp(0.0, price_h);
                        let new_price =
                            self.y_to_price_with(clamped_y, price_hi, price_range, price_h);
                        state.drag_order_new_price = Some(new_price);
                    }
                }
                DragKind::ResizeFundingPanel => {
                    let dy = pos.y - start.y;
                    let height = CandlestickChart::clamp_funding_panel_height(
                        state.drag_start_funding_panel_height - dy,
                    );
                    state.drag_funding_panel_height = Some(height);
                    self.candle_cache.clear();
                    return Some(
                        canvas::Action::publish(Message::ChartFundingPanelHeightChanged(
                            self.id,
                            height.round() as u16,
                            false,
                        ))
                        .and_capture(),
                    );
                }
            }
            return Some(canvas::Action::request_redraw());
        }

        let old_hover = state.hover_order_oid;
        state.hover_order_oid = None;
        if let Some(pos) = pos
            && let Some(hit) = self.hit_test_order_line(state, pos, chart_w, chart_h)
        {
            state.hover_order_oid = Some(hit.order.oid);
        }
        if needs_redraw_for_cursor || state.hover_order_oid != old_hover {
            Some(canvas::Action::request_redraw())
        } else {
            None
        }
    }

    pub(super) fn handle_left_release(
        &self,
        state: &mut ChartState,
        bounds: Rectangle,
    ) -> Option<canvas::Action<Message>> {
        if let Some(DragKind::MoveOrder { oid }) = state.drag {
            let new_price = state.drag_order_new_price.take();
            state.drag = None;
            state.drag_start = None;
            if let Some(price) = new_price {
                return Some(
                    canvas::Action::publish(Message::MoveOrder {
                        oid,
                        new_price: price,
                    })
                    .and_capture(),
                );
            }
            return Some(canvas::Action::request_redraw());
        }
        if let Some(kind) = state.drag {
            let funding_height = state.drag_funding_panel_height.take();
            state.drag = None;
            state.drag_start = None;
            if matches!(kind, DragKind::ResizeFundingPanel) {
                let height = funding_height
                    .unwrap_or(self.funding_panel_height)
                    .round()
                    .clamp(
                        super::super::MIN_FUNDING_PANEL_HEIGHT,
                        super::super::MAX_FUNDING_PANEL_HEIGHT,
                    ) as u16;
                return Some(
                    canvas::Action::publish(Message::ChartFundingPanelHeightChanged(
                        self.id, height, true,
                    ))
                    .and_capture(),
                );
            }
            if matches!(kind, DragKind::PanX | DragKind::PanY)
                && let Some(action) = self.viewport_action(state, bounds)
            {
                return Some(action);
            }
            return Some(canvas::Action::request_redraw());
        }
        None
    }
}
