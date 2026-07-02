use super::CandleLayerContext;
use crate::chart::indicators::{
    LELEDC_BAR_COUNT, LELEDC_SWING_LENGTH, LeledcLevel, LeledcSignal, calculate_leledc,
};
use crate::chart::model::CandlestickChart;
use iced::widget::canvas;
use iced::{Color, Point};

// ---------------------------------------------------------------------------
// Leledc Exhaustion Overlay
// ---------------------------------------------------------------------------

const LEVEL_LINE_ALPHA: f32 = 0.7;
const LEVEL_LINE_WIDTH: f32 = 2.0;
const MARKER_ALPHA: f32 = 0.9;
const MARKER_HALF_WIDTH_MIN: f32 = 3.0;
const MARKER_HALF_WIDTH_MAX: f32 = 7.0;
const MARKER_HEIGHT_RATIO: f32 = 1.7;
const MARKER_GAP: f32 = 4.0;

impl CandlestickChart {
    pub(super) fn draw_leledc_overlay<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let show_arrows = self.macro_indicators.show_leledc_arrows;
        let show_levels = self.macro_indicators.show_leledc_levels;
        if (!show_arrows && !show_levels) || self.candles.is_empty() || ctx.chart_w <= 0.0 {
            return;
        }

        let levels = calculate_leledc(&self.candles, LELEDC_SWING_LENGTH, LELEDC_BAR_COUNT);
        let extended = ctx.theme.extended_palette();
        let resistance_color = extended.danger.base.color;
        let support_color = extended.success.base.color;

        if show_levels {
            self.draw_leledc_level_lines(ctx, frame, &levels.resistance, resistance_color);
            self.draw_leledc_level_lines(ctx, frame, &levels.support, support_color);
        }
        if show_arrows {
            for &(idx, signal) in &levels.signals {
                if idx < ctx.first_vis || idx > ctx.last_vis {
                    continue;
                }
                let color = match signal {
                    LeledcSignal::Bearish => resistance_color,
                    LeledcSignal::Bullish => support_color,
                };
                self.draw_leledc_marker(ctx, frame, idx, signal, color);
            }
        }
    }

    fn draw_leledc_level_lines<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        segments: &[LeledcLevel],
        color: Color,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let stroke_color = Color {
            a: LEVEL_LINE_ALPHA,
            ..color
        };
        for segment in segments {
            if segment.end < ctx.first_vis || segment.start > ctx.last_vis {
                continue;
            }
            let mut x1 = (ctx.idx_to_cx)(segment.start.max(ctx.first_vis));
            let mut x2 = (ctx.idx_to_cx)(segment.end.min(ctx.last_vis));
            if x2 - x1 < ctx.candle_w {
                // Keep a freshly set level visible until the next bar arrives.
                let cx = (x1 + x2) * 0.5;
                x1 = cx - ctx.candle_w * 0.5;
                x2 = cx + ctx.candle_w * 0.5;
            }
            let y = (ctx.price_to_y)(segment.price);
            ctx.fisheye.stroke_projected_line(
                frame,
                Point::new(x1, y),
                Point::new(x2, y),
                canvas::Stroke::default()
                    .with_color(stroke_color)
                    .with_width(LEVEL_LINE_WIDTH),
            );
        }
    }

    fn draw_leledc_marker<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        idx: usize,
        signal: LeledcSignal,
        color: Color,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let Some(candle) = self.candles.get(idx) else {
            return;
        };
        let cx = (ctx.idx_to_cx)(idx);
        let half_width = (ctx.candle_w * 0.45).clamp(MARKER_HALF_WIDTH_MIN, MARKER_HALF_WIDTH_MAX);
        let height = half_width * MARKER_HEIGHT_RATIO;
        let fill_color = Color {
            a: MARKER_ALPHA,
            ..color
        };

        // Triangle tip points at the exhaustion extreme, base away from the bar.
        let points = match signal {
            LeledcSignal::Bearish => {
                let tip_y = (ctx.price_to_y)(candle.high) - MARKER_GAP;
                [
                    Point::new(cx, tip_y),
                    Point::new(cx - half_width, tip_y - height),
                    Point::new(cx + half_width, tip_y - height),
                ]
            }
            LeledcSignal::Bullish => {
                let tip_y = (ctx.price_to_y)(candle.low) + MARKER_GAP;
                [
                    Point::new(cx, tip_y),
                    Point::new(cx - half_width, tip_y + height),
                    Point::new(cx + half_width, tip_y + height),
                ]
            }
        };
        ctx.fisheye
            .fill_projected_polygon_without_edge_blur(frame, &points, fill_color);
    }
}
