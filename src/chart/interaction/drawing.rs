use super::super::{CandlestickChart, ChartState};
use crate::annotations::{
    Annotation, AnnotationKind, DEFAULT_LEVEL_COLOR, DEFAULT_LINE_COLOR, DrawingTool,
};
use crate::message::Message;
use iced::Point;
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
        if let Some((price_hi, price_range, price_h)) =
            self.visible_price_params(state, chart_w, chart_h)
        {
            let clamped_y = pos.y.clamp(0.0, price_h);
            let price = self.y_to_price_with(clamped_y, price_hi, price_range, price_h);
            let ts = self.x_to_timestamp(pos.x, state, chart_w).unwrap_or(0);

            match tool {
                DrawingTool::HorizontalLevel => {
                    let ann = Annotation {
                        id: 0,
                        kind: AnnotationKind::HorizontalLevel { price },
                        color: DEFAULT_LEVEL_COLOR,
                    };
                    return Some(
                        canvas::Action::publish(Message::AddAnnotation(self.id, ann)).and_capture(),
                    );
                }
                DrawingTool::TrendLine => {
                    if let Some((start_ts, start_price)) = state.pending_anchor {
                        let ann = Annotation {
                            id: 0,
                            kind: AnnotationKind::TrendLine {
                                start: (start_ts, start_price),
                                end: (ts, price),
                            },
                            color: DEFAULT_LINE_COLOR,
                        };
                        state.pending_anchor = None;
                        return Some(
                            canvas::Action::publish(Message::AddAnnotation(self.id, ann))
                                .and_capture(),
                        );
                    } else {
                        state.pending_anchor = Some((ts, price));
                        return Some(canvas::Action::capture());
                    }
                }
                DrawingTool::Eraser => {
                    if let Some(id) = self.hit_test_annotation(state, pos, chart_w, chart_h) {
                        return Some(
                            canvas::Action::publish(Message::RemoveAnnotation(self.id, id))
                                .and_capture(),
                        );
                    }
                    return Some(canvas::Action::capture());
                }
            }
        }
        Some(canvas::Action::capture())
    }
}
