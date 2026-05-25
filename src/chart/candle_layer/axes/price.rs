use super::super::CandleLayerContext;
use crate::chart::model::CandlestickChart;
use crate::helpers::format_price;
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point};

// ---------------------------------------------------------------------------
// Price Axis
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(in crate::chart::candle_layer) fn draw_price_grid<IdxToCx, PriceToY>(
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
    }

    pub(in crate::chart::candle_layer) fn draw_price_volume_separator<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
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

    pub(in crate::chart::candle_layer) fn draw_price_axis_labels<IdxToCx, PriceToY>(
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
                font: crate::app_fonts::monospace_font(),
                ..canvas::Text::default()
            });
        }
    }
}
