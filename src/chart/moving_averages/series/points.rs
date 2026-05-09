use crate::api::Candle;

use iced::Point;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Moving Average Point Selection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct AveragePathPoint {
    pub(super) point: Point,
    pub(super) starts_segment: bool,
}

pub(super) struct AveragePointContext<'a, X, Y>
where
    X: Fn(usize) -> f32,
    Y: Fn(f64) -> f32,
{
    pub(super) chart_candles: &'a [Candle],
    pub(super) ma_series: &'a [(u64, f64)],
    pub(super) first_vis: usize,
    pub(super) last_vis: usize,
    pub(super) chart_w: f32,
    pub(super) candle_w: f32,
    pub(super) idx_to_cx: &'a X,
    pub(super) price_to_y: &'a Y,
}

pub(super) fn visible_average_points<X, Y>(
    ctx: AveragePointContext<'_, X, Y>,
) -> Vec<AveragePathPoint>
where
    X: Fn(usize) -> f32,
    Y: Fn(f64) -> f32,
{
    if ctx.ma_series.is_empty() || ctx.chart_candles.is_empty() || ctx.first_vis > ctx.last_vis {
        return Vec::new();
    }

    let first_ts = ctx.chart_candles[ctx.first_vis].open_time;
    let mut last_ma_idx = match ctx.ma_series.binary_search_by_key(&first_ts, |&(ts, _)| ts) {
        Ok(idx) => idx,
        Err(idx) => idx.saturating_sub(1),
    };
    let mut starts_segment = true;
    let mut points = Vec::new();

    for (relative_index, candle) in ctx.chart_candles[ctx.first_vis..=ctx.last_vis]
        .iter()
        .enumerate()
    {
        let i = ctx.first_vis + relative_index;
        let cx = (ctx.idx_to_cx)(i);
        if cx + ctx.candle_w * 0.5 < 0.0 || cx - ctx.candle_w * 0.5 > ctx.chart_w {
            continue;
        }

        while last_ma_idx + 1 < ctx.ma_series.len()
            && ctx.ma_series[last_ma_idx + 1].0 <= candle.open_time
        {
            last_ma_idx += 1;
        }

        if ctx.ma_series[last_ma_idx].0 <= candle.open_time {
            let (_, val) = ctx.ma_series[last_ma_idx];
            points.push(AveragePathPoint {
                point: Point::new(cx, (ctx.price_to_y)(val)),
                starts_segment,
            });
            starts_segment = false;
        } else {
            starts_segment = true;
        }
    }

    points
}
