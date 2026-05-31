use super::series::{ChartPoint, prepare_chart_layout};
use crate::denomination::DisplayDenominationContext;
use crate::message::Message;

use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

mod tooltip;
use tooltip::draw_hover_state;
pub(super) use tooltip::tooltip_origin;

const PNL_AREA_MAX_ALPHA: f32 = 0.18;
const PNL_AREA_MID_ALPHA: f32 = 0.07;
const PNL_AREA_EDGE_ALPHA: f32 = 0.02;
const PNL_AREA_ZERO_ALPHA: f32 = 0.0;

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
    let positive_color = theme.palette().success;
    let negative_color = theme.palette().danger;
    draw_pnl_area(
        &mut frame,
        &layout.pnl_points,
        layout.zero_y,
        positive_color,
        negative_color,
    );
    draw_zero_line(&mut frame, theme, bounds.width, layout.zero_y);
    draw_signed_pnl_series(
        &mut frame,
        &layout.pnl_points,
        layout.zero_y,
        positive_color,
        negative_color,
    );

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
    positive_color: Color,
    negative_color: Color,
) {
    if points.len() < 2 {
        return;
    }

    let first_x = points
        .first()
        .map(|point| point.point.x)
        .unwrap_or_default();
    let last_x = points.last().map(|point| point.point.x).unwrap_or_default();
    let (min_y, max_y) = points
        .iter()
        .fold((zero_y, zero_y), |(min_y, max_y), point| {
            (min_y.min(point.point.y), max_y.max(point.point.y))
        });

    if (max_y - min_y).abs() <= f32::EPSILON {
        return;
    }

    let area = canvas::Path::new(|builder| {
        if let Some(first) = points.first() {
            builder.move_to(first.point);
            for point in points.iter().skip(1) {
                builder.line_to(point.point);
            }
            builder.line_to(Point::new(last_x, zero_y));
            builder.line_to(Point::new(first_x, zero_y));
            builder.close();
        }
    });

    let width = frame.width();
    let height = frame.height();
    let gradient = |color| pnl_area_gradient(color, min_y, zero_y, max_y);

    if zero_y > 0.0 {
        frame.with_clip(
            Rectangle {
                x: 0.0,
                y: 0.0,
                width,
                height: zero_y,
            },
            |frame| frame.fill(&area, gradient(positive_color)),
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
            |frame| frame.fill(&area, gradient(negative_color)),
        );
    }
}

fn draw_signed_pnl_series(
    frame: &mut canvas::Frame,
    points: &[ChartPoint],
    zero_y: f32,
    positive_color: Color,
    negative_color: Color,
) {
    match points {
        [] => {}
        [only] => {
            let color = if only.value >= 0.0 {
                positive_color
            } else {
                negative_color
            };
            let dot = canvas::Path::circle(only.point, 2.4);
            frame.fill(&dot, color);
        }
        points => {
            let line = canvas::Path::new(|path| {
                for (idx, point) in points.iter().enumerate() {
                    if idx == 0 {
                        path.move_to(point.point);
                    } else {
                        path.line_to(point.point);
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
                    |frame| draw_series_path(frame, &line, positive_color, 2.0, &[]),
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
                    |frame| draw_series_path(frame, &line, negative_color, 2.0, &[]),
                );
            }
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
            let path = canvas::Path::new(|path| {
                for (idx, point) in points.iter().enumerate() {
                    if idx == 0 {
                        path.move_to(point.point);
                    } else {
                        path.line_to(point.point);
                    }
                }
            });
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

fn pnl_area_gradient(
    color: Color,
    min_y: f32,
    zero_y: f32,
    max_y: f32,
) -> canvas::gradient::Linear {
    let span = (max_y - min_y).max(1.0);
    let zero_offset = ((zero_y - min_y) / span).clamp(0.0, 1.0);
    let strong = Color {
        a: PNL_AREA_MAX_ALPHA,
        ..color
    };
    let mid = Color {
        a: PNL_AREA_MID_ALPHA,
        ..color
    };
    let edge = Color {
        a: PNL_AREA_EDGE_ALPHA,
        ..color
    };
    let clear = Color {
        a: PNL_AREA_ZERO_ALPHA,
        ..color
    };

    let gradient = canvas::gradient::Linear::new(Point::new(0.0, min_y), Point::new(0.0, max_y));

    if zero_offset <= 0.02 {
        gradient
            .add_stop(0.0, clear)
            .add_stop(0.35, mid)
            .add_stop(1.0, strong)
    } else if zero_offset >= 0.98 {
        gradient
            .add_stop(0.0, strong)
            .add_stop(0.65, mid)
            .add_stop(1.0, clear)
    } else {
        let fade = zero_offset.min(1.0 - zero_offset).min(0.18);
        gradient
            .add_stop(0.0, strong)
            .add_stop((zero_offset - fade).max(0.0), edge)
            .add_stop(zero_offset, clear)
            .add_stop((zero_offset + fade).min(1.0), edge)
            .add_stop(1.0, strong)
    }
}

fn nearest_chart_point(points: &[ChartPoint], cursor_x: f32) -> Option<ChartPoint> {
    points.iter().copied().min_by(|left, right| {
        let left_dist = (left.point.x - cursor_x).abs();
        let right_dist = (right.point.x - cursor_x).abs();
        left_dist.total_cmp(&right_dist)
    })
}
