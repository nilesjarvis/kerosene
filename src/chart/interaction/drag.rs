use super::super::state::DragKind;
use super::super::{
    CANDLE_GAP_RATIO, CandlestickChart, ChartState, PAN_SPEED, VOLUME_REGION_RATIO,
    model::{
        FUNDING_PLOT_BOTTOM_PADDING, FUNDING_PLOT_TOP_PADDING, FUNDING_RATE_ANNUALIZATION_FACTOR,
    },
};
use super::{InteractionLayout, ProjectedCursor};
use crate::annotations::DrawingTool;
use crate::chart::fisheye::ChartFisheye;
use crate::message::Message;
use iced::Rectangle;
use iced::widget::canvas;

// ---------------------------------------------------------------------------
// Drag And Hover Handling
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn handle_cursor_moved(
        &self,
        state: &mut ChartState,
        cursor: Option<ProjectedCursor>,
        fisheye: ChartFisheye,
        layout: InteractionLayout,
        needs_redraw_for_cursor: bool,
    ) -> Option<canvas::Action<Message>> {
        let pos = cursor.map(|cursor| cursor.source);
        if let (Some(kind), Some(start), Some(pos)) = (state.drag, state.drag_start, pos) {
            match kind {
                DragKind::PanX => {
                    let step = state.candle_width * (1.0 + CANDLE_GAP_RATIO);
                    let dx = pos.x - start.x;
                    let candle_delta = dx / step * PAN_SPEED;
                    state.scroll_offset = self.clamp_scroll_offset_for(
                        state.drag_start_scroll + candle_delta,
                        layout.chart_w,
                        state.candle_width,
                    );
                    self.candle_cache.clear();
                }
                DragKind::PanY => {
                    let price_h = layout.chart_h * (1.0 - VOLUME_REGION_RATIO);
                    let dy = pos.y - start.y;
                    let visible_range = self.visible_price_range(state, layout.chart_w);
                    let price_per_px = visible_range / price_h as f64;
                    state.y_offset = state.drag_start_y_offset + (dy as f64) * price_per_px;
                    self.candle_cache.clear();
                }
                DragKind::PanFundingY => {
                    let plot_top = layout.chart_h + FUNDING_PLOT_TOP_PADDING;
                    let plot_bottom =
                        layout.chart_h + layout.funding_panel_h - FUNDING_PLOT_BOTTOM_PADDING;
                    let step = state.candle_width * (1.0 + CANDLE_GAP_RATIO);
                    if plot_bottom > plot_top
                        && let Some(range) = self.funding_display_range(state, layout.chart_w, step)
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
                        self.visible_price_params(state, layout.chart_w, layout.chart_h)
                    {
                        let clamped_y = pos.y.clamp(0.0, price_h);
                        let new_price =
                            self.y_to_price_with(clamped_y, price_hi, price_range, price_h);
                        state.drag_order_new_price = Some(new_price);
                    }
                }
                DragKind::MoveAnnotation { .. } => {
                    if let (Some(base), Some(drag_start), Some((price_hi, price_range, price_h))) = (
                        state.drag_annotation_base.clone(),
                        state.drag_start,
                        self.visible_price_params(state, layout.chart_w, layout.chart_h),
                    ) {
                        let price_start = self.y_to_price_with(
                            drag_start.y.clamp(0.0, price_h),
                            price_hi,
                            price_range,
                            price_h,
                        );
                        let price_now = self.y_to_price_with(
                            pos.y.clamp(0.0, price_h),
                            price_hi,
                            price_range,
                            price_h,
                        );
                        let ts_start = self
                            .x_to_timestamp(drag_start.x, state, layout.chart_w)
                            .unwrap_or(0);
                        let ts_now = self
                            .x_to_timestamp(pos.x, state, layout.chart_w)
                            .unwrap_or(0);
                        let mut live = base;
                        live.kind
                            .translate(ts_now as i64 - ts_start as i64, price_now - price_start);
                        state.drag_annotation = Some(live);
                    }
                }
                DragKind::MoveAnnotationAnchor { anchor_index, .. } => {
                    if let (Some(base), Some((price_hi, price_range, price_h))) = (
                        state.drag_annotation_base.clone(),
                        self.visible_price_params(state, layout.chart_w, layout.chart_h),
                    ) {
                        let price = self.y_to_price_with(
                            pos.y.clamp(0.0, price_h),
                            price_hi,
                            price_range,
                            price_h,
                        );
                        let ts = self
                            .x_to_timestamp(pos.x, state, layout.chart_w)
                            .unwrap_or(0);
                        let mut live = base;
                        live.kind.set_anchor(anchor_index, (ts, price));
                        state.drag_annotation = Some(live);
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
                DragKind::ResizeSessionPanel => {
                    let dy = pos.y - start.y;
                    let height = CandlestickChart::clamp_session_panel_height(
                        state.drag_start_session_panel_height - dy,
                    );
                    state.drag_session_panel_height = Some(height);
                    self.candle_cache.clear();
                    return Some(
                        canvas::Action::publish(Message::ChartSessionPanelHeightChanged(
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
        if self.active_tool.is_none()
            && let Some(cursor) = cursor
            && let Some(hit) = self.hit_test_order_line_at(
                state,
                cursor.source,
                cursor.visual,
                layout.chart_w,
                layout.chart_h,
                fisheye,
            )
        {
            state.hover_order_oid = Some(hit.order.oid);
        }

        // Hover feedback for the Select tool (grab cursor over annotations).
        let old_hover_ann = state.hover_annotation;
        state.hover_annotation = None;
        if self.active_tool == Some(DrawingTool::Select)
            && let Some(cursor) = cursor
            && let Some(hit) =
                self.hit_test_annotation(state, cursor.source, layout.chart_w, layout.chart_h)
        {
            state.hover_annotation = Some(hit.id);
        }

        if needs_redraw_for_cursor
            || state.hover_order_oid != old_hover
            || state.hover_annotation != old_hover_ann
        {
            Some(canvas::Action::request_redraw())
        } else {
            None
        }
    }

    pub(in crate::chart) fn handle_left_release(
        &self,
        state: &mut ChartState,
        bounds: Rectangle,
    ) -> Option<canvas::Action<Message>> {
        if let Some(DragKind::MoveOrder { oid }) = state.drag {
            let coin = state.drag_order_coin.take();
            let new_price = state.drag_order_new_price.take();
            state.drag = None;
            state.drag_start = None;
            if let (Some(coin), Some(price)) = (coin, new_price) {
                return Some(
                    canvas::Action::publish(Message::MoveOrder {
                        coin: coin.into(),
                        oid: oid.into(),
                        new_price: price.into(),
                    })
                    .and_capture(),
                );
            }
            return Some(canvas::Action::request_redraw());
        }
        if matches!(
            state.drag,
            Some(DragKind::MoveAnnotation { .. } | DragKind::MoveAnnotationAnchor { .. })
        ) {
            let live = state.drag_annotation.take();
            state.drag_annotation_base = None;
            state.drag = None;
            state.drag_start = None;
            if let Some(annotation) = live
                && annotation.is_valid()
            {
                return Some(
                    canvas::Action::publish(Message::UpdateAnnotation(self.id, annotation))
                        .and_capture(),
                );
            }
            return Some(canvas::Action::request_redraw());
        }
        if let Some(kind) = state.drag {
            let funding_height = state.drag_funding_panel_height.take();
            let session_height = state.drag_session_panel_height.take();
            state.drag = None;
            state.drag_start = None;
            if matches!(
                kind,
                DragKind::PanX | DragKind::PanY | DragKind::PanFundingY
            ) {
                // Drag frames tessellate the heatmap at the reduced panning
                // budget; redraw once at full fidelity now the gesture ended.
                self.candle_cache.clear();
            }
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
            if matches!(kind, DragKind::ResizeSessionPanel) {
                let height = session_height
                    .unwrap_or(self.session_panel_height)
                    .round()
                    .clamp(
                        super::super::MIN_SESSION_PANEL_HEIGHT,
                        super::super::MAX_SESSION_PANEL_HEIGHT,
                    ) as u16;
                return Some(
                    canvas::Action::publish(Message::ChartSessionPanelHeightChanged(
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
