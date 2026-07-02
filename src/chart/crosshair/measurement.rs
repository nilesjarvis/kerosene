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
        data_pos: Point,
        visual_pos: Point,
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
            data_pos,
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
            ctx.fisheye.fill_projected_rect_without_edge_blur(
                ctx.frame,
                Point::new(0.0, measurement.top),
                Size::new(ctx.chart_w, measurement.bottom - measurement.top),
                fill_color,
            );
        }

        ctx.fisheye.stroke_projected_line_without_edge_blur(
            ctx.frame,
            Point::new(0.0, measurement.anchor_y),
            Point::new(ctx.chart_w, measurement.anchor_y),
            canvas::Stroke::default()
                .with_color(line_color)
                .with_width(1.0),
        );

        ctx.fisheye.stroke_projected_line_without_edge_blur(
            ctx.frame,
            Point::new(0.0, measurement.hover_y),
            Point::new(ctx.chart_w, measurement.hover_y),
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.28,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );

        let label_x = (visual_pos.x + 10.0)
            .min(ctx.chart_w - measurement.label_width - 4.0)
            .max(4.0);
        let label_y = (visual_pos.y - 20.0)
            .max(4.0)
            .min(ctx.price_h - measurement.label_height - 2.0);

        ctx.frame.fill_rectangle(
            Point::new(label_x, label_y),
            Size::new(measurement.label_width, measurement.label_height),
            Color {
                a: 0.92,
                ..ctx.theme.extended_palette().background.strong.color
            },
        );
        ctx.frame.fill_text(canvas::Text {
            content: measurement.label,
            position: Point::new(label_x + 4.0, label_y + 8.0),
            color: line_color,
            size: iced::Pixels(11.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..canvas::Text::default()
        });
    }
}
