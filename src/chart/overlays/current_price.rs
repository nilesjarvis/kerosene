use super::TradingOverlayContext;
use crate::chart::drawing::{AxisBadgeStyle, fill_right_axis_badge, stroke_segmented_hline};
use crate::chart::model::CandlestickChart;
use crate::helpers::format_price;
use iced::Color;

// ---------------------------------------------------------------------------
// Current Price Overlay
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_current_price_line<PriceToY, IdxToCx>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    ) where
        PriceToY: Fn(f64) -> f32,
        IdxToCx: Fn(usize) -> f32,
    {
        if let Some(last_candle) = self.candles.last()
            && ctx.price_range > 0.0
        {
            let last_px = last_candle.close;
            let last_y = (ctx.price_to_y)(last_px);

            if last_y >= -10.0 && last_y <= ctx.price_h + 10.0 {
                let is_bullish = last_candle.close >= last_candle.open;
                let price_color = if is_bullish {
                    ctx.candle_bull_color
                } else {
                    ctx.candle_bear_color
                };
                let price_color_dim = if is_bullish {
                    Color {
                        a: 0.35,
                        ..ctx.candle_bull_color
                    }
                } else {
                    Color {
                        a: 0.35,
                        ..ctx.candle_bear_color
                    }
                };

                stroke_segmented_hline(
                    ctx.frame,
                    ctx.chart_w,
                    last_y,
                    2.0,
                    3.0,
                    price_color_dim,
                    1.0,
                );
                fill_right_axis_badge(
                    ctx.frame,
                    ctx.chart_w,
                    last_y,
                    format_price(last_px),
                    price_color,
                    AxisBadgeStyle {
                        char_width: 6.5,
                        padding_width: 8.0,
                        height: 16.0,
                        text_size: 10.0,
                        text_color: Color::BLACK,
                    },
                );
            }
        }
    }
}
