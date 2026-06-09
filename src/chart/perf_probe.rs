use super::{CANDLE_GAP_RATIO, CandlestickChart, ChartState, VOLUME_REGION_RATIO};
use iced::Rectangle;
use std::hint::black_box;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChartHotPathSample {
    pub(crate) visible_candles: usize,
    pub(crate) checksum: f64,
}

pub(crate) fn set_market_load_view(state: &mut ChartState, candle_width: f32, scroll_offset: f32) {
    state.candle_width = candle_width.max(2.0);
    state.scroll_offset = scroll_offset.max(0.0);
    state.y_auto = true;
    state.y_offset = 0.0;
    state.y_scale = 1.0;
    state.funding_y_offset = 0.0;
    state.funding_y_scale = 1.0;
}

pub(crate) fn chart_layout_hot_path(
    chart: &CandlestickChart,
    state: &ChartState,
    bounds: Rectangle,
) -> ChartHotPathSample {
    if chart.candles.is_empty() {
        return black_box(ChartHotPathSample {
            visible_candles: 0,
            checksum: 0.0,
        });
    }

    let chart_w = bounds.width - chart.price_axis_width();
    let (chart_h, funding_panel_h) = chart.chart_area_heights(bounds.height);
    if chart_w <= 0.0
        || chart_h <= 0.0
        || !chart_w.is_finite()
        || !chart_h.is_finite()
        || !bounds.width.is_finite()
        || !bounds.height.is_finite()
    {
        return black_box(ChartHotPathSample {
            visible_candles: 0,
            checksum: 0.0,
        });
    }

    let volume_h = chart_h * VOLUME_REGION_RATIO;
    let price_h = chart_h - volume_h;
    let candle_w = state.candle_width;
    let step = candle_w * (1.0 + CANDLE_GAP_RATIO);
    let Some(visible_range) = chart.visible_candle_range(state, chart_w) else {
        return black_box(ChartHotPathSample {
            visible_candles: 0,
            checksum: 0.0,
        });
    };
    let Some(price_stats) =
        chart.visible_price_stats_for_state(state, visible_range.first, visible_range.last)
    else {
        return black_box(ChartHotPathSample {
            visible_candles: 0,
            checksum: 0.0,
        });
    };

    let mut checksum = price_stats.price_lo
        + price_stats.price_hi
        + price_stats.price_range
        + price_stats.volume_max
        + funding_panel_h as f64;
    for idx in visible_range.first..=visible_range.last {
        let candle = &chart.candles[idx];
        let slots_from_right = visible_range.right_idx - idx as isize;
        let x = chart_w - (slots_from_right as f32) * step - step * 0.5;
        let high_y = chart.price_to_y_with(
            candle.high,
            price_stats.price_hi,
            price_stats.price_range,
            price_h,
        );
        let low_y = chart.price_to_y_with(
            candle.low,
            price_stats.price_hi,
            price_stats.price_range,
            price_h,
        );
        let close_y = chart.price_to_y_with(
            candle.close,
            price_stats.price_hi,
            price_stats.price_range,
            price_h,
        );
        checksum += x as f64 * 0.000_001
            + high_y as f64 * 0.000_01
            + low_y as f64 * 0.000_01
            + close_y as f64 * 0.000_01
            + candle.volume * 0.000_000_001;
    }

    black_box(ChartHotPathSample {
        visible_candles: visible_range.last - visible_range.first + 1,
        checksum,
    })
}
