use super::series::{ChartPoint, prepare_chart_layout};
use crate::denomination::DisplayDenominationContext;
use crate::helpers::ease_out_cubic;
use crate::message::Message;

use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

mod tooltip;
use tooltip::draw_hover_state;
pub(super) use tooltip::tooltip_origin;

const GRID_ALPHA: f32 = 0.055;
const ZERO_LINE_ALPHA: f32 = 0.10;
const AREA_FILL_TOP_ALPHA: f32 = 0.24;
const AREA_FILL_LAYERS: usize = 64;
const AREA_FILL_MIN_FADE_RATIO: f32 = 0.55;
const AREA_FILL_MIN_FADE_PX: f32 = 120.0;
const LINE_WIDTH: f32 = 1.5;

// ---------------------------------------------------------------------------
// Summary Chart Canvas
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(super) struct JournalSummaryChart {
    pub(super) pnl_points: Vec<(u64, f64)>,
    pub(super) account_value_points: Vec<(u64, f64)>,
    pub(super) show_account_value: bool,
    pub(super) denomination: DisplayDenominationContext,
    pub(super) reveal_progress: f32,
}

impl canvas::Program<Message> for JournalSummaryChart {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        draw_journal_summary_chart(self, renderer, theme, bounds, cursor)
    }
}

fn draw_journal_summary_chart(
    chart: &JournalSummaryChart,
    renderer: &Renderer,
    theme: &Theme,
    bounds: Rectangle,
    cursor: iced::mouse::Cursor,
) -> Vec<canvas::Geometry> {
    let mut frame = canvas::Frame::new(renderer, bounds.size());
    frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);

    let Some(layout) = prepare_chart_layout(chart, bounds.width, bounds.height) else {
        return vec![frame.into_geometry()];
    };

    let (line_color, area_color) = journal_chart_colors(theme);
    let reveal_progress = ease_out_cubic(chart.reveal_progress.clamp(0.0, 1.0));
    let reveal_width = (bounds.width * reveal_progress).clamp(0.0, bounds.width);

    draw_grid(&mut frame, theme, bounds.size());
    draw_zero_line(&mut frame, theme, bounds.width, layout.zero_y);

    if reveal_width > 0.5 {
        frame.with_clip(
            Rectangle {
                x: 0.0,
                y: 0.0,
                width: (reveal_width + 8.0).min(bounds.width),
                height: bounds.height,
            },
            |frame| {
                draw_pnl_area(frame, &layout.pnl_points, bounds.height, area_color);

                if chart.show_account_value && !layout.account_value_points.is_empty() {
                    draw_series(
                        frame,
                        &layout.account_value_points,
                        account_value_line_color(theme),
                        1.4,
                        &[4.0, 5.0],
                    );
                }

                draw_pnl_series(frame, &layout.pnl_points, line_color);
            },
        );
    }

    if reveal_progress >= 0.995 {
        draw_hover_state(
            &mut frame,
            &layout,
            chart.show_account_value,
            &chart.denomination,
            theme,
            bounds,
            cursor,
        );
    }

    vec![frame.into_geometry()]
}

fn draw_grid(frame: &mut canvas::Frame, theme: &Theme, size: Size) {
    let color = Color {
        a: GRID_ALPHA,
        ..theme.palette().text
    };

    for i in 0..=5 {
        let y = size.height * i as f32 / 5.0;
        let path = canvas::Path::line(Point::new(0.0, y), Point::new(size.width, y));
        frame.stroke(
            &path,
            canvas::Stroke::default().with_color(color).with_width(1.0),
        );
    }

    for i in 1..6 {
        let x = size.width * i as f32 / 6.0;
        let path = canvas::Path::line(Point::new(x, 0.0), Point::new(x, size.height));
        frame.stroke(
            &path,
            canvas::Stroke::default().with_color(color).with_width(1.0),
        );
    }
}

fn draw_zero_line(frame: &mut canvas::Frame, theme: &Theme, width: f32, zero_y: f32) {
    let path = canvas::Path::line(Point::new(0.0, zero_y), Point::new(width, zero_y));
    frame.stroke(
        &path,
        canvas::Stroke::default()
            .with_color(Color {
                a: ZERO_LINE_ALPHA,
                ..theme.palette().text
            })
            .with_width(1.0),
    );
}

fn draw_pnl_area(frame: &mut canvas::Frame, points: &[ChartPoint], height: f32, color: Color) {
    if points.len() < 2 {
        return;
    }

    let Some(first) = points.first() else {
        return;
    };
    let Some(last) = points.last() else {
        return;
    };

    let top_y = points
        .iter()
        .map(|point| point.point.y)
        .fold(f32::INFINITY, f32::min);
    let bottom_y = height;
    if bottom_y <= top_y {
        return;
    }

    let mut area_points = Vec::with_capacity(points.len() + 2);
    area_points.push(Point::new(first.point.x, bottom_y));
    area_points.extend(points.iter().map(|point| point.point));
    area_points.push(Point::new(last.point.x, bottom_y));

    draw_line_area_fade(frame, &area_points, color, top_y, bottom_y, height);
}

fn draw_pnl_series(frame: &mut canvas::Frame, points: &[ChartPoint], color: Color) {
    match points {
        [] => {}
        [only] => draw_point_marker(frame, only.point, color, 2.5),
        points => {
            let line = line_path(points);
            draw_series_path(frame, &line, color, LINE_WIDTH, &[]);
        }
    }
}

fn draw_series(
    frame: &mut canvas::Frame,
    points: &[ChartPoint],
    color: Color,
    width: f32,
    dash_segments: &'static [f32],
) {
    match points {
        [] => {}
        [only] => draw_point_marker(frame, only.point, color, 2.0),
        points => {
            let path = line_path(points);
            draw_series_path(frame, &path, color, width, dash_segments);
        }
    }
}

fn draw_series_path(
    frame: &mut canvas::Frame,
    path: &canvas::Path,
    color: Color,
    width: f32,
    dash_segments: &'static [f32],
) {
    let mut stroke = canvas::Stroke::default()
        .with_color(color)
        .with_width(width)
        .with_line_cap(canvas::LineCap::Butt)
        .with_line_join(canvas::LineJoin::Miter);
    if !dash_segments.is_empty() {
        stroke.line_dash = canvas::stroke::LineDash {
            segments: dash_segments,
            offset: 0,
        };
    }
    frame.stroke(path, stroke);
}

fn line_path(points: &[ChartPoint]) -> canvas::Path {
    canvas::Path::new(|path| {
        let Some(first) = points.first() else {
            return;
        };
        path.move_to(first.point);
        for point in &points[1..] {
            path.line_to(point.point);
        }
    })
}

fn draw_line_area_fade(
    frame: &mut canvas::Frame,
    area_points: &[Point],
    accent: Color,
    top_y: f32,
    bottom_y: f32,
    height: f32,
) {
    let Some((start_y, end_y)) = line_area_fade_bounds(top_y, bottom_y, height) else {
        return;
    };
    let fade_h = end_y - start_y;
    if fade_h <= f32::EPSILON {
        return;
    }

    let color = Color {
        a: line_area_layer_alpha(),
        ..accent
    };

    for layer in 0..AREA_FILL_LAYERS {
        let t = (layer + 1) as f32 / (AREA_FILL_LAYERS + 1) as f32;
        let clip_y = start_y + fade_h * t;
        let clipped = clip_polygon_to_max_y(area_points, clip_y);
        fill_polygon(frame, &clipped, color);
    }
}

fn line_area_fade_bounds(top_y: f32, bottom_y: f32, height: f32) -> Option<(f32, f32)> {
    if !top_y.is_finite()
        || !bottom_y.is_finite()
        || !height.is_finite()
        || bottom_y <= top_y
        || height <= 0.0
    {
        return None;
    }

    let min_fade_h = (height * AREA_FILL_MIN_FADE_RATIO).max(AREA_FILL_MIN_FADE_PX.min(height));
    let start_y = top_y.min(bottom_y - min_fade_h).min(bottom_y - 1.0);
    let end_y = bottom_y.max(start_y + 1.0);

    Some((start_y, end_y))
}

fn line_area_layer_alpha() -> f32 {
    1.0 - (1.0 - AREA_FILL_TOP_ALPHA).powf(1.0 / AREA_FILL_LAYERS as f32)
}

fn clip_polygon_to_max_y(points: &[Point], max_y: f32) -> Vec<Point> {
    if points.len() < 3 || !max_y.is_finite() {
        return Vec::new();
    }

    let mut clipped = Vec::with_capacity(points.len() + 2);
    let mut previous = *points.last().unwrap_or(&Point::ORIGIN);
    let mut previous_inside = previous.y <= max_y;

    for current in points.iter().copied() {
        let current_inside = current.y <= max_y;
        if current_inside != previous_inside {
            clipped.push(segment_y_intersection(previous, current, max_y));
        }
        if current_inside {
            clipped.push(current);
        }

        previous = current;
        previous_inside = current_inside;
    }

    clipped
}

fn segment_y_intersection(start: Point, end: Point, y: f32) -> Point {
    let dy = end.y - start.y;
    if dy.abs() <= f32::EPSILON {
        return Point::new(start.x, y);
    }

    let t = ((y - start.y) / dy).clamp(0.0, 1.0);
    Point::new(start.x + (end.x - start.x) * t, y)
}

fn fill_polygon(frame: &mut canvas::Frame, points: &[Point], color: Color) {
    if points.len() < 3 {
        return;
    }

    let path = canvas::Path::new(|path| {
        path.move_to(points[0]);
        for point in &points[1..] {
            path.line_to(*point);
        }
        path.close();
    });
    frame.fill(&path, color);
}

fn draw_point_marker(frame: &mut canvas::Frame, center: Point, color: Color, half_size: f32) {
    frame.fill_rectangle(
        Point::new(center.x - half_size, center.y - half_size),
        Size::new(half_size * 2.0, half_size * 2.0),
        color,
    );
}

fn journal_chart_colors(theme: &Theme) -> (Color, Color) {
    (
        theme.palette().text,
        theme.extended_palette().primary.base.color,
    )
}

fn account_value_line_color(theme: &Theme) -> Color {
    Color {
        a: 0.42,
        ..theme.palette().primary
    }
}

pub(super) fn nearest_chart_point(points: &[ChartPoint], cursor_x: f32) -> Option<ChartPoint> {
    points.iter().copied().min_by(|left, right| {
        let left_dist = (left.point.x - cursor_x).abs();
        let right_dist = (right.point.x - cursor_x).abs();
        left_dist.total_cmp(&right_dist)
    })
}
