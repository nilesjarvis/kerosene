use crate::api::Candle;

use super::*;
use iced::Point;

mod bounds;
mod segments;
mod values;

fn candle_at(open_time: u64) -> Candle {
    Candle::test_flat(open_time, 1.0)
}

fn points_with_spacing(
    chart_candles: &[Candle],
    ma_series: &[(u64, f64)],
    first_vis: usize,
    last_vis: usize,
    chart_w: f32,
    candle_w: f32,
    spacing: f32,
) -> Vec<AveragePathPoint> {
    let idx_to_cx = |idx: usize| idx as f32 * spacing;
    let price_to_y = |value: f64| value as f32;
    visible_average_points(AveragePointContext {
        chart_candles,
        ma_series,
        first_vis,
        last_vis,
        chart_w,
        candle_w,
        idx_to_cx: &idx_to_cx,
        price_to_y: &price_to_y,
    })
}
