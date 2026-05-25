use super::super::NormalizedRenderContext;
use super::series_render_color;
use crate::helpers::text_color_for_bg;
use crate::spaghetti::{ComparisonColorMode, Series};

use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point, Size};

mod connector;
mod layout;
use connector::{stroke_segmented_hline_range, stroke_segmented_label_connector};
pub(in crate::spaghetti::normalized) use layout::{
    SeriesLabelAnchor, stack_series_label_positions,
};

// ---------------------------------------------------------------------------
// Series Labels
// ---------------------------------------------------------------------------

const SERIES_LABEL_HEIGHT: f32 = 34.0;
const SERIES_LABEL_GAP: f32 = 4.0;
const SERIES_LABEL_MARGIN: f32 = 2.0;
const SERIES_LABEL_X_OFFSET: f32 = 12.0;
const SERIES_LABEL_PADDING_X: f32 = 10.0;
const SERIES_LABEL_DISPLAY_Y_OFFSET: f32 = -2.0;
const SERIES_LABEL_VALUE_Y_OFFSET: f32 = 1.0;
const SERIES_LABEL_CONNECTOR_SPAN: f32 = 22.0;
const SERIES_LABEL_DISPLAY_CHAR_W: f32 = 6.0;
const SERIES_LABEL_VALUE_CHAR_W: f32 = 5.4;

struct SeriesLabel {
    index: usize,
    display: String,
    pct: f64,
    y: f32,
    label_y: f32,
    color: Color,
}

pub(in crate::spaghetti::normalized) fn draw_series_labels(
    frame: &mut canvas::Frame,
    ctx: &NormalizedRenderContext<'_>,
    series_data: &[(&Series, Vec<(f32, f64)>)],
    pct_to_y: &impl Fn(f64) -> f32,
    color_mode: ComparisonColorMode,
) {
    let mut labels = Vec::new();
    for (index, (series, points)) in series_data.iter().enumerate() {
        let Some((_, pct)) = points.last() else {
            continue;
        };
        let y = pct_to_y(*pct);
        if !(0.0..=ctx.chart_h).contains(&y) || !y.is_finite() {
            continue;
        }

        labels.push(SeriesLabel {
            index,
            display: series.display.clone(),
            pct: *pct,
            y,
            label_y: y,
            color: series_render_color(ctx.theme, color_mode, series),
        });
    }

    let label_positions = stack_series_label_positions(
        labels
            .iter()
            .map(|label| SeriesLabelAnchor {
                index: label.index,
                y: label.y,
            })
            .collect(),
        ctx.chart_h,
    );
    for label in &mut labels {
        if let Some(label_y) = label_positions.get(label.index).copied().flatten() {
            label.label_y = label_y;
        }
    }

    for label in labels {
        draw_series_label(frame, ctx, &label);
    }
}

fn draw_series_label(
    frame: &mut canvas::Frame,
    ctx: &NormalizedRenderContext<'_>,
    label: &SeriesLabel,
) {
    let shifted = (label.label_y - label.y).abs() >= 1.0;
    let label_x = ctx.chart_w + 2.0;
    let performance = performance_label(label.pct);
    let label_width = series_label_width(&label.display, &performance)
        .min((ctx.bounds.width - label_x - 2.0).max(0.0));
    let label_top = label.label_y - SERIES_LABEL_HEIGHT * 0.5;
    let mut label_background = label.color;
    label_background.a = 0.92;
    let label_text_color = text_color_for_bg(label.color);
    let line_end = if shifted {
        (ctx.chart_w - SERIES_LABEL_CONNECTOR_SPAN).max(0.0)
    } else {
        ctx.chart_w
    };
    let line_color = Color {
        a: 0.5,
        ..label.color
    };

    stroke_segmented_hline_range(frame, 0.0, line_end, label.y, line_color);
    if shifted {
        stroke_segmented_label_connector(
            frame,
            Point::new(line_end, label.y),
            Point::new(line_end + (ctx.chart_w - line_end) * 0.55, label.y),
            Point::new(ctx.chart_w + 2.0, label.label_y),
            line_color,
        );
    }

    if label_width > 0.0 {
        frame.fill_rectangle(
            Point::new(label_x, label_top),
            Size::new(label_width, SERIES_LABEL_HEIGHT),
            label_background,
        );
    }

    frame.fill_text(canvas::Text {
        content: label.display.clone(),
        position: Point::new(
            ctx.chart_w + SERIES_LABEL_X_OFFSET,
            label.label_y + SERIES_LABEL_DISPLAY_Y_OFFSET,
        ),
        color: label_text_color,
        size: iced::Pixels(10.0),
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Bottom,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });

    frame.fill_text(canvas::Text {
        content: performance,
        position: Point::new(
            ctx.chart_w + SERIES_LABEL_X_OFFSET,
            label.label_y + SERIES_LABEL_VALUE_Y_OFFSET,
        ),
        color: Color {
            a: 0.82,
            ..label_text_color
        },
        size: iced::Pixels(9.0),
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Top,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

fn series_label_width(display: &str, performance: &str) -> f32 {
    let display_w = display.len() as f32 * SERIES_LABEL_DISPLAY_CHAR_W;
    let performance_w = performance.len() as f32 * SERIES_LABEL_VALUE_CHAR_W;
    display_w.max(performance_w) + SERIES_LABEL_PADDING_X * 2.0
}

pub(in crate::spaghetti::normalized) fn performance_label(pct: f64) -> String {
    format!("{pct:+.2}%")
}
