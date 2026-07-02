use super::TradingOverlayContext;
use crate::api::Candle;
use crate::chart::drawing::{SegmentedHLineStyle, stroke_projected_segmented_hline_with_offset};
use crate::chart::model::CandlestickChart;
use crate::helpers::format_price;

use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point, Size};

// ---------------------------------------------------------------------------
// Visible High / Low Overlay
// ---------------------------------------------------------------------------

const LABEL_HEIGHT: f32 = 13.0;
const LABEL_CHAR_WIDTH: f32 = 5.8;
const LABEL_HORIZONTAL_PADDING: f32 = 6.0;
const LABEL_RIGHT_PADDING: f32 = 7.0;
const LABEL_LINE_GAP: f32 = 6.0;
const LABEL_EDGE_PADDING: f32 = 2.0;

impl CandlestickChart {
    pub(super) fn draw_visible_high_low_lines<PriceToY, IdxToCx>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    ) where
        PriceToY: Fn(f64) -> f32,
        IdxToCx: Fn(usize) -> f32,
    {
        if !self.macro_indicators.show_high_low
            || ctx.chart_w <= 0.0
            || ctx.price_h <= 0.0
            || !ctx.chart_w.is_finite()
            || !ctx.price_h.is_finite()
        {
            return;
        }

        let Some((low, high)) = visible_high_low(ctx.candles, ctx.first_vis, ctx.last_vis) else {
            return;
        };

        if prices_match(low, high) {
            draw_high_low_label(ctx, "H/L", high);
            return;
        }

        draw_high_low_label(ctx, "H", high);
        draw_high_low_label(ctx, "L", low);
    }
}

fn visible_high_low(candles: &[Candle], first_vis: usize, last_vis: usize) -> Option<(f64, f64)> {
    let visible = candles.get(first_vis..=last_vis)?;
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;

    for candle in visible {
        if candle.low.is_finite() {
            low = low.min(candle.low);
        }
        if candle.high.is_finite() {
            high = high.max(candle.high);
        }
    }

    (low.is_finite() && high.is_finite()).then_some((low, high))
}

fn prices_match(a: f64, b: f64) -> bool {
    (a - b).abs() <= f64::EPSILON
}

fn draw_high_low_label<PriceToY, IdxToCx>(
    ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    prefix: &'static str,
    price: f64,
) where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    if !price.is_finite() {
        return;
    }

    let y = (ctx.price_to_y)(price);
    if !y.is_finite() || y < -10.0 || y > ctx.price_h + 10.0 {
        return;
    }

    let line_color = Color {
        a: 0.18,
        ..ctx.theme.palette().text
    };
    let line_style = SegmentedHLineStyle {
        segment_len: 3.0,
        gap_len: 5.0,
        offset: 0.0,
        color: line_color,
        width: 1.0,
    };

    let label = format!("{} {}", prefix, format_price(price));
    let label_width = label.len() as f32 * LABEL_CHAR_WIDTH + LABEL_HORIZONTAL_PADDING * 2.0;
    let label_right = (ctx.chart_w - LABEL_RIGHT_PADDING).max(0.0);
    let label_left = (label_right - label_width).max(0.0);
    let line_end_x = (label_left - LABEL_LINE_GAP).max(0.0);
    stroke_projected_segmented_hline_with_offset(ctx.frame, ctx.fisheye, line_end_x, y, line_style);

    let label_center_y = label_y_in_bounds(y, ctx.price_h);
    let background_width = (label_right - label_left).max(0.0);
    if background_width > 0.0 {
        ctx.frame.fill_rectangle(
            Point::new(label_left, label_center_y - LABEL_HEIGHT * 0.5),
            Size::new(background_width, LABEL_HEIGHT),
            Color {
                a: 0.42,
                ..ctx.theme.extended_palette().background.base.color
            },
        );
    }

    ctx.frame.fill_text(canvas::Text {
        content: label,
        position: Point::new(
            (label_right - LABEL_HORIZONTAL_PADDING).max(0.0),
            label_center_y,
        ),
        color: Color {
            a: 0.72,
            ..ctx.theme.palette().text
        },
        size: iced::Pixels(9.0),
        align_x: alignment::Horizontal::Right.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

fn label_y_in_bounds(y: f32, price_h: f32) -> f32 {
    let half_height = LABEL_HEIGHT * 0.5;
    let min_y = half_height + LABEL_EDGE_PADDING;
    let max_y = price_h - half_height - LABEL_EDGE_PADDING;
    if max_y <= min_y {
        return y;
    }
    y.clamp(min_y, max_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(open_time: u64, high: f64, low: f64) -> Candle {
        Candle::test_ohlcv(open_time, open_time + 59_999, [low, high, low, high], 1.0)
    }

    #[test]
    fn visible_high_low_uses_only_visible_candle_range() {
        let candles = vec![
            candle(0, 900.0, 1.0),
            candle(60_000, 110.0, 100.0),
            candle(120_000, 125.0, 90.0),
            candle(180_000, 130.0, -800.0),
        ];

        assert_eq!(visible_high_low(&candles, 1, 2), Some((90.0, 125.0)));
    }
}
