use super::PairRatioRenderContext;
use super::format_ratio_value;
use crate::spaghetti::helpers::format_relative_time;
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point};

// ---------------------------------------------------------------------------
// Pair Ratio Axes
// ---------------------------------------------------------------------------

pub(super) fn draw_ratio_grid(
    frame: &mut canvas::Frame,
    ctx: &PairRatioRenderContext<'_>,
    ratio_hi: f64,
    ratio_range: f64,
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
        let ratio_val = ratio_hi - (frac as f64) * ratio_range;
        frame.fill_text(canvas::Text {
            content: format_ratio_value(ratio_val),
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
}

pub(super) fn draw_ratio_time_axis(frame: &mut canvas::Frame, ctx: &PairRatioRenderContext<'_>) {
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

pub(super) fn draw_ratio_base_line(
    frame: &mut canvas::Frame,
    ctx: &PairRatioRenderContext<'_>,
    ts_to_x: &impl Fn(u64) -> f32,
) {
    if let Some(base_ts) = ctx.base_timestamp {
        let base_x = ts_to_x(base_ts);
        if base_x >= 0.0 && base_x <= ctx.chart_w {
            stroke_dashed_vline(frame, ctx, base_x);
        }
    }
}

fn stroke_dashed_vline(frame: &mut canvas::Frame, ctx: &PairRatioRenderContext<'_>, x: f32) {
    let dash_len: f32 = 4.0;
    let gap_len: f32 = 3.0;
    let mut y = 0.0_f32;
    while y < ctx.chart_h {
        let end = (y + dash_len).min(ctx.chart_h);
        let seg = canvas::Path::line(Point::new(x, y), Point::new(x, end));
        frame.stroke(
            &seg,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.2,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );
        y += dash_len + gap_len;
    }
}
