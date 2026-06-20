use iced::Point;

use super::{PnlChartLayout, PnlChartPoint};

// ---------------------------------------------------------------------------
// PnL Layout
// ---------------------------------------------------------------------------

const FLAT_RANGE_EPSILON: f64 = 1e-9;
// Asymmetric headroom that always keeps the zero baseline in view (spec §4):
// 12% below the minimum, 16% above the maximum of the zero-inclusive range.
const Y_PADDING_BOTTOM_RATIO: f64 = 0.12;
const Y_PADDING_TOP_RATIO: f64 = 0.16;

pub(in crate::portfolio_state::charts::pnl) fn prepare_pnl_chart_layout(
    points: &[(u64, f64)],
    width: f32,
    height: f32,
) -> Option<PnlChartLayout> {
    if points.len() < 2 || width <= 0.0 || height <= 0.0 {
        return None;
    }

    let (min_ts, max_ts) = points.iter().fold((u64::MAX, 0_u64), |(lo, hi), (ts, _)| {
        (lo.min(*ts), hi.max(*ts))
    });
    if max_ts <= min_ts {
        return None;
    }

    let (min_pnl, max_pnl) = points
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), (_, pnl)| {
            (lo.min(*pnl), hi.max(*pnl))
        });
    let (lo, hi) = padded_pnl_range(min_pnl, max_pnl);
    let ts_span = (max_ts - min_ts) as f64;
    let pnl_span = hi - lo;

    let chart_points = points
        .iter()
        .map(|(timestamp_ms, pnl)| {
            let x = (((*timestamp_ms - min_ts) as f64 / ts_span) * f64::from(width)) as f32;
            let y = (((hi - *pnl) / pnl_span) * f64::from(height)) as f32;
            PnlChartPoint {
                point: Point::new(x, y),
                timestamp_ms: *timestamp_ms,
                pnl: *pnl,
            }
        })
        .collect();
    let zero_y = (((hi - 0.0) / pnl_span) * f64::from(height)) as f32;

    Some(PnlChartLayout {
        points: chart_points,
        zero_y: zero_y.clamp(0.0, height),
    })
}

pub(in crate::portfolio_state::charts::pnl) fn nearest_pnl_point(
    points: &[PnlChartPoint],
    cursor_x: f32,
) -> Option<PnlChartPoint> {
    points.iter().copied().min_by(|left, right| {
        let left_dist = (left.point.x - cursor_x).abs();
        let right_dist = (right.point.x - cursor_x).abs();
        left_dist.total_cmp(&right_dist)
    })
}

fn padded_pnl_range(min_pnl: f64, max_pnl: f64) -> (f64, f64) {
    // Always include the zero baseline so the dashed axis stays on screen.
    let mut lo = min_pnl.min(0.0);
    let mut hi = max_pnl.max(0.0);
    if (hi - lo).abs() < FLAT_RANGE_EPSILON {
        lo -= 1.0;
        hi += 1.0;
    }
    let range = hi - lo;
    (
        lo - range * Y_PADDING_BOTTOM_RATIO,
        hi + range * Y_PADDING_TOP_RATIO,
    )
}
