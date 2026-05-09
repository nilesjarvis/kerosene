use super::CrosshairOverlayContext;
use super::range::calculate_range_measurement;
use crate::chart::CandlestickChart;
use iced::widget::canvas;
use iced::{Color, Point, Size, alignment};

// ---------------------------------------------------------------------------
// Range Measurement Rendering
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_range_measurement<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
        pos: Point,
        hover_price: f64,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let Some(anchor_price) = ctx.state.range_anchor_price else {
            return;
        };

        let measurement = calculate_range_measurement(
            anchor_price,
            hover_price,
            (ctx.price_to_y)(anchor_price),
            pos.x,
            pos.y,
            ctx.chart_w,
            ctx.price_h,
        );
        let line_color = if measurement.is_up {
            ctx.theme.palette().success
        } else {
            ctx.theme.palette().danger
        };
        let fill_color = if measurement.is_up {
            Color {
                a: 0.12,
                ..ctx.theme.palette().success
            }
        } else {
            Color {
                a: 0.12,
                ..ctx.theme.palette().danger
            }
        };

        if measurement.bottom > measurement.top {
            ctx.frame.fill_rectangle(
                Point::new(0.0, measurement.top),
                Size::new(ctx.chart_w, measurement.bottom - measurement.top),
                fill_color,
            );
        }

        let anchor_line = canvas::Path::line(
            Point::new(0.0, measurement.anchor_y),
            Point::new(ctx.chart_w, measurement.anchor_y),
        );
        ctx.frame.stroke(
            &anchor_line,
            canvas::Stroke::default()
                .with_color(line_color)
                .with_width(1.0),
        );

        let hover_line = canvas::Path::line(
            Point::new(0.0, measurement.hover_y),
            Point::new(ctx.chart_w, measurement.hover_y),
        );
        ctx.frame.stroke(
            &hover_line,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.28,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );

        ctx.frame.fill_rectangle(
            Point::new(measurement.label_x, measurement.label_y),
            Size::new(measurement.label_width, measurement.label_height),
            Color {
                a: 0.92,
                ..ctx.theme.extended_palette().background.strong.color
            },
        );
        ctx.frame.fill_text(canvas::Text {
            content: measurement.label,
            position: Point::new(measurement.label_x + 4.0, measurement.label_y + 8.0),
            color: line_color,
            size: iced::Pixels(11.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: iced::Font::MONOSPACE,
            ..canvas::Text::default()
        });
    }
}
