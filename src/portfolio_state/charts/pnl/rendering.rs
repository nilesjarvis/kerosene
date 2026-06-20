use crate::denomination::DisplayDenominationContext;
use crate::helpers::format_signed_percent_value;

use super::{
    PnlValueDisplayMode,
    series::{
        PNL_TOOLTIP_HEIGHT, PNL_TOOLTIP_WIDTH, PnlChartPoint, nearest_pnl_point,
        pnl_tooltip_origin, prepare_pnl_chart_layout,
    },
};
use chrono::{DateTime, Utc};
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

const AREA_POSITIVE_MAX_ALPHA: f32 = 0.34;
const AREA_NEGATIVE_MAX_ALPHA: f32 = 0.30;
const AREA_MID_RATIO: f32 = 0.36;
const AREA_EDGE_RATIO: f32 = 0.09;
const LINE_WIDTH: f32 = 1.7;
const ZERO_LINE_ALPHA: f32 = 0.16;
const END_DOT_RADIUS: f32 = 3.2;
const END_DOT_HALO_WIDTH: f32 = 1.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PnlAreaSide {
    Positive,
    Negative,
}

#[derive(Debug, Clone, PartialEq)]
struct PnlAreaSegment {
    side: PnlAreaSide,
    points: Vec<Point>,
}

pub(super) fn draw_portfolio_pnl_chart(
    points: &[(u64, f64)],
    value_mode: PnlValueDisplayMode,
    denomination: &DisplayDenominationContext,
    renderer: &Renderer,
    theme: &Theme,
    bounds: Rectangle,
    cursor: iced::mouse::Cursor,
) -> Vec<canvas::Geometry> {
    let mut frame = canvas::Frame::new(renderer, bounds.size());
    frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);

    let Some(layout) = prepare_pnl_chart_layout(points, bounds.width, bounds.height) else {
        return vec![frame.into_geometry()];
    };
    let zero_y = layout.zero_y;

    let positive_color = theme.palette().success;
    let negative_color = theme.palette().danger;

    draw_pnl_area(
        &mut frame,
        &layout.points,
        zero_y,
        positive_color,
        negative_color,
    );

    let zero_line = canvas::Path::line(Point::new(0.0, zero_y), Point::new(bounds.width, zero_y));
    let zero_dash = [2.0_f32, 3.0_f32];
    frame.stroke(
        &zero_line,
        canvas::Stroke {
            style: canvas::Style::Solid(Color {
                a: ZERO_LINE_ALPHA,
                ..theme.palette().text
            }),
            width: 1.0,
            line_cap: canvas::LineCap::Butt,
            line_join: canvas::LineJoin::Miter,
            line_dash: canvas::LineDash {
                segments: &zero_dash,
                offset: 0,
            },
        },
    );

    draw_pnl_line(
        &mut frame,
        &layout.points,
        zero_y,
        positive_color,
        negative_color,
    );

    draw_end_dot(
        &mut frame,
        &layout.points,
        positive_color,
        negative_color,
        theme,
    );

    if let Some(cursor_pos) = cursor.position_in(bounds)
        && cursor_pos.x >= 0.0
        && cursor_pos.x <= bounds.width
        && cursor_pos.y >= 0.0
        && cursor_pos.y <= bounds.height
        && let Some(nearest) = nearest_pnl_point(&layout.points, cursor_pos.x)
    {
        let nearest_point = nearest.point;
        let v_line = canvas::Path::line(
            Point::new(nearest_point.x, 0.0),
            Point::new(nearest_point.x, bounds.height),
        );
        frame.stroke(
            &v_line,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.20,
                    ..theme.palette().text
                })
                .with_width(1.0),
        );

        let marker = canvas::Path::circle(nearest_point, 2.8);
        frame.fill(&marker, theme.palette().text);

        let ts_label = i64::try_from(nearest.timestamp_ms)
            .ok()
            .and_then(DateTime::<Utc>::from_timestamp_millis)
            .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
            .unwrap_or_else(|| "UTC time unavailable".to_string());
        let pnl_label = match value_mode {
            PnlValueDisplayMode::Usd => {
                format!("PnL {}", denomination.format_signed_value(nearest.pnl, 2))
            }
            PnlValueDisplayMode::Percent => {
                format!("Performance {}", format_signed_percent_value(nearest.pnl))
            }
        };
        let label = format!("{}\n{}", ts_label, pnl_label);

        let pad = 6.0;
        let tooltip_origin = pnl_tooltip_origin(nearest_point, bounds.width, bounds.height);

        frame.fill_rectangle(
            tooltip_origin,
            Size::new(PNL_TOOLTIP_WIDTH, PNL_TOOLTIP_HEIGHT),
            Color {
                a: 0.92,
                ..theme.extended_palette().background.strong.color
            },
        );
        frame.fill_text(canvas::Text {
            content: label,
            position: Point::new(tooltip_origin.x + pad, tooltip_origin.y + 9.0),
            color: theme.palette().text,
            size: iced::Pixels(10.0),
            font: crate::app_fonts::monospace_font(),
            ..canvas::Text::default()
        });
    }

    vec![frame.into_geometry()]
}

fn draw_pnl_area(
    frame: &mut canvas::Frame,
    points: &[PnlChartPoint],
    zero_y: f32,
    positive_color: Color,
    negative_color: Color,
) {
    if points.len() < 2 {
        return;
    }

    let segments = pnl_area_segments(points, zero_y);
    if segments.is_empty() {
        return;
    }

    let width = frame.width();
    let height = frame.height();

    for segment in segments {
        let area = area_segment_path(&segment, zero_y);

        match segment.side {
            PnlAreaSide::Positive if zero_y > 0.0 => {
                let gradient = segment_area_gradient(
                    positive_color,
                    &segment,
                    zero_y,
                    AREA_POSITIVE_MAX_ALPHA,
                );
                frame.with_clip(
                    Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width,
                        height: zero_y,
                    },
                    |frame| frame.fill(&area, gradient),
                );
            }
            PnlAreaSide::Negative if zero_y < height => {
                let gradient = segment_area_gradient(
                    negative_color,
                    &segment,
                    zero_y,
                    AREA_NEGATIVE_MAX_ALPHA,
                );
                frame.with_clip(
                    Rectangle {
                        x: 0.0,
                        y: zero_y,
                        width,
                        height: height - zero_y,
                    },
                    |frame| frame.fill(&area, gradient),
                );
            }
            _ => {}
        }
    }
}

fn pnl_area_segments(points: &[PnlChartPoint], zero_y: f32) -> Vec<PnlAreaSegment> {
    let Some(first) = points.first().copied() else {
        return Vec::new();
    };

    let mut segments = Vec::new();
    let mut current_side = pnl_area_side(first.pnl);
    let mut current_points = vec![first.point];

    for pair in points.windows(2) {
        let previous = pair[0];
        let next = pair[1];
        let previous_side = pnl_area_side(previous.pnl);
        let next_side = pnl_area_side(next.pnl);

        if previous_side == next_side {
            current_points.push(next.point);
            continue;
        }

        let crossing = zero_crossing_point(previous, next, zero_y);
        current_points.push(crossing);
        push_area_segment(&mut segments, current_side, &mut current_points, zero_y);

        current_side = next_side;
        current_points.push(crossing);
        current_points.push(next.point);
    }

    push_area_segment(&mut segments, current_side, &mut current_points, zero_y);
    segments
}

fn push_area_segment(
    segments: &mut Vec<PnlAreaSegment>,
    side: PnlAreaSide,
    points: &mut Vec<Point>,
    zero_y: f32,
) {
    let has_area = points
        .iter()
        .any(|point| (point.y - zero_y).abs() > f32::EPSILON);
    if points.len() >= 2 && has_area {
        segments.push(PnlAreaSegment {
            side,
            points: std::mem::take(points),
        });
    } else {
        points.clear();
    }
}

fn pnl_area_side(pnl: f64) -> PnlAreaSide {
    if pnl >= 0.0 {
        PnlAreaSide::Positive
    } else {
        PnlAreaSide::Negative
    }
}

fn zero_crossing_point(previous: PnlChartPoint, next: PnlChartPoint, zero_y: f32) -> Point {
    let denominator = previous.pnl - next.pnl;
    let t = if denominator.abs() <= f64::EPSILON {
        0.5
    } else {
        (previous.pnl / denominator).clamp(0.0, 1.0)
    } as f32;

    Point::new(
        previous.point.x + (next.point.x - previous.point.x) * t,
        zero_y,
    )
}

fn area_segment_path(segment: &PnlAreaSegment, zero_y: f32) -> canvas::Path {
    canvas::Path::new(|builder| {
        if let Some(first) = segment.points.first() {
            builder.move_to(*first);
            for point in segment.points.iter().skip(1) {
                builder.line_to(*point);
            }
            if let Some(last) = segment.points.last() {
                builder.line_to(Point::new(last.x, zero_y));
                builder.line_to(Point::new(first.x, zero_y));
                builder.close();
            }
        }
    })
}

fn draw_pnl_line(
    frame: &mut canvas::Frame,
    points: &[PnlChartPoint],
    zero_y: f32,
    positive_color: Color,
    negative_color: Color,
) {
    let line = canvas::Path::new(|builder| {
        for (idx, chart_point) in points.iter().enumerate() {
            if idx == 0 {
                builder.move_to(chart_point.point);
            } else {
                builder.line_to(chart_point.point);
            }
        }
    });

    let width = frame.width();
    let height = frame.height();
    if zero_y > 0.0 {
        frame.with_clip(
            Rectangle {
                x: 0.0,
                y: 0.0,
                width,
                height: zero_y,
            },
            |frame| draw_pnl_line_stroke(frame, &line, positive_color),
        );
    }

    if zero_y < height {
        frame.with_clip(
            Rectangle {
                x: 0.0,
                y: zero_y,
                width,
                height: height - zero_y,
            },
            |frame| draw_pnl_line_stroke(frame, &line, negative_color),
        );
    }
}

fn draw_pnl_line_stroke(frame: &mut canvas::Frame, line: &canvas::Path, color: Color) {
    frame.stroke(
        line,
        canvas::Stroke::default()
            .with_color(color)
            .with_width(LINE_WIDTH)
            .with_line_cap(canvas::LineCap::Round)
            .with_line_join(canvas::LineJoin::Round),
    );
}

fn draw_end_dot(
    frame: &mut canvas::Frame,
    points: &[PnlChartPoint],
    positive_color: Color,
    negative_color: Color,
    theme: &Theme,
) {
    let Some(last) = points.last() else {
        return;
    };
    let dot_color = if last.pnl >= 0.0 {
        positive_color
    } else {
        negative_color
    };
    let dot = canvas::Path::circle(last.point, END_DOT_RADIUS);
    frame.fill(&dot, dot_color);
    // Halo straddling the dot edge so it reads against the panel surface.
    frame.stroke(
        &dot,
        canvas::Stroke::default()
            .with_color(theme.extended_palette().background.strong.color)
            .with_width(END_DOT_HALO_WIDTH),
    );
}

fn segment_area_gradient(
    color: Color,
    segment: &PnlAreaSegment,
    zero_y: f32,
    max_alpha: f32,
) -> canvas::gradient::Linear {
    let strong = Color {
        a: max_alpha,
        ..color
    };
    let mid = Color {
        a: max_alpha * AREA_MID_RATIO,
        ..color
    };
    let edge = Color {
        a: max_alpha * AREA_EDGE_RATIO,
        ..color
    };
    let clear = Color { a: 0.0, ..color };

    match segment.side {
        PnlAreaSide::Positive => {
            let top_y = segment
                .points
                .iter()
                .fold(zero_y, |top, point| top.min(point.y))
                .min(zero_y - 1.0);
            canvas::gradient::Linear::new(Point::new(0.0, top_y), Point::new(0.0, zero_y))
                .add_stop(0.0, strong)
                .add_stop(0.70, mid)
                .add_stop(0.92, edge)
                .add_stop(1.0, clear)
        }
        PnlAreaSide::Negative => {
            let bottom_y = segment
                .points
                .iter()
                .fold(zero_y, |bottom, point| bottom.max(point.y))
                .max(zero_y + 1.0);
            canvas::gradient::Linear::new(Point::new(0.0, zero_y), Point::new(0.0, bottom_y))
                .add_stop(0.0, clear)
                .add_stop(0.08, edge)
                .add_stop(0.30, mid)
                .add_stop(1.0, strong)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::assert_close_loose as assert_near;

    fn chart_point(x: f32, y: f32, pnl: f64) -> PnlChartPoint {
        PnlChartPoint {
            point: Point::new(x, y),
            timestamp_ms: x as u64,
            pnl,
        }
    }

    #[test]
    fn area_segments_split_at_zero_crossings() {
        let zero_y = 50.0;
        let points = vec![
            chart_point(0.0, 75.0, -10.0),
            chart_point(50.0, 25.0, 10.0),
            chart_point(100.0, 75.0, -10.0),
        ];

        let segments = pnl_area_segments(&points, zero_y);

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].side, PnlAreaSide::Negative);
        assert_eq!(segments[1].side, PnlAreaSide::Positive);
        assert_eq!(segments[2].side, PnlAreaSide::Negative);
        assert_near(segments[0].points[1].x, 25.0);
        assert_near(segments[0].points[1].y, zero_y);
        assert_near(segments[1].points[0].x, 25.0);
        assert_near(segments[1].points[2].x, 75.0);
        assert_near(segments[2].points[0].x, 75.0);
    }

    #[test]
    fn area_segments_keep_single_sided_series_contiguous() {
        let zero_y = 100.0;
        let points = vec![
            chart_point(0.0, 80.0, 1.0),
            chart_point(50.0, 40.0, 3.0),
            chart_point(100.0, 60.0, 2.0),
        ];

        let segments = pnl_area_segments(&points, zero_y);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].side, PnlAreaSide::Positive);
        assert_eq!(segments[0].points.len(), 3);
    }

    #[test]
    fn area_segments_ignore_baseline_only_runs() {
        let zero_y = 50.0;
        let points = vec![chart_point(0.0, 50.0, 0.0), chart_point(50.0, 50.0, 0.0)];

        assert!(pnl_area_segments(&points, zero_y).is_empty());
    }

    #[test]
    fn segment_gradients_use_local_vertical_extents() {
        let zero_y = 50.0;
        let points = [
            chart_point(0.0, 45.0, 1.0),
            chart_point(50.0, 10.0, 8.0),
            chart_point(100.0, 70.0, -2.0),
        ];
        let positive_segment = PnlAreaSegment {
            side: PnlAreaSide::Positive,
            points: vec![points[0].point, points[1].point],
        };
        let negative_segment = PnlAreaSegment {
            side: PnlAreaSide::Negative,
            points: vec![points[2].point],
        };

        let positive_gradient = segment_area_gradient(
            Color::WHITE,
            &positive_segment,
            zero_y,
            AREA_POSITIVE_MAX_ALPHA,
        );
        let negative_gradient = segment_area_gradient(
            Color::WHITE,
            &negative_segment,
            zero_y,
            AREA_NEGATIVE_MAX_ALPHA,
        );

        assert_near(positive_gradient.start.y, 10.0);
        assert_near(positive_gradient.end.y, zero_y);
        assert_near(negative_gradient.start.y, zero_y);
        assert_near(negative_gradient.end.y, 70.0);
    }
}
