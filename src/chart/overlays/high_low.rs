use super::TradingOverlayContext;
use crate::api::Candle;
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
const LABEL_POINT_GAP: f32 = 4.0;
const LABEL_EDGE_PADDING: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct VisiblePricePoint {
    candle_index: usize,
    price: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct VisibleHighLow {
    low: VisiblePricePoint,
    high: VisiblePricePoint,
}

#[derive(Debug, Clone, Copy)]
enum LabelSide {
    Above,
    Below,
}

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

        let Some(extrema) = visible_high_low(ctx.candles, ctx.first_vis, ctx.last_vis) else {
            return;
        };

        if prices_match(extrema.low.price, extrema.high.price) {
            draw_high_low_label(ctx, "H/L", extrema.high, LabelSide::Above);
            return;
        }

        draw_high_low_label(ctx, "H", extrema.high, LabelSide::Above);
        draw_high_low_label(ctx, "L", extrema.low, LabelSide::Below);
    }
}

fn visible_high_low(
    candles: &[Candle],
    first_vis: usize,
    last_vis: usize,
) -> Option<VisibleHighLow> {
    let visible = candles.get(first_vis..=last_vis)?;
    let mut low: Option<VisiblePricePoint> = None;
    let mut high: Option<VisiblePricePoint> = None;

    for (offset, candle) in visible.iter().enumerate() {
        let candle_index = first_vis + offset;
        if candle.low.is_finite() {
            let point = VisiblePricePoint {
                candle_index,
                price: candle.low,
            };
            if low.is_none_or(|current| point.price < current.price) {
                low = Some(point);
            }
        }
        if candle.high.is_finite() {
            let point = VisiblePricePoint {
                candle_index,
                price: candle.high,
            };
            if high.is_none_or(|current| point.price > current.price) {
                high = Some(point);
            }
        }
    }

    Some(VisibleHighLow {
        low: low?,
        high: high?,
    })
}

fn prices_match(a: f64, b: f64) -> bool {
    (a - b).abs() <= f64::EPSILON
}

fn draw_high_low_label<PriceToY, IdxToCx>(
    ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    prefix: &'static str,
    point: VisiblePricePoint,
    side: LabelSide,
) where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    if !point.price.is_finite() {
        return;
    }

    let source_x = (ctx.idx_to_cx)(point.candle_index);
    let source_y = (ctx.price_to_y)(point.price);
    if !source_x.is_finite() || !source_y.is_finite() {
        return;
    }

    let marker = ctx.fisheye.project(Point::new(source_x, source_y));
    if !marker.x.is_finite()
        || !marker.y.is_finite()
        || marker.x < -10.0
        || marker.x > ctx.chart_w + 10.0
        || marker.y < -10.0
        || marker.y > ctx.price_h + 10.0
    {
        return;
    }

    let marker_color = Color {
        a: 0.18,
        ..ctx.theme.palette().text
    };
    let tick_half_width = 4.0;
    ctx.fisheye.stroke_projected_line_without_edge_blur(
        ctx.frame,
        Point::new((source_x - tick_half_width).max(0.0), source_y),
        Point::new((source_x + tick_half_width).min(ctx.chart_w), source_y),
        canvas::Stroke::default()
            .with_color(marker_color)
            .with_width(1.0),
    );

    let label = format!("{} {}", prefix, format_price(point.price));
    let label_width = label_width_in_bounds(label.len(), ctx.chart_w);
    let label_left = label_left_in_bounds(marker.x, label_width, ctx.chart_w);
    let label_top = label_top_near_price(marker.y, side, ctx.price_h);

    if label_width > 0.0 {
        ctx.frame.fill_rectangle(
            Point::new(label_left, label_top),
            Size::new(label_width, LABEL_HEIGHT),
            Color {
                a: 0.42,
                ..ctx.theme.extended_palette().background.base.color
            },
        );
    }

    ctx.frame.fill_text(canvas::Text {
        content: label,
        position: Point::new(
            label_left + label_width * 0.5,
            label_top + LABEL_HEIGHT * 0.5,
        ),
        color: Color {
            a: 0.72,
            ..ctx.theme.palette().text
        },
        size: iced::Pixels(9.0),
        align_x: alignment::Horizontal::Center.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

fn label_width_in_bounds(label_len: usize, chart_w: f32) -> f32 {
    let desired = label_len as f32 * LABEL_CHAR_WIDTH + LABEL_HORIZONTAL_PADDING * 2.0;
    let max_width = (chart_w - LABEL_EDGE_PADDING * 2.0).max(0.0);
    desired.min(max_width)
}

fn label_left_in_bounds(center_x: f32, label_width: f32, chart_w: f32) -> f32 {
    let min_left = LABEL_EDGE_PADDING;
    let max_left = (chart_w - label_width - LABEL_EDGE_PADDING).max(min_left);
    (center_x - label_width * 0.5).clamp(min_left, max_left)
}

fn label_top_near_price(price_y: f32, side: LabelSide, price_h: f32) -> f32 {
    let desired = match side {
        LabelSide::Above => price_y - LABEL_POINT_GAP - LABEL_HEIGHT,
        LabelSide::Below => price_y + LABEL_POINT_GAP,
    };
    let min_top = LABEL_EDGE_PADDING;
    let max_top = (price_h - LABEL_HEIGHT - LABEL_EDGE_PADDING).max(min_top);
    desired.clamp(min_top, max_top)
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

        assert_eq!(
            visible_high_low(&candles, 1, 2),
            Some(VisibleHighLow {
                low: VisiblePricePoint {
                    candle_index: 2,
                    price: 90.0,
                },
                high: VisiblePricePoint {
                    candle_index: 2,
                    price: 125.0,
                },
            })
        );
    }

    #[test]
    fn visible_high_low_keeps_first_visible_candle_for_tied_extrema() {
        let candles = vec![
            candle(0, 110.0, 95.0),
            candle(60_000, 120.0, 90.0),
            candle(120_000, 120.0, 92.0),
            candle(180_000, 118.0, 90.0),
        ];

        assert_eq!(
            visible_high_low(&candles, 1, 3),
            Some(VisibleHighLow {
                low: VisiblePricePoint {
                    candle_index: 1,
                    price: 90.0,
                },
                high: VisiblePricePoint {
                    candle_index: 1,
                    price: 120.0,
                },
            })
        );
    }
}
