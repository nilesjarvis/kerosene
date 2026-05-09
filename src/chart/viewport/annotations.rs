use super::super::geometry::point_to_segment_dist;
use crate::annotations::{ANNOTATION_HIT_TOLERANCE, AnnotationId, AnnotationKind};
use crate::chart::{CandlestickChart, ChartState};
use iced::Point;

// ---------------------------------------------------------------------------
// Annotation Hit Testing
// ---------------------------------------------------------------------------

impl CandlestickChart {
    /// Hit-test annotations against a click position.
    /// Returns the id of the nearest annotation within tolerance, or None.
    pub(in crate::chart) fn hit_test_annotation(
        &self,
        state: &ChartState,
        pos: Point,
        chart_w: f32,
        chart_h: f32,
    ) -> Option<AnnotationId> {
        let (price_hi, price_range, price_h) =
            self.visible_price_params(state, chart_w, chart_h)?;
        let price_to_y =
            |price: f64| -> f32 { self.price_to_y_with(price, price_hi, price_range, price_h) };

        for ann in &self.annotations {
            match &ann.kind {
                AnnotationKind::HorizontalLevel { price } => {
                    let y = price_to_y(*price);
                    if (pos.y - y).abs() < ANNOTATION_HIT_TOLERANCE {
                        return Some(ann.id);
                    }
                }
                AnnotationKind::TrendLine { start, end } => {
                    let Some(x1) = self.timestamp_to_x(start.0, state, chart_w) else {
                        continue;
                    };
                    let y1 = price_to_y(start.1);
                    let Some(x2) = self.timestamp_to_x(end.0, state, chart_w) else {
                        continue;
                    };
                    let y2 = price_to_y(end.1);
                    let dist = point_to_segment_dist(pos.x, pos.y, x1, y1, x2, y2);
                    if dist < ANNOTATION_HIT_TOLERANCE {
                        return Some(ann.id);
                    }
                }
            }
        }
        None
    }
}
