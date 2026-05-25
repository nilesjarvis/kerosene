use super::NormalizedRenderContext;
use crate::spaghetti::helpers::format_relative_time;
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point};

// ---------------------------------------------------------------------------
// Normalized Axes
// ---------------------------------------------------------------------------

pub(super) fn draw_grid_and_axes(
    frame: &mut canvas::Frame,
    ctx: &NormalizedRenderContext<'_>,
    pct_hi: f64,
    pct_range: f64,
    pct_to_y: &impl Fn(f64) -> f32,
    show_grid: bool,
) {
    let grid_steps = 6usize;
    for i in 0..=grid_steps {
        let frac = i as f32 / grid_steps as f32;
        let y = frac * ctx.chart_h;
        if show_grid {
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
        let pct_val = pct_hi - (frac as f64) * pct_range;
        frame.fill_text(canvas::Text {
            content: format!("{pct_val:+.1}%"),
            position: Point::new(ctx.chart_w + 4.0, y),
            color: Color {
                a: 0.45,
                ..ctx.theme.palette().text
            },
            size: iced::Pixels(10.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..canvas::Text::default()
        });
    }

    let zero_y = pct_to_y(0.0);
    if zero_y >= 0.0 && zero_y <= ctx.chart_h {
        let baseline = canvas::Path::line(Point::new(0.0, zero_y), Point::new(ctx.chart_w, zero_y));
        frame.stroke(
            &baseline,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.15,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );
    }

    let axis_border = canvas::Path::line(
        Point::new(ctx.chart_w, 0.0),
        Point::new(ctx.chart_w, ctx.chart_h),
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

    draw_time_axis(frame, ctx);
}

fn draw_time_axis(frame: &mut canvas::Frame, ctx: &NormalizedRenderContext<'_>) {
    let label_count = 7usize;
    let now_ms = ctx.effective_max as f64;
    for i in 0..label_count {
        let frac = i as f64 / (label_count - 1).max(1) as f64;
        let ts = ctx.left_ts + frac * ctx.visible_ms;
        let x = (frac * ctx.chart_w as f64) as f32;
        if x > 10.0 && x < ctx.chart_w - 10.0 {
            let delta_from_now = now_ms - ts;
            let label = format_relative_time(delta_from_now);
            frame.fill_text(canvas::Text {
                content: label,
                position: Point::new(x, ctx.chart_h + 4.0),
                color: Color {
                    a: 0.45,
                    ..ctx.theme.palette().text
                },
                size: iced::Pixels(10.0),
                align_x: alignment::Horizontal::Center.into(),
                align_y: alignment::Vertical::Top,
                font: crate::app_fonts::monospace_font(),
                ..canvas::Text::default()
            });
        }
    }
}
