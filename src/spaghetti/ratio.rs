mod axes;
mod crosshair;
mod series;

use self::axes::{draw_ratio_base_line, draw_ratio_grid, draw_ratio_time_axis};
use self::crosshair::draw_ratio_crosshair;
use self::series::{draw_ratio_candles, draw_ratio_line};
use super::helpers::has_positive_finite_prices;
use super::{PRICE_PADDING_PCT, Series, SpaghettiCanvas, SpaghettiChartState};
use crate::api::Candle;
use crate::chart_background::draw_dotted_background;
use iced::alignment;
use iced::widget::canvas;
use iced::{Point, Rectangle, Renderer, Theme};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Pair Ratio Rendering
// ---------------------------------------------------------------------------

pub(super) struct PairRatioRenderContext<'a> {
    pub(super) state: &'a SpaghettiChartState,
    pub(super) renderer: &'a Renderer,
    pub(super) theme: &'a Theme,
    pub(super) bounds: Rectangle,
    pub(super) chart_w: f32,
    pub(super) chart_h: f32,
    pub(super) left_ts: f64,
    pub(super) right_ts: f64,
    pub(super) visible_ms: f64,
    pub(super) time_px_per_ms: f64,
    pub(super) effective_max: u64,
    pub(super) base_timestamp: Option<u64>,
    pub(super) crosshair_style: crate::config::ChartCrosshairStyle,
    pub(super) crosshair_guides_enabled: bool,
    pub(super) crosshair_scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RatioCandle {
    pub(super) x: f32,
    pub(super) open: f64,
    pub(super) high: f64,
    pub(super) low: f64,
    pub(super) close: f64,
}

impl SpaghettiCanvas {
    pub(super) fn draw_pair_ratio(
        &self,
        ctx: PairRatioRenderContext<'_>,
        loaded_series: &[&Series],
    ) -> Vec<canvas::Geometry> {
        let series_a = loaded_series[0];
        let series_b = loaded_series[1];
        let ts_to_x = |ts: u64| -> f32 { ((ts as f64 - ctx.left_ts) * ctx.time_px_per_ms) as f32 };

        let ratio_candles = build_ratio_candles(
            &series_a.candles,
            &series_b.candles,
            ctx.left_ts,
            ctx.right_ts,
            &ts_to_x,
        );

        if ratio_candles.is_empty() {
            return vec![];
        }

        let (auto_lo, auto_hi) = if self.pair_candle_mode {
            ratio_candles
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), candle| {
                    (lo.min(candle.low), hi.max(candle.high))
                })
        } else {
            ratio_candles
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), candle| {
                    (lo.min(candle.close), hi.max(candle.close))
                })
        };
        let min_range = minimum_ratio_range(auto_lo, auto_hi);
        let pad = (auto_hi - auto_lo).max(min_range) * PRICE_PADDING_PCT;
        let auto_lo = auto_lo - pad;
        let auto_hi = auto_hi + pad;

        let (ratio_lo, ratio_hi) = if ctx.state.y_auto {
            (auto_lo, auto_hi)
        } else {
            let range = (auto_hi - auto_lo) * ctx.state.y_scale;
            let mid = (auto_hi + auto_lo) * 0.5 + ctx.state.y_offset;
            (mid - range * 0.5, mid + range * 0.5)
        };
        let ratio_range = (ratio_hi - ratio_lo).max(minimum_ratio_range(ratio_lo, ratio_hi));
        let ratio_to_y =
            |ratio: f64| -> f32 { ((ratio_hi - ratio) / ratio_range * ctx.chart_h as f64) as f32 };

        let mut frame = canvas::Frame::new(ctx.renderer, ctx.bounds.size());
        frame.fill_rectangle(Point::ORIGIN, ctx.bounds.size(), iced::Color::TRANSPARENT);

        if self.dotted_background {
            draw_dotted_background(
                &mut frame,
                ctx.theme,
                ctx.chart_w,
                ctx.chart_h,
                self.dotted_background_opacity,
                crate::chart::fisheye::ChartFisheye::disabled(),
            );
        }
        draw_ratio_grid(
            &mut frame,
            &ctx,
            ratio_hi,
            ratio_range,
            !self.dotted_background,
        );
        draw_ratio_time_axis(&mut frame, &ctx);
        draw_ratio_base_line(&mut frame, &ctx, &ts_to_x);

        if self.pair_candle_mode {
            draw_ratio_candles(
                &mut frame,
                &ctx,
                &ratio_candles,
                &ratio_to_y,
                ctx.theme,
                self.hollow_candle_mode,
            );
        } else {
            draw_ratio_line(
                &mut frame,
                &ctx,
                &ratio_candles,
                &ratio_to_y,
                ctx.theme.palette().primary,
            );
        }

        if let Some(last) = ratio_candles.last() {
            frame.fill_text(canvas::Text {
                content: format!(
                    "{} / {}  {}",
                    series_a.display,
                    series_b.display,
                    format_ratio_value(last.close)
                ),
                position: Point::new(8.0, 12.0),
                color: ctx.theme.palette().primary,
                size: iced::Pixels(11.0),
                align_x: alignment::Horizontal::Left.into(),
                align_y: alignment::Vertical::Center,
                font: crate::app_fonts::monospace_font(),
                ..canvas::Text::default()
            });
        }

        let base_geo = frame.into_geometry();
        let overlay = draw_ratio_crosshair(&ctx, ratio_hi, ratio_range);
        vec![base_geo, overlay]
    }
}

pub(super) fn build_ratio_candles(
    series_a: &[Candle],
    series_b: &[Candle],
    left_ts: f64,
    right_ts: f64,
    ts_to_x: &impl Fn(u64) -> f32,
) -> Vec<RatioCandle> {
    let mut b_by_ts: HashMap<u64, &Candle> = HashMap::new();
    for candle in series_b {
        if has_positive_finite_prices(candle) {
            b_by_ts.insert(candle.open_time, candle);
        }
    }

    series_a
        .iter()
        .filter(|candle| {
            (candle.open_time as f64) >= left_ts && (candle.open_time as f64) <= right_ts
        })
        .filter_map(|candle| {
            if !has_positive_finite_prices(candle) {
                return None;
            }

            let b = b_by_ts.get(&candle.open_time).copied()?;
            let ratio = RatioCandle {
                x: ts_to_x(candle.open_time),
                open: candle.open / b.open,
                high: candle.high / b.low,
                low: candle.low / b.high,
                close: candle.close / b.close,
            };

            (ratio.open.is_finite()
                && ratio.high.is_finite()
                && ratio.low.is_finite()
                && ratio.close.is_finite()
                && ratio.high >= ratio.low)
                .then_some(ratio)
        })
        .collect()
}

pub(super) fn format_ratio_value(value: f64) -> String {
    if !value.is_finite() {
        return "--".to_string();
    }

    let abs = value.abs();
    if abs >= 1_000.0 {
        format!("{value:.0}")
    } else if abs >= 1.0 {
        format!("{value:.4}")
    } else if abs >= 0.01 {
        format!("{value:.5}")
    } else if abs >= 0.0001 {
        format!("{value:.6}")
    } else {
        format!("{value:.8}")
    }
}

fn minimum_ratio_range(lo: f64, hi: f64) -> f64 {
    let magnitude = ((lo + hi) * 0.5).abs().max(lo.abs()).max(hi.abs());
    (magnitude * 0.02).max(f64::EPSILON)
}

#[cfg(test)]
mod tests;
