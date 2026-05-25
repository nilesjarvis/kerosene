use super::super::series::ChartLayout;
use super::nearest_chart_point;
use crate::denomination::DisplayDenominationContext;

use chrono::{DateTime, Utc};
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Size, Theme};

// ---------------------------------------------------------------------------
// Summary Chart Tooltip
// ---------------------------------------------------------------------------

const TOOLTIP_WIDTH: f32 = 210.0;
const TOOLTIP_HEIGHT: f32 = 46.0;

pub(super) fn draw_hover_state(
    frame: &mut canvas::Frame,
    layout: &ChartLayout,
    show_account_value: bool,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
    bounds: Rectangle,
    cursor: iced::mouse::Cursor,
) {
    let Some(cursor_pos) = cursor.position_in(bounds) else {
        return;
    };
    if cursor_pos.x < 0.0
        || cursor_pos.x > bounds.width
        || cursor_pos.y < 0.0
        || cursor_pos.y > bounds.height
    {
        return;
    }

    let Some(nearest_pnl) = nearest_chart_point(&layout.pnl_points, cursor_pos.x) else {
        return;
    };

    let v_line = canvas::Path::line(
        Point::new(nearest_pnl.point.x, 0.0),
        Point::new(nearest_pnl.point.x, bounds.height),
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
    frame.fill(&canvas::Path::circle(nearest_pnl.point, 2.8), Color::WHITE);

    let mut lines = vec![
        format_timestamp(nearest_pnl.timestamp_ms),
        format!(
            "PnL {}",
            denomination.format_signed_value(nearest_pnl.value, 2)
        ),
    ];

    if show_account_value
        && let Some(nearest_account) =
            nearest_chart_point(&layout.account_value_points, cursor_pos.x)
    {
        frame.fill(
            &canvas::Path::circle(nearest_account.point, 2.4),
            theme.palette().primary,
        );
        lines.push(format!(
            "Acct {}",
            denomination.format_value(nearest_account.value, 2)
        ));
    }

    let tooltip_height = if lines.len() > 2 {
        TOOLTIP_HEIGHT
    } else {
        TOOLTIP_HEIGHT - 12.0
    };
    let origin = tooltip_origin(
        nearest_pnl.point,
        bounds.width,
        bounds.height,
        Size::new(TOOLTIP_WIDTH, tooltip_height),
    );
    frame.fill_rectangle(
        origin,
        Size::new(TOOLTIP_WIDTH, tooltip_height),
        Color {
            a: 0.93,
            ..theme.extended_palette().background.strong.color
        },
    );
    frame.fill_text(canvas::Text {
        content: lines.join("\n"),
        position: Point::new(origin.x + 7.0, origin.y + 9.0),
        color: theme.palette().text,
        size: iced::Pixels(10.0),
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

pub(in crate::journal_views::summary::chart) fn tooltip_origin(
    point: Point,
    width: f32,
    height: f32,
    tooltip_size: Size,
) -> Point {
    let x = if point.x + tooltip_size.width + 10.0 > width {
        point.x - tooltip_size.width - 8.0
    } else {
        point.x + 8.0
    }
    .clamp(0.0, (width - tooltip_size.width).max(0.0));

    let y = if point.y + tooltip_size.height + 10.0 > height {
        point.y - tooltip_size.height - 8.0
    } else {
        point.y + 8.0
    }
    .clamp(0.0, (height - tooltip_size.height).max(0.0));

    Point::new(x, y)
}

fn format_timestamp(timestamp_ms: u64) -> String {
    i64::try_from(timestamp_ms)
        .ok()
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "UTC time unavailable".to_string())
}
