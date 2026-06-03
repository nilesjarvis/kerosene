use super::series::{ChartPoint, prepare_chart_layout};
use crate::denomination::DisplayDenominationContext;
use crate::message::Message;

use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

mod tooltip;
use tooltip::draw_hover_state;
pub(super) use tooltip::tooltip_origin;

const CARD_RADIUS: f32 = 12.0;
const CARD_BORDER_ALPHA: f32 = 0.16;
const GRID_ALPHA: f32 = 0.055;
const ZERO_LINE_ALPHA: f32 = 0.16;
const PNL_AREA_TOP_ALPHA: f32 = 0.26;
const PNL_AREA_MID_ALPHA: f32 = 0.10;
const PNL_AREA_BOTTOM_ALPHA: f32 = 0.0;

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
    draw_card_backdrop(&mut frame, theme, bounds.size());

    let Some(layout) = prepare_chart_layout(chart, bounds.width, bounds.height) else {
        return vec![frame.into_geometry()];
    };

    let mint = journal_chart_mint();
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
                draw_pnl_area(frame, &layout.pnl_points, bounds.height, mint);

                if chart.show_account_value && !layout.account_value_points.is_empty() {
                    draw_series(
                        frame,
                        &layout.account_value_points,
                        account_value_line_color(theme),
                        1.4,
                        &[4.0, 5.0],
                    );
                }

                draw_pnl_series_glow(frame, &layout.pnl_points, mint);
            },
        );

        if let Some(marker) = point_at_x(&layout.pnl_points, reveal_width) {
            draw_reveal_marker(&mut frame, marker, mint);
        }
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

fn draw_card_backdrop(frame: &mut canvas::Frame, theme: &Theme, size: Size) {
    let rect = canvas::Path::rounded_rectangle(Point::ORIGIN, size, CARD_RADIUS.into());
    frame.fill(
        &rect,
        Color {
            a: 0.86,
            ..theme.extended_palette().background.base.color
        },
    );
    frame.stroke(
        &rect,
        canvas::Stroke::default()
            .with_color(Color {
                a: CARD_BORDER_ALPHA,
                ..journal_chart_mint()
            })
            .with_width(1.0),
    );
}

fn draw_grid(frame: &mut canvas::Frame, theme: &Theme, size: Size) {
    for fraction in [0.25_f32, 0.5, 0.75] {
        let y = size.height * fraction;
        let path = canvas::Path::line(Point::new(0.0, y), Point::new(size.width, y));
        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_color(Color {
                    a: GRID_ALPHA,
                    ..theme.palette().text
                })
                .with_width(1.0),
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

    let area = smooth_area_path(points, height);
    frame.fill(&area, pnl_area_gradient(color, height));
}

fn draw_pnl_series_glow(frame: &mut canvas::Frame, points: &[ChartPoint], color: Color) {
    match points {
        [] => {}
        [only] => draw_reveal_marker(frame, only.point, color),
        points => {
            let line = smooth_line_path(points);
            draw_series_path(frame, &line, Color { a: 0.10, ..color }, 11.0, &[]);
            draw_series_path(frame, &line, Color { a: 0.18, ..color }, 6.0, &[]);
            draw_series_path(frame, &line, Color { a: 0.38, ..color }, 3.2, &[]);
            draw_series_path(frame, &line, color, 2.0, &[]);
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
        [only] => {
            let dot = canvas::Path::circle(only.point, 2.4);
            frame.fill(&dot, color);
        }
        points => {
            let path = smooth_line_path(points);
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
        .with_line_cap(canvas::LineCap::Round)
        .with_line_join(canvas::LineJoin::Round);
    if !dash_segments.is_empty() {
        stroke.line_dash = canvas::stroke::LineDash {
            segments: dash_segments,
            offset: 0,
        };
    }
    frame.stroke(path, stroke);
}

fn smooth_line_path(points: &[ChartPoint]) -> canvas::Path {
    canvas::Path::new(|path| {
        let Some(first) = points.first() else {
            return;
        };
        path.move_to(first.point);
        add_smoothed_points(path, points);
    })
}

fn smooth_area_path(points: &[ChartPoint], baseline_y: f32) -> canvas::Path {
    canvas::Path::new(|path| {
        let Some(first) = points.first() else {
            return;
        };
        let Some(last) = points.last() else {
            return;
        };

        path.move_to(first.point);
        add_smoothed_points(path, points);
        path.line_to(Point::new(last.point.x, baseline_y));
        path.line_to(Point::new(first.point.x, baseline_y));
        path.close();
    })
}

fn add_smoothed_points(path: &mut canvas::path::Builder, points: &[ChartPoint]) {
    if points.len() < 2 {
        return;
    }

    for pair in points.windows(2) {
        let previous = pair[0].point;
        let current = pair[1].point;
        let mid = Point::new(
            previous.x + (current.x - previous.x) * 0.5,
            previous.y + (current.y - previous.y) * 0.5,
        );
        path.quadratic_curve_to(previous, mid);
    }

    if let Some(last) = points.last() {
        path.line_to(last.point);
    }
}

fn draw_reveal_marker(frame: &mut canvas::Frame, point: Point, color: Color) {
    let glow = canvas::Path::circle(point, 6.0);
    frame.fill(&glow, Color { a: 0.16, ..color });
    let marker = canvas::Path::circle(point, 2.7);
    frame.fill(&marker, Color::WHITE);
    let inner = canvas::Path::circle(point, 1.5);
    frame.fill(&inner, color);
}

fn point_at_x(points: &[ChartPoint], x: f32) -> Option<Point> {
    let first = points.first()?;
    if x <= first.point.x {
        return Some(first.point);
    }

    for pair in points.windows(2) {
        let left = pair[0].point;
        let right = pair[1].point;
        if x <= right.x {
            let span = (right.x - left.x).max(f32::EPSILON);
            let ratio = ((x - left.x) / span).clamp(0.0, 1.0);
            return Some(Point::new(
                left.x + (right.x - left.x) * ratio,
                left.y + (right.y - left.y) * ratio,
            ));
        }
    }

    points.last().map(|point| point.point)
}

fn pnl_area_gradient(color: Color, height: f32) -> canvas::gradient::Linear {
    canvas::gradient::Linear::new(Point::new(0.0, 0.0), Point::new(0.0, height.max(1.0)))
        .add_stop(
            0.0,
            Color {
                a: PNL_AREA_TOP_ALPHA,
                ..color
            },
        )
        .add_stop(
            0.45,
            Color {
                a: PNL_AREA_MID_ALPHA,
                ..color
            },
        )
        .add_stop(
            1.0,
            Color {
                a: PNL_AREA_BOTTOM_ALPHA,
                ..color
            },
        )
}

fn journal_chart_mint() -> Color {
    Color {
        r: 0.16,
        g: 0.94,
        b: 0.78,
        a: 1.0,
    }
}

fn account_value_line_color(theme: &Theme) -> Color {
    Color {
        a: 0.42,
        ..theme.palette().primary
    }
}

fn ease_out_cubic(progress: f32) -> f32 {
    1.0 - (1.0 - progress).powi(3)
}

fn nearest_chart_point(points: &[ChartPoint], cursor_x: f32) -> Option<ChartPoint> {
    points.iter().copied().min_by(|left, right| {
        let left_dist = (left.point.x - cursor_x).abs();
        let right_dist = (right.point.x - cursor_x).abs();
        left_dist.total_cmp(&right_dist)
    })
}
