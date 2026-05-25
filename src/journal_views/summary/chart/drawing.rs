use super::series::{ChartPoint, prepare_chart_layout};
use crate::denomination::DisplayDenominationContext;
use crate::message::Message;

use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

mod tooltip;
use tooltip::draw_hover_state;
pub(super) use tooltip::tooltip_origin;

// ---------------------------------------------------------------------------
// Summary Chart Canvas
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(super) struct JournalSummaryChart {
    pub(super) pnl_points: Vec<(u64, f64)>,
    pub(super) account_value_points: Vec<(u64, f64)>,
    pub(super) show_account_value: bool,
    pub(super) denomination: DisplayDenominationContext,
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

    draw_grid(&mut frame, theme, bounds.size());
    draw_zero_line(&mut frame, theme, bounds.width, layout.zero_y);

    let pnl_color = match chart.pnl_points.last().map(|(_, value)| *value) {
        Some(value) if value < 0.0 => theme.palette().danger,
        _ => theme.palette().success,
    };
    draw_pnl_area(
        &mut frame,
        &layout.pnl_points,
        layout.zero_y,
        pnl_color,
        bounds.height,
    );
    draw_series(&mut frame, &layout.pnl_points, pnl_color, 2.0, &[]);

    if chart.show_account_value && !layout.account_value_points.is_empty() {
        draw_series(
            &mut frame,
            &layout.account_value_points,
            theme.palette().primary,
            1.5,
            &[5.0, 4.0],
        );
    }

    draw_hover_state(
        &mut frame,
        &layout,
        chart.show_account_value,
        &chart.denomination,
        theme,
        bounds,
        cursor,
    );

    vec![frame.into_geometry()]
}

fn draw_grid(frame: &mut canvas::Frame, theme: &Theme, size: Size) {
    for fraction in [0.25_f32, 0.5, 0.75] {
        let y = size.height * fraction;
        let path = canvas::Path::line(Point::new(0.0, y), Point::new(size.width, y));
        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.08,
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
                a: 0.20,
                ..theme.palette().text
            })
            .with_width(1.0),
    );
}

fn draw_pnl_area(
    frame: &mut canvas::Frame,
    points: &[ChartPoint],
    zero_y: f32,
    color: Color,
    height: f32,
) {
    if points.len() < 2 {
        return;
    }

    for segment in points.windows(2) {
        let p1 = segment[0].point;
        let p2 = segment[1].point;
        let top = p1.y.min(p2.y).min(zero_y);
        let depth = (zero_y - top).abs();
        let alpha = (0.05 + (depth / height) * 0.14).clamp(0.05, 0.18);
        let fill = Color { a: alpha, ..color };
        let poly = canvas::Path::new(|builder| {
            builder.move_to(Point::new(p1.x, zero_y));
            builder.line_to(p1);
            builder.line_to(p2);
            builder.line_to(Point::new(p2.x, zero_y));
            builder.close();
        });
        frame.fill(&poly, fill);
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
            let mut path = canvas::path::Builder::new();
            for (idx, point) in points.iter().enumerate() {
                if idx == 0 {
                    path.move_to(point.point);
                } else {
                    path.line_to(point.point);
                }
            }

            let mut stroke = canvas::Stroke::default()
                .with_color(color)
                .with_width(width);
            if !dash_segments.is_empty() {
                stroke.line_dash = canvas::stroke::LineDash {
                    segments: dash_segments,
                    offset: 0,
                };
            }
            frame.stroke(&path.build(), stroke);
        }
    }
}

fn nearest_chart_point(points: &[ChartPoint], cursor_x: f32) -> Option<ChartPoint> {
    points.iter().copied().min_by(|left, right| {
        let left_dist = (left.point.x - cursor_x).abs();
        let right_dist = (right.point.x - cursor_x).abs();
        left_dist.total_cmp(&right_dist)
    })
}
