use super::super::NormalizedRenderContext;
use super::series_render_color;
use crate::helpers::{ellipsized_text, symbol_svg_logo, text_color_for_bg};
use crate::spaghetti::{ComparisonColorMode, Series};

use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Size};

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
const SERIES_LABEL_PADDING_X: f32 = 10.0;
const SERIES_LABEL_DISPLAY_SIZE: f32 = 10.0;
const SERIES_LABEL_DISPLAY_Y_OFFSET: f32 = -2.0;
const SERIES_LABEL_VALUE_Y_OFFSET: f32 = 1.0;
const SERIES_LABEL_CONNECTOR_SPAN: f32 = 22.0;
const SERIES_LABEL_DISPLAY_CHAR_W: f32 = 6.0;
const SERIES_LABEL_VALUE_CHAR_W: f32 = 5.4;
/// Height of the asset logo painted immediately left of the ticker text, when
/// an SVG is available for the series' symbol. The drawn width follows the
/// logo's intrinsic aspect ratio (capped at [`SERIES_LABEL_ICON_MAX_W`]) so
/// non-square logos are neither stretched nor clipped.
const SERIES_LABEL_ICON_H: f32 = 9.0;
/// Upper bound on the logo's drawn width; wide wordmark logos are scaled down
/// preserving aspect rather than overrunning the label.
const SERIES_LABEL_ICON_MAX_W: f32 = 14.0;
/// Padding between the label's left edge and the logo.
const SERIES_LABEL_ICON_LEFT_PAD: f32 = 5.0;
/// Gap between the logo and the ticker text.
const SERIES_LABEL_ICON_GAP: f32 = 4.0;

struct SeriesLabel {
    index: usize,
    symbol: String,
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
            symbol: series.symbol.clone(),
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
    let max_label_width = (ctx.bounds.width - label_x - 2.0).max(0.0);

    // Resolve the asset logo and size it to a fixed height with an
    // aspect-correct width so non-square logos render without distortion or
    // clipping. The text is inset past the logo so the logo sits directly to
    // the left of the ticker rather than floating in the label's padding.
    let logo = symbol_svg_logo(&label.symbol).map(|(handle, aspect)| {
        let mut size = Size::new(SERIES_LABEL_ICON_H * aspect, SERIES_LABEL_ICON_H);
        if size.width > SERIES_LABEL_ICON_MAX_W {
            size = Size::new(SERIES_LABEL_ICON_MAX_W, SERIES_LABEL_ICON_MAX_W / aspect);
        }
        (handle, size)
    });
    let left_inset = match &logo {
        Some((_, size)) => SERIES_LABEL_ICON_LEFT_PAD + size.width + SERIES_LABEL_ICON_GAP,
        None => SERIES_LABEL_PADDING_X,
    };
    let text_x = label_x + left_inset;

    let display = axis_display_label(
        &label.display,
        max_label_width - left_inset - SERIES_LABEL_PADDING_X,
    );
    let performance = performance_label(label.pct);
    let label_width =
        (series_text_width(&display, &performance) + left_inset + SERIES_LABEL_PADDING_X)
            .min(max_label_width);
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

        // Paint a single-color asset logo centered on the ticker text line (the
        // upper of the two stacked lines) so it reads as part of the ticker
        // rather than floating at the label's vertical center. The logo is
        // tinted to the label text color so it stays legible against the
        // colored label background regardless of the asset's original colors.
        if let Some((handle, size)) = logo {
            let icon = iced::advanced::svg::Svg::new(handle).color(label_text_color);
            // The ticker is bottom-anchored at `label_y + DISPLAY_Y_OFFSET`; its
            // glyphs rise above that, so center the logo half a line-height up to
            // align it with the ticker line rather than the label's center.
            let ticker_center_y =
                label.label_y + SERIES_LABEL_DISPLAY_Y_OFFSET - SERIES_LABEL_DISPLAY_SIZE * 0.5;
            let position = Point::new(
                label_x + SERIES_LABEL_ICON_LEFT_PAD,
                ticker_center_y - size.height * 0.5,
            );
            frame.draw_svg(Rectangle::new(position, size), icon);
        }
    }

    frame.fill_text(canvas::Text {
        content: display,
        position: Point::new(text_x, label.label_y + SERIES_LABEL_DISPLAY_Y_OFFSET),
        color: label_text_color,
        size: iced::Pixels(SERIES_LABEL_DISPLAY_SIZE),
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Bottom,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });

    frame.fill_text(canvas::Text {
        content: performance,
        position: Point::new(text_x, label.label_y + SERIES_LABEL_VALUE_Y_OFFSET),
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

/// Width of the widest text line in a label, before left inset and right
/// padding are added by the caller.
fn series_text_width(display: &str, performance: &str) -> f32 {
    let display_w = display.chars().count() as f32 * SERIES_LABEL_DISPLAY_CHAR_W;
    let performance_w = performance.chars().count() as f32 * SERIES_LABEL_VALUE_CHAR_W;
    display_w.max(performance_w)
}

/// Long display names (canonical outcome labels run 50-80 chars) are
/// ellipsized to the narrow price-axis strip; the legend keeps the full text.
fn axis_display_label(display: &str, max_width: f32) -> String {
    let max_chars = (max_width / SERIES_LABEL_DISPLAY_CHAR_W).floor().max(0.0) as usize;
    ellipsized_text(display, max_chars)
}

pub(in crate::spaghetti::normalized) fn performance_label(pct: f64) -> String {
    format!("{pct:+.2}%")
}

#[cfg(test)]
mod tests;
