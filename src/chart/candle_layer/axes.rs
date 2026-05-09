use super::CandleLayerContext;
use crate::chart::model::CandlestickChart;
use crate::helpers::{format_price, format_timestamp};
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point};

// ---------------------------------------------------------------------------
// Axis and Grid Rendering
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_price_grid<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let grid_steps = 5usize;
        for i in 0..=grid_steps {
            let frac = i as f32 / grid_steps as f32;
            let y = frac * ctx.price_h;
            let line = canvas::Path::line(Point::new(0.0, y), Point::new(ctx.chart_w, y));
            frame.stroke(
                &line,
                canvas::Stroke::default()
                    .with_color(Color {
                        a: 0.06,
                        ..ctx.theme.palette().text
                    })
                    .with_width(1.0),
            );
        }

        let sep = canvas::Path::line(
            Point::new(0.0, ctx.price_h),
            Point::new(ctx.chart_w, ctx.price_h),
        );
        frame.stroke(
            &sep,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.10,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );
    }

    pub(super) fn draw_price_axis_labels<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let grid_steps = 5usize;
        for i in 0..=grid_steps {
            let frac = i as f32 / grid_steps as f32;
            let y = frac * ctx.price_h;
            let price_val = if self.inverted {
                ctx.price_lo + (frac as f64) * ctx.price_range
            } else {
                ctx.price_hi - (frac as f64) * ctx.price_range
            };
            frame.fill_text(canvas::Text {
                content: format_price(price_val),
                position: Point::new(ctx.chart_w + 6.0, y),
                color: Color {
                    a: 0.45,
                    ..ctx.theme.palette().text
                },
                size: iced::Pixels(11.0),
                align_x: alignment::Horizontal::Left.into(),
                align_y: alignment::Vertical::Center,
                font: iced::Font::MONOSPACE,
                ..canvas::Text::default()
            });
        }
    }

    pub(super) fn draw_time_axis_labels<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let label_count = 6usize.min(ctx.last_vis - ctx.first_vis + 1);
        if label_count <= 1 {
            return;
        }

        let vis_count = ctx.last_vis - ctx.first_vis + 1;
        let step_i = vis_count / label_count;
        for li in 0..label_count {
            let ci = ctx.first_vis + li * step_i;
            if ci > ctx.last_vis {
                continue;
            }
            let ts_secs = self.candles[ci].open_time / 1000;
            let label = format_timestamp(ts_secs);
            let x = (ctx.idx_to_cx)(ci);
            if x > 0.0 && x < ctx.chart_w {
                frame.fill_text(canvas::Text {
                    content: label,
                    position: Point::new(x, ctx.chart_h + ctx.funding_panel_h + 4.0),
                    color: Color {
                        a: 0.45,
                        ..ctx.theme.palette().text
                    },
                    size: iced::Pixels(10.0),
                    align_x: alignment::Horizontal::Center.into(),
                    align_y: alignment::Vertical::Top,
                    font: iced::Font::MONOSPACE,
                    ..canvas::Text::default()
                });
            }
        }
    }

    pub(super) fn draw_axis_border<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let axis_border = canvas::Path::line(
            Point::new(ctx.chart_w, 0.0),
            Point::new(ctx.chart_w, ctx.chart_h + ctx.funding_panel_h),
        );
        frame.stroke(
            &axis_border,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.10,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );
    }
}
