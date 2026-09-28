use super::super::state::DragKind;
use super::super::{CandlestickChart, ChartState};
use crate::annotations::{
    Anchor, Annotation, AnnotationKind, AnnotationStyle, DrawingTool, FibKind, MAX_PEN_POINTS,
};
use crate::message::Message;
use iced::Point;
use iced::keyboard::{self, key};
use iced::widget::canvas;

// ---------------------------------------------------------------------------
// Drawing Tool Press Handling
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn handle_drawing_tool_press(
        &self,
        state: &mut ChartState,
        pos: Point,
        chart_w: f32,
        chart_h: f32,
        tool: DrawingTool,
    ) -> Option<canvas::Action<Message>> {
        // Eraser deletes the nearest annotation under the cursor, but leaves
        // locked drawings alone (unlock them via the Select-tool style bar).
        if tool == DrawingTool::Eraser {
            if let Some(hit) = self.hit_test_annotation(state, pos, chart_w, chart_h) {
                let locked = self
                    .annotations
                    .iter()
                    .any(|ann| ann.id == hit.id && ann.style.locked);
                if !locked {
                    return Some(
                        canvas::Action::publish(Message::RemoveAnnotation(self.id, hit.id))
                            .and_capture(),
                    );
                }
            }
            return Some(canvas::Action::capture());
        }

        // Select is handled before we get here (in handle_left_press_at).
        if !tool.is_shape() {
            return Some(canvas::Action::capture());
        }

        let Some((price_hi, price_range, price_h)) =
            self.visible_price_params(state, chart_w, chart_h)
        else {
            return Some(canvas::Action::capture());
        };
        if tool == DrawingTool::Pen {
            if pos.y > price_h {
                return Some(canvas::Action::capture());
            }
            state.draft_anchors.clear();
            state.draft_tool = Some(tool);
            state.drag = Some(DragKind::DrawPen);
            state.drag_start = Some(pos);
            self.sample_pen_point(state, pos, chart_w, chart_h, true);
            return Some(canvas::Action::request_redraw().and_capture());
        }
        let clamped_y = pos.y.clamp(0.0, price_h);
        let price = self.y_to_price_with(clamped_y, price_hi, price_range, price_h);
        let ts = self.x_to_timestamp(pos.x, state, chart_w).unwrap_or(0);

        // Drafts are per-tool; switching tools abandons any in-progress shape.
        if state.draft_tool != Some(tool) {
            state.draft_anchors.clear();
            state.draft_tool = Some(tool);
        }

        state.draft_anchors.push((ts, price));
        let needed = tool.anchor_count();
        if state.draft_anchors.len() < needed {
            return Some(canvas::Action::request_redraw().and_capture());
        }

        let anchors = std::mem::take(&mut state.draft_anchors);
        state.draft_tool = None;
        if let Some(kind) = build_annotation_kind(tool, &anchors) {
            let annotation = Annotation {
                id: 0,
                kind,
                style: AnnotationStyle::for_tool(tool),
            };
            return Some(
                canvas::Action::publish(Message::AddAnnotation(self.id, annotation)).and_capture(),
            );
        }
        Some(canvas::Action::request_redraw().and_capture())
    }

    /// Sample in chart coordinates so strokes follow the viewport. Small mouse
    /// movements are filtered; long strokes are decimated to bound geometry.
    pub(super) fn sample_pen_point(
        &self,
        state: &mut ChartState,
        pos: Point,
        chart_w: f32,
        chart_h: f32,
        force: bool,
    ) {
        let Some((price_hi, price_range, price_h)) =
            self.visible_price_params(state, chart_w, chart_h)
        else {
            return;
        };
        let pos = Point::new(pos.x.clamp(0.0, chart_w), pos.y.clamp(0.0, price_h));
        let Some(time) = self.x_to_timestamp(pos.x, state, chart_w) else {
            return;
        };
        let price = self.y_to_price_with(pos.y, price_hi, price_range, price_h);
        if !price.is_finite() || price <= 0.0 {
            return;
        }
        if let Some(&(last_time, last_price)) = state.draft_anchors.last() {
            if (time, price) == (last_time, last_price) {
                return;
            }
            if !force && let Some(x) = self.timestamp_to_x(last_time, state, chart_w) {
                let y = self.price_to_y_with(last_price, price_hi, price_range, price_h);
                if pos.distance(Point::new(x, y)) < 2.0 {
                    return;
                }
            }
        }
        if state.draft_anchors.len() >= MAX_PEN_POINTS {
            let last = state.draft_anchors.len() - 1;
            let mut index: usize = 0;
            state.draft_anchors.retain(|_| {
                let keep = index.is_multiple_of(2) || index == last;
                index += 1;
                keep
            });
        }
        state.draft_anchors.push((time, price));
    }

    pub(super) fn finish_pen_stroke(
        &self,
        state: &mut ChartState,
        chart_w: f32,
        chart_h: f32,
    ) -> Option<canvas::Action<Message>> {
        if self.active_tool == Some(DrawingTool::Pen)
            && let Some(pos) = state.cursor_position
        {
            self.sample_pen_point(state, pos, chart_w, chart_h, true);
        }
        let points = std::mem::take(&mut state.draft_anchors);
        state.draft_tool = None;
        state.drag = None;
        state.drag_start = None;
        let annotation = Annotation {
            id: 0,
            kind: AnnotationKind::Pen { points },
            style: AnnotationStyle::for_tool(DrawingTool::Pen),
        };
        if self.active_tool == Some(DrawingTool::Pen) && annotation.is_valid() {
            return Some(
                canvas::Action::publish(Message::AddAnnotation(self.id, annotation)).and_capture(),
            );
        }
        Some(canvas::Action::request_redraw().and_capture())
    }
}

impl CandlestickChart {
    /// Non-HUD keyboard handling for drawing mode: Escape cancels the active
    /// tool / in-progress draft / selection, and Delete removes the selected
    /// annotation. Returns `None` for keys it does not own.
    pub(in crate::chart) fn handle_drawing_key_pressed(
        &self,
        state: &mut ChartState,
        key: keyboard::Key<&str>,
        modifiers: keyboard::Modifiers,
    ) -> Option<canvas::Action<Message>> {
        if modifiers.control() || modifiers.alt() || modifiers.logo() {
            return None;
        }
        match key {
            keyboard::Key::Named(key::Named::Escape) => {
                let active = self.active_tool.is_some()
                    || !state.draft_anchors.is_empty()
                    || state.selected_annotation.is_some()
                    || state.drag_annotation.is_some();
                if !active {
                    return None;
                }
                state.draft_anchors.clear();
                state.draft_tool = None;
                state.selected_annotation = None;
                state.drag_annotation = None;
                state.drag_annotation_base = None;
                if matches!(
                    state.drag,
                    Some(
                        DragKind::DrawPen
                            | DragKind::MoveAnnotation { .. }
                            | DragKind::MoveAnnotationAnchor { .. }
                    )
                ) {
                    state.drag = None;
                    state.drag_start = None;
                }
                Some(
                    canvas::Action::publish(Message::ClearDrawingTool(self.id, self.surface_id))
                        .and_capture(),
                )
            }
            keyboard::Key::Named(key::Named::Delete | key::Named::Backspace) => {
                // Only the Select tool owns Delete; a stale selection left over
                // after switching tools must not silently delete a drawing.
                if self.active_tool != Some(DrawingTool::Select) {
                    return None;
                }
                let id = state.selected_annotation?;
                let Some(annotation) = self.annotations.iter().find(|ann| ann.id == id) else {
                    state.selected_annotation = None;
                    return Some(canvas::Action::capture());
                };
                if annotation.style.locked {
                    return Some(canvas::Action::capture());
                }
                state.selected_annotation = None;
                Some(canvas::Action::publish(Message::RemoveAnnotation(self.id, id)).and_capture())
            }
            _ => None,
        }
    }
}

/// Build the annotation kind a finished `tool` draft commits to.
fn build_annotation_kind(tool: DrawingTool, anchors: &[Anchor]) -> Option<AnnotationKind> {
    let two = |build: fn(Anchor, Anchor) -> AnnotationKind| -> Option<AnnotationKind> {
        match anchors {
            [a, b, ..] => Some(build(*a, *b)),
            _ => None,
        }
    };
    match tool {
        DrawingTool::HorizontalLevel => anchors
            .first()
            .map(|(_, price)| AnnotationKind::HorizontalLevel { price: *price }),
        DrawingTool::VerticalLine => anchors
            .first()
            .map(|(time, _)| AnnotationKind::VerticalLine { time: *time }),
        DrawingTool::TrendLine => two(|start, end| AnnotationKind::TrendLine { start, end }),
        DrawingTool::Ray => two(|start, end| AnnotationKind::Ray { start, end }),
        DrawingTool::ExtendedLine => two(|start, end| AnnotationKind::ExtendedLine { start, end }),
        DrawingTool::Rectangle => two(|a, b| AnnotationKind::Rectangle { a, b }),
        DrawingTool::Measure => two(|start, end| AnnotationKind::Measure { start, end }),
        DrawingTool::FibRetracement => two(|a, b| AnnotationKind::Fib {
            kind: FibKind::Retracement,
            points: vec![a, b],
        }),
        DrawingTool::FibExtension => (anchors.len() >= 3).then(|| AnnotationKind::Fib {
            kind: FibKind::Extension,
            points: anchors[..3].to_vec(),
        }),
        DrawingTool::Pen | DrawingTool::Select | DrawingTool::Eraser => None,
    }
}
