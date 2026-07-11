use super::super::state::DragKind;
use super::super::{CandlestickChart, ChartState, VOLUME_REGION_RATIO};
use super::{InteractionLayout, ProjectedCursor};
use crate::chart::fisheye::ChartFisheye;
use crate::message::Message;
use crate::order_execution::{HudOrderRequest, HudOrderSide, HudOrderType};
use iced::Rectangle;
use iced::widget::canvas;

// ---------------------------------------------------------------------------
// Mouse Press Handling
// ---------------------------------------------------------------------------

impl CandlestickChart {
    #[cfg(test)]
    pub(in crate::chart) fn handle_left_press(
        &self,
        state: &mut ChartState,
        pos: iced::Point,
        chart_w: f32,
        chart_h: f32,
        bounds_height: f32,
    ) -> Option<canvas::Action<Message>> {
        self.handle_left_press_at(
            state,
            ProjectedCursor::identity(pos),
            ChartFisheye::disabled(),
            InteractionLayout::without_funding(chart_w, chart_h),
            bounds_height,
        )
    }

    pub(in crate::chart) fn handle_left_press_at(
        &self,
        state: &mut ChartState,
        cursor: ProjectedCursor,
        fisheye: ChartFisheye,
        layout: InteractionLayout,
        bounds_height: f32,
    ) -> Option<canvas::Action<Message>> {
        let pos = cursor.source;
        let visual_pos = cursor.visual;
        let chart_w = layout.chart_w;
        let chart_h = layout.chart_h;

        if self.funding_mode_button_contains(bounds_height, visual_pos, chart_w) {
            return Some(
                canvas::Action::publish(Message::ToggleFundingRateDisplayMode(self.id))
                    .and_capture(),
            );
        }

        if visual_pos.x < chart_w
            && self
                .funding_panel_resize_target_y(bounds_height, visual_pos.y)
                .is_some()
        {
            state.drag = Some(DragKind::ResizeFundingPanel);
            state.drag_start = Some(pos);
            state.drag_start_funding_panel_height = self.funding_panel_height;
            state.drag_funding_panel_height = Some(self.funding_panel_height);
            return Some(canvas::Action::capture());
        }

        if visual_pos.x < chart_w
            && self
                .session_panel_resize_target_y(bounds_height, visual_pos.y)
                .is_some()
        {
            state.drag = Some(DragKind::ResizeSessionPanel);
            state.drag_start = Some(pos);
            state.drag_start_session_panel_height = self.session_panel_height;
            state.drag_session_panel_height = Some(self.session_panel_height);
            return Some(canvas::Action::capture());
        }

        if visual_pos.x < chart_w && visual_pos.y < chart_h {
            if self.quick_order_open {
                return Some(
                    canvas::Action::publish(Message::CloseQuickOrder(self.id)).and_capture(),
                );
            }

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

        if let Some(tool) = self.active_tool
            && visual_pos.x < chart_w
            && visual_pos.y < chart_h
        {
            // Drawing tools own chart clicks before live-order hit testing.
            // This prevents an edit/select click near an order line from
            // accidentally moving or cancelling a real order.
            if tool == crate::annotations::DrawingTool::Select {
                return self.handle_select_press(state, pos, chart_w, chart_h);
            }
            return self.handle_drawing_tool_press(state, pos, chart_w, chart_h, tool);
        }

        if visual_pos.x < chart_w && visual_pos.y < chart_h {
            let price_h = chart_h * (1.0 - VOLUME_REGION_RATIO);
            if let Some(time_ms) = self.hit_test_earnings_marker_at(state, pos, chart_w, price_h) {
                return Some(
                    canvas::Action::publish(Message::OpenChartEarningsFiling(
                        self.id,
                        self.surface_id,
                        time_ms,
                    ))
                    .and_capture(),
                );
            }
        }

        if let Some(hit) =
            self.hit_test_order_line_at(state, pos, visual_pos, chart_w, chart_h, fisheye)
        {
            if hit.is_cancel_hit(visual_pos) {
                return Some(
                    canvas::Action::publish(Message::CancelOrder {
                        coin: hit.order.coin.clone().into(),
                        oid: hit.order.oid.into(),
                    })
                    .and_capture(),
                );
            }

            state.drag = Some(DragKind::MoveOrder { oid: hit.order.oid });
            state.drag_start = Some(pos);
            state.drag_order_coin = Some(hit.order.coin.clone());
            state.drag_order_new_price = Some(hit.order.limit_px);
            return Some(
                canvas::Action::publish(Message::MoveOrderDragStarted {
                    coin: hit.order.coin.clone().into(),
                    oid: hit.order.oid.into(),
                })
                .and_capture(),
            );
        }

        if self.hud_game_mode_enabled()
            && self.hud_armed
            && !state.ctrl_down
            && !state.shift_down
            && visual_pos.x < chart_w
            && visual_pos.y < chart_h
            && let Some((price_hi, price_range, price_h)) =
                self.visible_price_params(state, chart_w, chart_h)
        {
            // No-fire deadzone over the weapon station and the open selector
            // popup: a swallowed click costs a re-click, a misplaced order
            // costs money. Both are drawn in screen space, so hit-test the
            // physical cursor.
            let station_hit = super::super::crosshair::hud_station_metrics(chart_w, price_h)
                .is_some_and(|metrics| metrics.bounds.contains(visual_pos));
            let selector_hit = self.hud_weapon_selector.is_some()
                && super::super::crosshair::hud_selector_bounds(chart_w, price_h)
                    .is_some_and(|bounds| bounds.contains(visual_pos));
            if station_hit || selector_hit {
                return Some(canvas::Action::request_redraw().and_capture());
            }

            let clamped_y = pos.y.clamp(0.0, price_h);
            let price = self.y_to_price_with(clamped_y, price_hi, price_range, price_h);
            let order_type = match state.hud_order_kind {
                super::super::state::HudOrderKind::Limit => HudOrderType::Limit,
                super::super::state::HudOrderKind::Market => HudOrderType::Market,
            };
            let limit_side = if order_type == HudOrderType::Limit {
                self.hud_limit_click_is_buy(price).map(|is_buy| {
                    if is_buy {
                        HudOrderSide::Long
                    } else {
                        HudOrderSide::Short
                    }
                })
            } else {
                None
            };
            return Some(
                canvas::Action::publish(Message::SubmitHudOrder(HudOrderRequest {
                    chart_id: self.id,
                    surface_id: self.surface_id,
                    symbol_key: self.symbol_key.clone(),
                    price,
                    quantity: hud_order_quantity(state),
                    order_type,
                    market_side: match state.hud_market_side {
                        super::super::state::HudMarketSide::Long => HudOrderSide::Long,
                        super::super::state::HudMarketSide::Short => HudOrderSide::Short,
                    },
                    limit_side,
                    click_x: pos.x,
                    click_y: pos.y,
                    chart_w,
                    chart_h,
                }))
                .and_capture(),
            );
        }

        let in_session_panel = layout.session_panel_h > 0.0
            && visual_pos.x < chart_w
            && visual_pos.y >= chart_h + layout.funding_panel_h
            && visual_pos.y < chart_h + layout.funding_panel_h + layout.session_panel_h;
        let in_main_plot = visual_pos.x < chart_w && visual_pos.y < chart_h;

        if visual_pos.x >= chart_w && visual_pos.y < chart_h {
            state.drag = Some(DragKind::PanY);
            state.drag_start = Some(pos);
            state.drag_start_y_offset = state.y_offset;
            if state.y_auto {
                state.y_auto = false;
                state.y_offset = 0.0;
                state.y_scale = 1.0;
                state.drag_start_y_offset = 0.0;
            }
        } else if layout.funding_panel_h > 0.0
            && visual_pos.y >= chart_h
            && visual_pos.y < chart_h + layout.funding_panel_h
        {
            state.drag = Some(DragKind::PanFundingY);
            state.drag_start = Some(pos);
            state.drag_start_y_offset = state.funding_y_offset;
        } else if in_session_panel || in_main_plot {
            state.drag = Some(DragKind::PanX);
            state.drag_start = Some(pos);
            state.drag_start_scroll = state.scroll_offset;
        }
        Some(canvas::Action::publish(Message::ChartFocused(self.id)))
    }

    #[cfg(test)]
    pub(in crate::chart) fn handle_right_press(
        &self,
        state: &mut ChartState,
        bounds: Rectangle,
        pos: iced::Point,
        chart_w: f32,
        chart_h: f32,
    ) -> Option<canvas::Action<Message>> {
        self.handle_right_press_at(
            state,
            bounds,
            ProjectedCursor::identity(pos),
            ChartFisheye::disabled(),
            InteractionLayout::without_funding(chart_w, chart_h),
        )
    }

    pub(in crate::chart) fn handle_right_press_at(
        &self,
        state: &mut ChartState,
        bounds: Rectangle,
        cursor: ProjectedCursor,
        fisheye: ChartFisheye,
        layout: InteractionLayout,
    ) -> Option<canvas::Action<Message>> {
        let pos = cursor.source;
        let visual_pos = cursor.visual;
        let chart_w = layout.chart_w;
        let chart_h = layout.chart_h;

        if self.quick_order_open
            && visual_pos.x < chart_w
            && visual_pos.y < chart_h
            && let Some((price_hi, price_range, price_h)) =
                self.visible_price_params(state, chart_w, chart_h)
        {
            let clamped_y = pos.y.clamp(0.0, price_h);
            let price = self.y_to_price_with(clamped_y, price_hi, price_range, price_h);
            return Some(
                canvas::Action::publish(Message::OpenQuickOrder(
                    self.id,
                    self.surface_id,
                    price.into(),
                    pos.x,
                    pos.y,
                    chart_w,
                    chart_h,
                ))
                .and_capture(),
            );
        }

        if visual_pos.x < chart_w && visual_pos.y < chart_h && state.range_anchor_price.is_some() {
            state.range_anchor_price = None;
            return Some(canvas::Action::request_redraw());
        }

        if self.active_tool.is_some() && visual_pos.x < chart_w && visual_pos.y < chart_h {
            return Some(
                canvas::Action::publish(Message::ClearDrawingTool(self.id, self.surface_id))
                    .and_capture(),
            );
        }

        if let Some(hit) =
            self.hit_test_order_line_at(state, pos, visual_pos, chart_w, chart_h, fisheye)
        {
            return Some(
                canvas::Action::publish(Message::CancelOrder {
                    coin: hit.order.coin.clone().into(),
                    oid: hit.order.oid.into(),
                })
                .and_capture(),
            );
        }

        if visual_pos.x >= chart_w && visual_pos.y < chart_h {
            state.y_auto = true;
            state.y_offset = 0.0;
            state.y_scale = 1.0;
            self.candle_cache.clear();
            if let Some(action) = self.viewport_action(state, bounds) {
                return Some(action);
            }
            return Some(canvas::Action::request_redraw());
        }

        if layout.funding_panel_h > 0.0
            && visual_pos.x >= chart_w
            && visual_pos.y >= chart_h
            && visual_pos.y < chart_h + layout.funding_panel_h
        {
            state.funding_y_scale = 1.0;
            state.funding_y_offset = 0.0;
            self.candle_cache.clear();
            return Some(canvas::Action::request_redraw());
        }

        if visual_pos.x < chart_w
            && visual_pos.y < chart_h
            && let Some((price_hi, price_range, price_h)) =
                self.visible_price_params(state, chart_w, chart_h)
        {
            let clamped_y = pos.y.clamp(0.0, price_h);
            let price = self.y_to_price_with(clamped_y, price_hi, price_range, price_h);
            return Some(
                canvas::Action::publish(Message::OpenQuickOrder(
                    self.id,
                    self.surface_id,
                    price.into(),
                    pos.x,
                    pos.y,
                    chart_w,
                    chart_h,
                ))
                .and_capture(),
            );
        }
        None
    }
}

impl CandlestickChart {
    /// Select-tool press: pick the annotation under the cursor and begin a
    /// body/anchor drag, or clear the selection when clicking empty space.
    pub(in crate::chart) fn handle_select_press(
        &self,
        state: &mut ChartState,
        pos: iced::Point,
        chart_w: f32,
        chart_h: f32,
    ) -> Option<canvas::Action<Message>> {
        use super::super::viewport::annotations::AnnotationHitPart;

        if let Some(hit) = self.hit_test_annotation(state, pos, chart_w, chart_h) {
            state.selected_annotation = Some(hit.id);
            // Locked drawings can be selected (to restyle / unlock) but not
            // moved; only start a drag for unlocked annotations.
            if let Some(base) = self
                .annotations
                .iter()
                .find(|ann| ann.id == hit.id)
                .cloned()
                .filter(|ann| !ann.style.locked)
            {
                state.drag_annotation_base = Some(base.clone());
                state.drag_annotation = Some(base);
                state.drag_start = Some(pos);
                state.drag = Some(match hit.part {
                    AnnotationHitPart::Body => DragKind::MoveAnnotation { id: hit.id },
                    AnnotationHitPart::Anchor(anchor_index) => DragKind::MoveAnnotationAnchor {
                        id: hit.id,
                        anchor_index,
                    },
                });
            }
            return Some(
                canvas::Action::publish(Message::SelectAnnotation(self.id, Some(hit.id)))
                    .and_capture(),
            );
        }

        if state.selected_annotation.take().is_some() {
            return Some(
                canvas::Action::publish(Message::SelectAnnotation(self.id, None)).and_capture(),
            );
        }
        Some(canvas::Action::capture())
    }
}

fn hud_order_quantity(state: &ChartState) -> String {
    let quantity = state.hud_size_input.trim();
    if quantity.is_empty() {
        "0".to_string()
    } else {
        quantity.to_string()
    }
}
