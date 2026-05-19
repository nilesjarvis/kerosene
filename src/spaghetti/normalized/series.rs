use super::NormalizedRenderContext;
use crate::helpers::text_color_for_bg;
use crate::spaghetti::{ComparisonColorMode, Series};
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point, Size, Theme};

// ---------------------------------------------------------------------------
// Normalized Series
// ---------------------------------------------------------------------------

const SERIES_LABEL_HEIGHT: f32 = 34.0;
const SERIES_LABEL_GAP: f32 = 4.0;
const SERIES_LABEL_MARGIN: f32 = 2.0;
const SERIES_LABEL_X_OFFSET: f32 = 12.0;
const SERIES_LABEL_PADDING_X: f32 = 10.0;
const SERIES_LABEL_DISPLAY_Y_OFFSET: f32 = -2.0;
const SERIES_LABEL_VALUE_Y_OFFSET: f32 = 1.0;
const SERIES_LABEL_CONNECTOR_SPAN: f32 = 22.0;
const SERIES_LABEL_DASH: f32 = 1.0;
const SERIES_LABEL_GAP_DASH: f32 = 4.0;
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

pub(super) fn draw_session_start_line(
    frame: &mut canvas::Frame,
    ctx: &NormalizedRenderContext<'_>,
    ts_to_x: &impl Fn(u64) -> f32,
    base_timestamp: Option<u64>,
) {
    if let Some(base_ts) = base_timestamp {
        let base_x = ts_to_x(base_ts);
        if base_x >= 0.0 && base_x <= ctx.chart_w {
            let dash_len: f32 = 4.0;
            let gap_len: f32 = 3.0;
            let mut y = 0.0_f32;
            while y < ctx.chart_h {
                let end = (y + dash_len).min(ctx.chart_h);
                let seg = canvas::Path::line(Point::new(base_x, y), Point::new(base_x, end));
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
    }
}

pub(super) fn draw_series_lines(
    frame: &mut canvas::Frame,
    ctx: &NormalizedRenderContext<'_>,
    series_data: &[(&Series, Vec<(f32, f64)>)],
    pct_to_y: &impl Fn(f64) -> f32,
    color_mode: ComparisonColorMode,
) {
    for (series, points) in series_data {
        if points.len() < 2 {
            continue;
        }
        let mut path = canvas::path::Builder::new();
        for (idx, (x, pct)) in points.iter().enumerate() {
            let px = x.clamp(-10.0, ctx.chart_w + 10.0);
            let py = pct_to_y(*pct).clamp(-50.0, ctx.chart_h + 50.0);
            if idx == 0 {
                path.move_to(Point::new(px, py));
            } else {
                path.line_to(Point::new(px, py));
            }
        }
        frame.stroke(
            &path.build(),
            canvas::Stroke::default()
                .with_color(series_render_color(ctx.theme, color_mode, series))
                .with_width(1.5),
        );
    }
}

pub(super) fn draw_series_labels(
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
        font: iced::Font::MONOSPACE,
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
        font: iced::Font::MONOSPACE,
        ..canvas::Text::default()
    });
}

fn series_label_width(display: &str, performance: &str) -> f32 {
    let display_w = display.len() as f32 * SERIES_LABEL_DISPLAY_CHAR_W;
    let performance_w = performance.len() as f32 * SERIES_LABEL_VALUE_CHAR_W;
    display_w.max(performance_w) + SERIES_LABEL_PADDING_X * 2.0
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SeriesLabelAnchor {
    index: usize,
    y: f32,
}

fn stack_series_label_positions(
    mut anchors: Vec<SeriesLabelAnchor>,
    chart_h: f32,
) -> Vec<Option<f32>> {
    if anchors.is_empty() {
        return Vec::new();
    }

    let slot_count = anchors.iter().map(|anchor| anchor.index).max().unwrap_or(0) + 1;
    let mut slots = vec![None; slot_count];
    if chart_h <= 0.0 || !chart_h.is_finite() {
        return slots;
    }

    anchors.sort_by(|a, b| a.y.total_cmp(&b.y).then_with(|| a.index.cmp(&b.index)));

    let min_y = SERIES_LABEL_HEIGHT * 0.5 + SERIES_LABEL_MARGIN;
    let max_y = (chart_h - SERIES_LABEL_HEIGHT * 0.5 - SERIES_LABEL_MARGIN).max(min_y);
    let step = SERIES_LABEL_HEIGHT + SERIES_LABEL_GAP;
    let mut positions = Vec::with_capacity(anchors.len());
    let mut next_y = min_y;

    for anchor in anchors {
        let desired_y = anchor.y.clamp(min_y, max_y);
        let label_y = desired_y.max(next_y);
        positions.push((anchor.index, label_y));
        next_y = label_y + step;
    }

    if positions
        .last()
        .is_some_and(|(_, label_y)| *label_y > max_y)
    {
        let mut next_y = max_y;
        for (_, label_y) in positions.iter_mut().rev() {
            *label_y = (*label_y).min(next_y);
            next_y = *label_y - step;
        }

        if let Some((_, first_y)) = positions.first()
            && *first_y < min_y
        {
            let shift = min_y - *first_y;
            for (_, label_y) in &mut positions {
                *label_y += shift;
            }
        }
    }

    for (index, label_y) in positions {
        if let Some(slot) = slots.get_mut(index) {
            *slot = Some(label_y);
        }
    }
    slots
}

fn stroke_segmented_hline_range(
    frame: &mut canvas::Frame,
    start_x: f32,
    end_x: f32,
    y: f32,
    color: Color,
) {
    if end_x <= start_x || !start_x.is_finite() || !end_x.is_finite() || !y.is_finite() {
        return;
    }

    let mut has_segment = false;
    let line = canvas::Path::new(|path| {
        let mut x = start_x;
        while x < end_x {
            let end = (x + SERIES_LABEL_DASH).min(end_x);
            if end > x {
                path.move_to(Point::new(x, y));
                path.line_to(Point::new(end, y));
                has_segment = true;
            }
            x += SERIES_LABEL_DASH + SERIES_LABEL_GAP_DASH;
        }
    });

    if has_segment {
        frame.stroke(
            &line,
            canvas::Stroke::default().with_color(color).with_width(1.0),
        );
    }
}

fn stroke_segmented_label_connector(
    frame: &mut canvas::Frame,
    start: Point,
    control: Point,
    end: Point,
    color: Color,
) {
    let mut has_segment = false;
    let connector = canvas::Path::new(|path| {
        has_segment = append_segmented_quadratic_curve(path, start, control, end);
    });

    if has_segment {
        frame.stroke(
            &connector,
            canvas::Stroke::default().with_color(color).with_width(1.0),
        );
    }
}

fn append_segmented_quadratic_curve(
    path: &mut canvas::path::Builder,
    start: Point,
    control: Point,
    end: Point,
) -> bool {
    const SAMPLES: usize = 14;

    if !valid_point(start) || !valid_point(control) || !valid_point(end) {
        return false;
    }

    let stride = SERIES_LABEL_DASH + SERIES_LABEL_GAP_DASH;
    let mut next_dash_start = 0.0;
    let mut previous = start;
    let mut previous_distance = 0.0;
    let mut has_segment = false;

    for sample in 1..=SAMPLES {
        let t = sample as f32 / SAMPLES as f32;
        let current = quadratic_point(start, control, end, t);
        let segment_len = distance(previous, current);
        if segment_len > 0.0 && segment_len.is_finite() {
            let segment_start_distance = previous_distance;
            let segment_end_distance = previous_distance + segment_len;

            while next_dash_start < segment_end_distance {
                let dash_start = next_dash_start.max(segment_start_distance);
                let dash_end = (next_dash_start + SERIES_LABEL_DASH).min(segment_end_distance);
                if dash_end > dash_start {
                    let start_t = (dash_start - segment_start_distance) / segment_len;
                    let end_t = (dash_end - segment_start_distance) / segment_len;
                    path.move_to(lerp_point(previous, current, start_t));
                    path.line_to(lerp_point(previous, current, end_t));
                    has_segment = true;
                }
                next_dash_start += stride;
            }

            previous_distance = segment_end_distance;
        }
        previous = current;
    }

    has_segment
}

fn quadratic_point(start: Point, control: Point, end: Point, t: f32) -> Point {
    let inv_t = 1.0 - t;
    let start_weight = inv_t * inv_t;
    let control_weight = 2.0 * inv_t * t;
    let end_weight = t * t;
    Point::new(
        start.x * start_weight + control.x * control_weight + end.x * end_weight,
        start.y * start_weight + control.y * control_weight + end.y * end_weight,
    )
}

fn lerp_point(start: Point, end: Point, t: f32) -> Point {
    Point::new(
        start.x + (end.x - start.x) * t,
        start.y + (end.y - start.y) * t,
    )
}

fn distance(start: Point, end: Point) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    (dx * dx + dy * dy).sqrt()
}

fn valid_point(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

pub(super) fn draw_legend(
    frame: &mut canvas::Frame,
    theme: &Theme,
    color_mode: ComparisonColorMode,
    series_data: &[(&Series, Vec<(f32, f64)>)],
) {
    let mut legend_y = 8.0_f32;
    for (series, points) in series_data {
        let color = series_render_color(theme, color_mode, series);
        frame.fill_rectangle(Point::new(8.0, legend_y), Size::new(8.0, 8.0), color);
        frame.fill_text(canvas::Text {
            content: legend_label(&series.display, points),
            position: Point::new(20.0, legend_y + 4.0),
            color,
            size: iced::Pixels(10.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: iced::Font::MONOSPACE,
            ..canvas::Text::default()
        });
        legend_y += 14.0;
    }
}

fn series_render_color(theme: &Theme, color_mode: ComparisonColorMode, series: &Series) -> Color {
    match color_mode {
        ComparisonColorMode::Multi => series.color,
        ComparisonColorMode::Single => crate::spaghetti::SpaghettiCanvas::single_color(theme),
    }
}

fn performance_label(pct: f64) -> String {
    format!("{pct:+.2}%")
}

fn legend_label(display: &str, points: &[(f32, f64)]) -> String {
    points
        .last()
        .map(|(_, pct)| format!("{display} {pct:+.2}%"))
        .unwrap_or_else(|| format!("{display} --"))
}

#[cfg(test)]
mod tests {
    use super::{SeriesLabelAnchor, legend_label, performance_label, stack_series_label_positions};

    #[test]
    fn legend_label_marks_empty_series_unavailable() {
        assert_eq!(legend_label("BTC", &[]), "BTC --");
    }

    #[test]
    fn legend_label_formats_latest_percent() {
        assert_eq!(legend_label("BTC", &[(1.0, -2.5)]), "BTC -2.50%");
    }

    #[test]
    fn performance_label_formats_signed_percent() {
        assert_eq!(performance_label(3.456), "+3.46%");
        assert_eq!(performance_label(-0.25), "-0.25%");
    }

    #[test]
    fn series_label_stack_separates_nearby_labels() {
        let positions = stack_series_label_positions(
            vec![
                SeriesLabelAnchor { index: 0, y: 40.0 },
                SeriesLabelAnchor { index: 1, y: 42.0 },
                SeriesLabelAnchor { index: 2, y: 43.0 },
            ],
            240.0,
        );

        assert_eq!(positions[0], Some(40.0));
        assert_eq!(positions[1], Some(78.0));
        assert_eq!(positions[2], Some(116.0));
    }

    #[test]
    fn series_label_stack_stays_inside_available_height_when_possible() {
        let positions = stack_series_label_positions(
            vec![
                SeriesLabelAnchor { index: 0, y: 92.0 },
                SeriesLabelAnchor { index: 1, y: 94.0 },
            ],
            100.0,
        );

        assert_eq!(positions[0], Some(43.0));
        assert_eq!(positions[1], Some(81.0));
    }
}
