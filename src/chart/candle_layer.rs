mod axes;
mod candles;
mod earnings;
mod funding;
mod line_series;
mod liquidity;
mod secondary_series;
mod sessions;

pub(in crate::chart) use earnings::{EARNINGS_DOT_RADIUS, earnings_marker_dot_y};
pub(in crate::chart) use funding::format_funding_rate_percent;
pub(in crate::chart) use line_series::line_series_stroke_color;

use super::fisheye::ChartFisheye;
use super::model::CandlestickChart;
use super::moving_averages::MovingAverageLayer;
use super::state::ChartState;
use crate::chart_background::{draw_dotted_background, draw_gradient_background};
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
    pub(super) session_panel_h: f32,
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
    pub(super) fisheye: ChartFisheye,
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
                    width: ctx.chart_w,
                    height: ctx.chart_h,
                };
                frame.with_clip(chart_region, |frame| {
                    if self.gradient_background {
                        draw_gradient_background(frame, ctx.theme, ctx.chart_w, ctx.chart_h);
                    }
                    self.draw_session_chart_context(ctx, frame);
                    if self.dotted_background {
                        draw_dotted_background(
                            frame,
                            ctx.theme,
                            ctx.chart_w,
                            ctx.chart_h,
                            self.dotted_background_opacity,
                            ctx.fisheye,
                        );
                    } else {
                        self.draw_price_grid(ctx, frame);
                        self.draw_time_grid(ctx, frame);
                    }
                    self.draw_price_volume_separator(ctx, frame);
                    self.draw_historical_heatmap(ctx, frame);
                    if self.series_style.is_line() {
                        self.draw_line_series(ctx, frame);
                    } else {
                        self.draw_candles(ctx, frame);
                    }
                    self.draw_volume_bars(ctx, frame);
                    self.draw_earnings_markers(ctx, frame);
                    self.draw_volume_profile(ctx, frame);

                    let mut moving_average_layer = MovingAverageLayer {
                        frame: &mut *frame,
                        theme: ctx.theme,
                        first_vis: ctx.first_vis,
                        last_vis: ctx.last_vis,
                        chart_w: ctx.chart_w,
                        candle_w: ctx.candle_w,
                        fisheye: ctx.fisheye,
                        idx_to_cx: ctx.idx_to_cx,
                        price_to_y: ctx.price_to_y,
                    };
                    self.draw_macro_moving_averages(&mut moving_average_layer);

                    self.draw_secondary_series(ctx, frame);
                    self.draw_liquidation_bucket_bars(ctx, frame);
                });
                self.draw_price_axis_labels(ctx, frame);
                self.draw_secondary_price_axis_labels(ctx, frame);
                self.draw_funding_panel(ctx, frame);
                self.draw_session_panel(ctx, frame);
                self.draw_time_axis_labels(ctx, frame);
                self.draw_axis_border(ctx, frame);
            })
    }
}
