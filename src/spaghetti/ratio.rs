mod axes;
mod crosshair;
mod series;

use self::axes::{draw_ratio_base_line, draw_ratio_grid, draw_ratio_time_axis};
use self::crosshair::draw_ratio_crosshair;
use self::series::{draw_ratio_candles, draw_ratio_line};
use super::{PRICE_PADDING_PCT, Series, SpaghettiCanvas, SpaghettiChartState};
use crate::api::Candle;
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

        let mut b_by_ts: HashMap<u64, &Candle> = HashMap::new();
        for candle in &series_b.candles {
            if candle.open > 0.0 && candle.high > 0.0 && candle.low > 0.0 && candle.close > 0.0 {
                b_by_ts.insert(candle.open_time, candle);
            }
        }

        let ratio_candles: Vec<(f32, f64, f64, f64, f64)> = series_a
            .candles
            .iter()
            .filter(|candle| {
                (candle.open_time as f64) >= ctx.left_ts
                    && (candle.open_time as f64) <= ctx.right_ts
            })
            .filter_map(|candle| {
                let b = b_by_ts.get(&candle.open_time).copied()?;
                if candle.open <= 0.0
                    || candle.high <= 0.0
                    || candle.low <= 0.0
                    || candle.close <= 0.0
                {
                    return None;
                }
                let open = candle.open / b.open;
                let high = candle.high / b.high;
                let low = candle.low / b.low;
                let close = candle.close / b.close;
                Some((ts_to_x(candle.open_time), open, high, low, close))
            })
            .collect();

        if ratio_candles.is_empty() {
            return vec![];
        }

        let (auto_lo, auto_hi) = if self.pair_candle_mode {
            ratio_candles.iter().fold(
                (f64::INFINITY, f64::NEG_INFINITY),
                |(lo, hi), (_, _o, h, l, _c)| (lo.min(*l), hi.max(*h)),
            )
        } else {
            ratio_candles.iter().fold(
                (f64::INFINITY, f64::NEG_INFINITY),
                |(lo, hi), (_, _o, _h, _l, c)| (lo.min(*c), hi.max(*c)),
            )
        };
        let pad = (auto_hi - auto_lo).max(0.0001) * PRICE_PADDING_PCT;
        let auto_lo = auto_lo - pad;
        let auto_hi = auto_hi + pad;

        let (ratio_lo, ratio_hi) = if ctx.state.y_auto {
            (auto_lo, auto_hi)
        } else {
            let range = (auto_hi - auto_lo) * ctx.state.y_scale;
            let mid = (auto_hi + auto_lo) * 0.5 + ctx.state.y_offset;
            (mid - range * 0.5, mid + range * 0.5)
        };
        let ratio_range = (ratio_hi - ratio_lo).max(0.0001);
        let ratio_to_y =
            |ratio: f64| -> f32 { ((ratio_hi - ratio) / ratio_range * ctx.chart_h as f64) as f32 };

        let mut frame = canvas::Frame::new(ctx.renderer, ctx.bounds.size());
        frame.fill_rectangle(Point::ORIGIN, ctx.bounds.size(), iced::Color::TRANSPARENT);

        draw_ratio_grid(&mut frame, &ctx, ratio_hi, ratio_range);
        draw_ratio_time_axis(&mut frame, &ctx);
        draw_ratio_base_line(&mut frame, &ctx, &ts_to_x);

        if self.pair_candle_mode {
            draw_ratio_candles(&mut frame, &ctx, &ratio_candles, &ratio_to_y, ctx.theme);
        } else {
            draw_ratio_line(
                &mut frame,
                &ctx,
                &ratio_candles,
                &ratio_to_y,
                ctx.theme.palette().primary,
            );
        }

        if let Some((_, _, _, _, last_ratio)) = ratio_candles.last() {
            frame.fill_text(canvas::Text {
                content: format!(
                    "{} / {}  {last_ratio:.4}",
                    series_a.display, series_b.display
                ),
                position: Point::new(8.0, 12.0),
                color: ctx.theme.palette().primary,
                size: iced::Pixels(11.0),
                align_x: alignment::Horizontal::Left.into(),
                align_y: alignment::Vertical::Center,
                font: iced::Font::MONOSPACE,
                ..canvas::Text::default()
            });
        }

        let base_geo = frame.into_geometry();
        let overlay = draw_ratio_crosshair(&ctx, ratio_hi, ratio_range);
        vec![base_geo, overlay]
    }
}
