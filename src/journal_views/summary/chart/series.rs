use super::drawing::JournalSummaryChart;
use crate::helpers::finite_value;

use iced::Point;

mod account_value;
mod pnl;

pub(super) use account_value::account_value_points_for_range;
pub(super) use pnl::journal_cumulative_pnl_points;

const FLAT_RANGE_EPSILON: f64 = 1e-9;
const Y_PADDING_RATIO: f64 = 0.10;

// ---------------------------------------------------------------------------
// Chart Series
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(super) struct ChartPoint {
    pub(super) point: Point,
    pub(super) timestamp_ms: u64,
    pub(super) value: f64,
}

#[derive(Debug, Clone)]
pub(super) struct ChartLayout {
    pub(super) pnl_points: Vec<ChartPoint>,
    pub(super) account_value_points: Vec<ChartPoint>,
    pub(super) zero_y: f32,
}

pub(super) fn prepare_chart_layout(
    chart: &JournalSummaryChart,
    width: f32,
    height: f32,
) -> Option<ChartLayout> {
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let pnl_points = finite_sorted_points(&chart.pnl_points);
    if pnl_points.len() < 2 {
        return None;
    }

    let (min_ts, max_ts) = chart_time_range(&pnl_points)?;
    if max_ts <= min_ts {
        return None;
    }

    let (pnl_lo, pnl_hi) = padded_value_range(&pnl_points, true)?;
    let pnl_plot_points =
        map_chart_points(&pnl_points, min_ts, max_ts, pnl_lo, pnl_hi, width, height);
    let zero_y = value_y(0.0, pnl_lo, pnl_hi, height).clamp(0.0, height);

    let account_value_points = if chart.show_account_value {
        let account_points = finite_sorted_points(&chart.account_value_points)
            .into_iter()
            .filter(|(timestamp_ms, _)| *timestamp_ms >= min_ts && *timestamp_ms <= max_ts)
            .collect::<Vec<_>>();
        if let Some((account_lo, account_hi)) = padded_value_range(&account_points, false) {
            map_chart_points(
                &account_points,
                min_ts,
                max_ts,
                account_lo,
                account_hi,
                width,
                height,
            )
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    Some(ChartLayout {
        pnl_points: pnl_plot_points,
        account_value_points,
        zero_y,
    })
}

pub(super) fn finite_sorted_points(points: &[(u64, f64)]) -> Vec<(u64, f64)> {
    let mut sorted = points
        .iter()
        .filter_map(|(timestamp_ms, value)| {
            finite_value(*value).map(|value| (*timestamp_ms, value))
        })
        .collect::<Vec<_>>();
    sorted.sort_by_key(|(timestamp_ms, _)| *timestamp_ms);
    sorted
}

pub(super) fn chart_time_range(points: &[(u64, f64)]) -> Option<(u64, u64)> {
    let start = points.first().map(|(timestamp_ms, _)| *timestamp_ms)?;
    let end = points.last().map(|(timestamp_ms, _)| *timestamp_ms)?;
    Some((start, end))
}

fn padded_value_range(points: &[(u64, f64)], include_zero: bool) -> Option<(f64, f64)> {
    let (mut lo, mut hi) = points.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(lo, hi), (_, value)| (lo.min(*value), hi.max(*value)),
    );
    if !lo.is_finite() || !hi.is_finite() {
        return None;
    }

    if include_zero {
        lo = lo.min(0.0);
        hi = hi.max(0.0);
    }
    if (hi - lo).abs() < FLAT_RANGE_EPSILON {
        let pad = hi.abs().max(1.0) * 0.05;
        lo -= pad;
        hi += pad;
    }

    let pad = (hi - lo) * Y_PADDING_RATIO;
    Some((lo - pad, hi + pad))
}

fn map_chart_points(
    points: &[(u64, f64)],
    min_ts: u64,
    max_ts: u64,
    value_lo: f64,
    value_hi: f64,
    width: f32,
    height: f32,
) -> Vec<ChartPoint> {
    let ts_span = (max_ts - min_ts) as f64;
    points
        .iter()
        .map(|(timestamp_ms, value)| {
            let x = (((*timestamp_ms - min_ts) as f64 / ts_span) * f64::from(width)) as f32;
            ChartPoint {
                point: Point::new(x, value_y(*value, value_lo, value_hi, height)),
                timestamp_ms: *timestamp_ms,
                value: *value,
            }
        })
        .collect()
}

fn value_y(value: f64, value_lo: f64, value_hi: f64, height: f32) -> f32 {
    (((value_hi - value) / (value_hi - value_lo)) * f64::from(height)) as f32
}
