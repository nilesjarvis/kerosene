use super::CandleLayerContext;
use crate::api::Candle;
use crate::chart::CandlestickChart;
use crate::chart::indicators::calculate_ema;
use crate::config::EmaCloudTimeframe;
use iced::widget::canvas;
use iced::{Color, Point, Rectangle};

impl CandlestickChart {
    pub(super) fn draw_ema_clouds<X, Y>(
        &self,
        ctx: &CandleLayerContext<'_, X, Y>,
        frame: &mut canvas::Frame,
    ) where
        X: Fn(usize) -> f32,
        Y: Fn(f64) -> f32,
    {
        frame.with_clip(
            Rectangle::new(Point::ORIGIN, iced::Size::new(ctx.chart_w, ctx.price_h)),
            |frame| {
                for cloud in &self.macro_indicators.ema_clouds {
                    if !cloud.enabled || !cloud.is_valid() || cloud.opacity == 0 {
                        continue;
                    }
                    let source = match cloud.timeframe {
                        EmaCloudTimeframe::Chart => &self.candles,
                        EmaCloudTimeframe::Hour => &self.hourly_candles,
                        EmaCloudTimeframe::Day => &self.daily_candles,
                        EmaCloudTimeframe::Week => &self.weekly_candles,
                        EmaCloudTimeframe::Month => &self.monthly_candles,
                    };
                    let series = ema_cloud_series(source, cloud.fast_period, cloud.slow_period);
                    let points: Vec<_> =
                        visible_cloud_samples(&self.candles, &series, ctx.first_vis, ctx.last_vis)
                            .into_iter()
                            .map(|(index, fast, slow)| CloudEdge {
                                fast: Point::new((ctx.idx_to_cx)(index), (ctx.price_to_y)(fast)),
                                slow: Point::new((ctx.idx_to_cx)(index), (ctx.price_to_y)(slow)),
                                bullish: fast >= slow,
                            })
                            .collect();
                    for band in cloud_bands(&points) {
                        ctx.fisheye.fill_projected_polygon_without_edge_blur(
                            frame,
                            &band.points,
                            Color {
                                a: f32::from(cloud.opacity) / 100.0,
                                ..cloud.color.color(ctx.theme, band.bullish)
                            },
                        );
                    }
                }
            },
        );
    }
}

/// Both EMAs must be seeded before the cloud starts. Align by candle, not by
/// vector offset: different periods have different first sample timestamps.
fn ema_cloud_series(candles: &[Candle], fast: usize, slow: usize) -> Vec<(u64, f64, f64)> {
    let fast = calculate_ema(candles, fast);
    let slow = calculate_ema(candles, slow);
    let common = fast.len().min(slow.len());
    fast[fast.len() - common..]
        .iter()
        .zip(&slow[slow.len() - common..])
        .map(|(&(time, fast), &(_, slow))| (time, fast, slow))
        .collect()
}

/// Sample the latest source value at or before each displayed candle, just as
/// the EMA lines do. Include one neighbor on either side for viewport clipping.
fn visible_cloud_samples(
    candles: &[Candle],
    series: &[(u64, f64, f64)],
    first: usize,
    last: usize,
) -> Vec<(usize, f64, f64)> {
    if candles.is_empty() || series.is_empty() || first > last || first >= candles.len() {
        return Vec::new();
    }
    let first = first.saturating_sub(1);
    let last = last.saturating_add(1).min(candles.len() - 1);
    let mut source_index = series
        .partition_point(|sample| sample.0 <= candles[first].open_time)
        .saturating_sub(1);
    let mut samples = Vec::with_capacity(last - first + 1);
    for (index, candle) in candles.iter().enumerate().take(last + 1).skip(first) {
        while source_index + 1 < series.len() && series[source_index + 1].0 <= candle.open_time {
            source_index += 1;
        }
        let (time, fast, slow) = series[source_index];
        if time <= candle.open_time {
            samples.push((index, fast, slow));
        }
    }
    samples
}

#[derive(Clone, Copy)]
struct CloudEdge {
    fast: Point,
    slow: Point,
    bullish: bool,
}

struct CloudBand {
    points: Vec<Point>,
    bullish: bool,
}

/// Split at every crossing so polygons never self-intersect, even on an
/// inverted price axis. Contiguous runs avoid translucent seams between bars.
fn cloud_bands(edges: &[CloudEdge]) -> Vec<CloudBand> {
    let mut bands = Vec::new();
    let mut fast = Vec::new();
    let mut slow = Vec::new();
    let mut previous: Option<CloudEdge> = None;
    let finish =
        |fast: &mut Vec<Point>, slow: &mut Vec<Point>, bullish, bands: &mut Vec<CloudBand>| {
            if fast.len() >= 2 {
                fast.extend(slow.drain(..).rev());
                bands.push(CloudBand {
                    points: std::mem::take(fast),
                    bullish,
                });
            } else {
                fast.clear();
                slow.clear();
            }
        };
    for &edge in edges {
        if ![edge.fast.x, edge.fast.y, edge.slow.x, edge.slow.y]
            .into_iter()
            .all(f32::is_finite)
        {
            if let Some(prior) = previous.take() {
                finish(&mut fast, &mut slow, prior.bullish, &mut bands);
            }
            continue;
        }
        if let Some(prior) = previous
            && prior.bullish != edge.bullish
        {
            let before = prior.fast.y - prior.slow.y;
            let after = edge.fast.y - edge.slow.y;
            let t = if before == after {
                0.0
            } else {
                (before / (before - after)).clamp(0.0, 1.0)
            };
            let crossing = Point::new(
                prior.fast.x + (edge.fast.x - prior.fast.x) * t,
                prior.fast.y + (edge.fast.y - prior.fast.y) * t,
            );
            fast.push(crossing);
            slow.push(crossing);
            finish(&mut fast, &mut slow, prior.bullish, &mut bands);
            fast.push(crossing);
            slow.push(crossing);
        }
        fast.push(edge.fast);
        slow.push(edge.slow);
        previous = Some(edge);
    }
    if let Some(prior) = previous {
        finish(&mut fast, &mut slow, prior.bullish, &mut bands);
    }
    bands
}

#[cfg(test)]
mod tests;
