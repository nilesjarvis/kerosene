use crate::denomination::DisplayDenominationContext;
use crate::helpers::format_signed_percent_value;

use super::{
    PnlValueDisplayMode,
    series::{
        PNL_TOOLTIP_HEIGHT, PNL_TOOLTIP_WIDTH, nearest_pnl_point, pnl_tooltip_origin,
        prepare_pnl_chart_layout,
    },
};
use chrono::{DateTime, Utc};
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

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

    let mut path = canvas::path::Builder::new();
    for (idx, chart_point) in layout.points.iter().enumerate() {
        let p = chart_point.point;
        if idx == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    let color = match points.last().map(|(_, value)| *value) {
        Some(value) if value < 0.0 => theme.palette().danger,
        _ => theme.palette().success,
    };
    frame.stroke(
        &path.build(),
        canvas::Stroke::default().with_color(color).with_width(2.0),
    );

    if layout.points.len() >= 2 {
        for segment in layout.points.windows(2) {
            let p1 = segment[0].point;
            let p2 = segment[1].point;
            let top = p1.y.min(p2.y).min(zero_y);
            let depth = (zero_y - top).abs();
            let alpha = (0.05 + (depth / bounds.height) * 0.18).clamp(0.05, 0.24);
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
