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

const AREA_MAX_ALPHA: f32 = 0.22;
const AREA_MID_ALPHA: f32 = 0.08;
const AREA_EDGE_ALPHA: f32 = 0.02;
const AREA_ZERO_ALPHA: f32 = 0.0;
const LINE_HALO_ALPHA: f32 = 0.16;

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
    frame.stroke(
        &zero_line,
        canvas::Stroke::default()
            .with_color(Color {
                a: 0.18,
                ..theme.palette().text
            })
            .with_width(1.0),
    );

    draw_pnl_line(
        &mut frame,
        &layout.points,
        zero_y,
        positive_color,
        negative_color,
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
        frame.fill(&marker, Color::WHITE);

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
    let gradient = |color| area_gradient(color, min_y, zero_y, max_y);

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
            .with_color(Color {
                a: LINE_HALO_ALPHA,
                ..color
            })
            .with_width(5.0)
            .with_line_cap(canvas::LineCap::Round)
            .with_line_join(canvas::LineJoin::Round),
    );
    frame.stroke(
        line,
        canvas::Stroke::default()
            .with_color(color)
            .with_width(2.0)
            .with_line_cap(canvas::LineCap::Round)
            .with_line_join(canvas::LineJoin::Round),
    );
}

fn area_gradient(color: Color, min_y: f32, zero_y: f32, max_y: f32) -> canvas::gradient::Linear {
    let span = (max_y - min_y).max(1.0);
    let zero_offset = ((zero_y - min_y) / span).clamp(0.0, 1.0);
    let strong = Color {
        a: AREA_MAX_ALPHA,
        ..color
    };
    let mid = Color {
        a: AREA_MID_ALPHA,
        ..color
    };
    let edge = Color {
        a: AREA_EDGE_ALPHA,
        ..color
    };
    let clear = Color {
        a: AREA_ZERO_ALPHA,
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
