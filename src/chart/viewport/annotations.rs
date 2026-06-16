use super::super::geometry::{LineExtension, extend_and_clip_line, point_to_segment_dist};
use crate::annotations::{
    ANNOTATION_HIT_TOLERANCE, Anchor, AnnotationId, AnnotationKind, FIB_EXTENSION_LEVELS,
    FIB_RETRACEMENT_LEVELS, FibKind, fib_extension_price, fib_retracement_price,
};
use crate::chart::{CandlestickChart, ChartState};
use iced::Point;

// ---------------------------------------------------------------------------
// Annotation Hit Testing
// ---------------------------------------------------------------------------

/// Which part of an annotation a click landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::chart) enum AnnotationHitPart {
    /// The body of the shape (drag moves the whole annotation).
    Body,
    /// A specific draggable anchor handle (drag moves just that point).
    Anchor(usize),
}

#[derive(Debug, Clone, Copy)]
pub(in crate::chart) struct AnnotationHit {
    pub(in crate::chart) id: AnnotationId,
    pub(in crate::chart) part: AnnotationHitPart,
}

impl CandlestickChart {
    /// Hit-test annotations against a click position. Anchors take priority over
    /// bodies, and later (topmost) annotations take priority over earlier ones.
    /// Hidden and locked annotations are not hittable.
    pub(in crate::chart) fn hit_test_annotation(
        &self,
        state: &ChartState,
        pos: Point,
        chart_w: f32,
        chart_h: f32,
    ) -> Option<AnnotationHit> {
        let (price_hi, price_range, price_h) =
            self.visible_price_params(state, chart_w, chart_h)?;
        let price_to_y =
            |price: f64| -> f32 { self.price_to_y_with(price, price_hi, price_range, price_h) };
        let tx = |ts: u64| -> Option<f32> { self.timestamp_to_x(ts, state, chart_w) };
        let tol = ANNOTATION_HIT_TOLERANCE;

        let seg_hit = |s: Anchor, e: Anchor, ext: LineExtension| -> bool {
            let (Some(x1), Some(x2)) = (tx(s.0), tx(e.0)) else {
                return false;
            };
            let y1 = price_to_y(s.1);
            let y2 = price_to_y(e.1);
            extend_and_clip_line(x1, y1, x2, y2, chart_w, price_h, ext).is_some_and(
                |(cx1, cy1, cx2, cy2)| {
                    point_to_segment_dist(pos.x, pos.y, cx1, cy1, cx2, cy2) <= tol
                },
            )
        };

        let rect_hit = |a: Anchor, b: Anchor| -> bool {
            let (Some(xa), Some(xb)) = (tx(a.0), tx(b.0)) else {
                return false;
            };
            let ya = price_to_y(a.1);
            let yb = price_to_y(b.1);
            let (x0, x1) = (xa.min(xb), xa.max(xb));
            let (y0, y1) = (ya.min(yb), ya.max(yb));
            pos.x >= x0 - tol && pos.x <= x1 + tol && pos.y >= y0 - tol && pos.y <= y1 + tol
        };

        let fib_hit = |kind: FibKind, points: &[Anchor]| -> bool {
            let expected = match kind {
                FibKind::Retracement => 2,
                FibKind::Extension => 3,
            };
            if points.len() != expected {
                return false;
            }
            let x_left = points
                .iter()
                .filter_map(|p| tx(p.0))
                .fold(f32::INFINITY, f32::min)
                .max(0.0);
            if !x_left.is_finite() || pos.x < x_left - tol {
                return false;
            }
            let levels: &[f64] = match kind {
                FibKind::Retracement => FIB_RETRACEMENT_LEVELS,
                FibKind::Extension => FIB_EXTENSION_LEVELS,
            };
            levels.iter().any(|&ratio| {
                let price = match kind {
                    FibKind::Retracement => fib_retracement_price(points[0], points[1], ratio),
                    FibKind::Extension => {
                        fib_extension_price(points[0], points[1], points[2], ratio)
                    }
                };
                (pos.y - price_to_y(price)).abs() <= tol
            })
        };

        for ann in self.annotations.iter().rev() {
            // Locked annotations stay hittable so they can be selected and
            // unlocked; callers (select drag, eraser) gate edits on the lock.
            if !ann.style.visible {
                continue;
            }

            // Anchor handles take priority so endpoints stay grabbable.
            for (index, (ts, price)) in ann.kind.anchor_points().into_iter().enumerate() {
                if let Some(x) = tx(ts) {
                    let y = price_to_y(price);
                    if ((pos.x - x).powi(2) + (pos.y - y).powi(2)).sqrt() <= tol + 2.0 {
                        return Some(AnnotationHit {
                            id: ann.id,
                            part: AnnotationHitPart::Anchor(index),
                        });
                    }
                }
            }

            let body = match &ann.kind {
                AnnotationKind::HorizontalLevel { price } => {
                    pos.x >= 0.0 && pos.x <= chart_w && (pos.y - price_to_y(*price)).abs() <= tol
                }
                AnnotationKind::TrendLine { start, end } => {
                    seg_hit(*start, *end, LineExtension::Segment)
                }
                AnnotationKind::Ray { start, end } => seg_hit(*start, *end, LineExtension::Forward),
                AnnotationKind::ExtendedLine { start, end } => {
                    seg_hit(*start, *end, LineExtension::Both)
                }
                AnnotationKind::Measure { start, end } => {
                    seg_hit(*start, *end, LineExtension::Segment)
                }
                AnnotationKind::VerticalLine { time } => tx(*time)
                    .is_some_and(|x| pos.y >= 0.0 && pos.y <= price_h && (pos.x - x).abs() <= tol),
                AnnotationKind::Rectangle { a, b } => rect_hit(*a, *b),
                AnnotationKind::Fib { kind, points } => fib_hit(*kind, points),
            };
            if body {
                return Some(AnnotationHit {
                    id: ann.id,
                    part: AnnotationHitPart::Body,
                });
            }
        }
        None
    }
}
