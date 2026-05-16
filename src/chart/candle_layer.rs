mod axes;
mod candles;
mod funding;
mod liquidity;

pub(in crate::chart) use funding::format_funding_rate_percent;

use super::model::CandlestickChart;
use super::moving_averages::MovingAverageLayer;
use super::state::ChartState;
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Theme};

// ---------------------------------------------------------------------------
// Candle Layer Rendering
// ---------------------------------------------------------------------------

pub(super) struct CandleLayerContext<'a, IdxToCx, PriceToY>
where
    IdxToCx: Fn(usize) -> f32,
    PriceToY: Fn(f64) -> f32,
{
    pub(super) renderer: &'a Renderer,
    pub(super) theme: &'a Theme,
    pub(super) bounds: Rectangle,
    pub(super) state: &'a ChartState,
    pub(super) chart_w: f32,
    pub(super) chart_h: f32,
    pub(super) funding_panel_h: f32,
    pub(super) price_h: f32,
    pub(super) volume_h: f32,
    pub(super) candle_w: f32,
    pub(super) step: f32,
    pub(super) heatmap_stride: usize,
    pub(super) first_vis: usize,
    pub(super) last_vis: usize,
    pub(super) right_idx: isize,
    pub(super) price_lo: f64,
    pub(super) price_hi: f64,
    pub(super) price_range: f64,
    pub(super) vol_max: f64,
    pub(super) candle_bull_color: Color,
    pub(super) candle_bear_color: Color,
    pub(super) idx_to_cx: &'a IdxToCx,
    pub(super) price_to_y: &'a PriceToY,
}

impl CandlestickChart {
    pub(super) fn draw_candle_layer<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
    ) -> canvas::Geometry
    where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        self.candle_cache
            .draw(ctx.renderer, ctx.bounds.size(), |frame| {
                frame.fill_rectangle(Point::ORIGIN, ctx.bounds.size(), Color::TRANSPARENT);

                let chart_region = Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: ctx.bounds.width,
                    height: ctx.chart_h,
                };
                frame.with_clip(chart_region, |frame| {
                    self.draw_price_grid(ctx, frame);
                    self.draw_time_grid(ctx, frame);
                    self.draw_historical_heatmap(ctx, frame);
                    self.draw_candles_and_volume(ctx, frame);

                    let mut moving_average_layer = MovingAverageLayer {
                        frame: &mut *frame,
                        theme: ctx.theme,
                        first_vis: ctx.first_vis,
                        last_vis: ctx.last_vis,
                        chart_w: ctx.chart_w,
                        candle_w: ctx.candle_w,
                        idx_to_cx: ctx.idx_to_cx,
                        price_to_y: ctx.price_to_y,
                    };
                    self.draw_macro_moving_averages(&mut moving_average_layer);

                    self.draw_liquidation_bucket_bars(ctx, frame);
                });
                self.draw_price_axis_labels(ctx, frame);
                self.draw_funding_panel(ctx, frame);
                self.draw_time_axis_labels(ctx, frame);
                self.draw_axis_border(ctx, frame);
            })
    }
}
