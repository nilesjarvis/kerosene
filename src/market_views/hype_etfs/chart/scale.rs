use crate::helpers::{finite_value, positive_finite_value};
use crate::hype_etf_state::HypeEtfDailyFlow;

use iced::Point;

// ---------------------------------------------------------------------------
// Daily Flow Chart Scale
// ---------------------------------------------------------------------------

pub(super) const FLOW_BAR_SPACING: u32 = 3;
pub(in crate::market_views::hype_etfs) const FLOW_CHART_HEIGHT: f32 = 126.0;
pub(super) const FLOW_AXIS_HEIGHT: f32 = 1.0;
const FLOW_CHART_VERTICAL_PADDING: f32 = 8.0;

pub(in crate::market_views::hype_etfs) fn cumulative_inflows(
    flows: &[HypeEtfDailyFlow],
) -> Vec<f64> {
    let mut total = 0.0;
    flows
        .iter()
        .map(|flow| {
            total += flow.amount_usd;
            total
        })
        .collect()
}

#[derive(Clone, Copy)]
pub(in crate::market_views::hype_etfs) struct FlowChartScale {
    pub(in crate::market_views::hype_etfs) zero_y: f32,
    pub(in crate::market_views::hype_etfs) positive_height: f32,
    pub(in crate::market_views::hype_etfs) negative_height: f32,
    max_positive: f64,
    max_negative: f64,
    pub(in crate::market_views::hype_etfs) top_padding: f32,
    pub(in crate::market_views::hype_etfs) bottom_padding: f32,
}

pub(in crate::market_views::hype_etfs) fn flow_chart_scale(
    values: &[f64],
    height: f32,
) -> FlowChartScale {
    let max_positive = values
        .iter()
        .copied()
        .filter_map(positive_finite_value)
        .fold(0.0_f64, f64::max);
    let max_negative = values
        .iter()
        .copied()
        .filter_map(finite_value)
        .filter(|value| *value < 0.0)
        .map(f64::abs)
        .fold(0.0_f64, f64::max);
    let top_padding = FLOW_CHART_VERTICAL_PADDING.min(height * 0.4);
    let bottom_padding = FLOW_CHART_VERTICAL_PADDING.min(height * 0.4);
    let usable_height = (height - top_padding - bottom_padding - FLOW_AXIS_HEIGHT).max(1.0);

    let (positive_height, negative_height) = match (max_positive > 0.0, max_negative > 0.0) {
        (true, true) => {
            let positive_share = max_positive / (max_positive + max_negative);
            let positive_height = (usable_height * positive_share as f32).clamp(1.0, usable_height);
            (positive_height, usable_height - positive_height)
        }
        (true, false) => (usable_height, 0.0),
        (false, true) => (0.0, usable_height),
        (false, false) => (usable_height * 0.5, usable_height * 0.5),
    };

    FlowChartScale {
        zero_y: top_padding + positive_height,
        positive_height,
        negative_height,
        max_positive: max_positive.max(1.0),
        max_negative: max_negative.max(1.0),
        top_padding,
        bottom_padding,
    }
}

pub(in crate::market_views::hype_etfs) fn flow_bar_layout(
    value: f64,
    scale: FlowChartScale,
) -> (f32, f32, f32, f32) {
    let value = finite_value(value).unwrap_or(0.0);
    let min_visible_height = 2.0_f32.min(scale.positive_height.max(scale.negative_height));
    let positive_height = if value > 0.0 {
        ((value / scale.max_positive) as f32 * scale.positive_height)
            .max(min_visible_height)
            .min(scale.positive_height)
    } else {
        0.0
    };
    let negative_height = if value < 0.0 {
        ((value.abs() / scale.max_negative) as f32 * scale.negative_height)
            .max(min_visible_height)
            .min(scale.negative_height)
    } else {
        0.0
    };
    let axis_bottom = scale.zero_y + FLOW_AXIS_HEIGHT;
    let top_spacer = scale.top_padding + (scale.positive_height - positive_height).max(0.0);
    let bottom_spacer = scale.bottom_padding + (scale.negative_height - negative_height).max(0.0);

    debug_assert!((top_spacer + positive_height - scale.zero_y).abs() < 0.5);
    debug_assert!((axis_bottom + negative_height + bottom_spacer - FLOW_CHART_HEIGHT).abs() < 0.5);

    (top_spacer, positive_height, negative_height, bottom_spacer)
}

pub(in crate::market_views::hype_etfs) fn cumulative_line_points(
    values: &[f64],
    width: f32,
    height: f32,
    scale: FlowChartScale,
) -> Vec<Point> {
    if values.is_empty() || width <= 0.0 || height <= 0.0 {
        return Vec::new();
    }

    let line_scale = flow_chart_scale(values, height);
    let count = values.len();
    let spacing = FLOW_BAR_SPACING as f32;
    let available_width = (width - spacing * count.saturating_sub(1) as f32).max(count as f32);
    let bar_width = available_width / count as f32;

    values
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(idx, value)| finite_value(value).map(|value| (idx, value)))
        .map(|(idx, value)| {
            let x = bar_width * 0.5 + idx as f32 * (bar_width + spacing);
            let y = if value >= 0.0 {
                scale.zero_y - (value / line_scale.max_positive) as f32 * scale.positive_height
            } else {
                scale.zero_y
                    + FLOW_AXIS_HEIGHT
                    + (value.abs() / line_scale.max_negative) as f32 * scale.negative_height
            };
            Point::new(x.clamp(0.0, width), y.clamp(0.0, height))
        })
        .collect()
}
