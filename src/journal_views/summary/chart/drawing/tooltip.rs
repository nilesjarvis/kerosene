use super::super::series::ChartLayout;
use super::nearest_chart_point;
use crate::journal_views::style::tooltip_origin;

use super::JournalSummaryChart;
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
    chart: &JournalSummaryChart,
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
    draw_hover_marker(frame, nearest_pnl.point, Color::WHITE, 2.8);

    let mut lines = vec![
        chart.zone.format(
            nearest_pnl.timestamp_ms,
            "%Y-%m-%d %H:%M %Z",
            "Time unavailable",
        ),
        format!(
            "PnL {}",
            chart.denomination.format_signed_value(nearest_pnl.value, 2)
        ),
    ];

    if chart.show_account_value
        && let Some(nearest_account) =
            nearest_chart_point(&layout.account_value_points, cursor_pos.x)
    {
        draw_hover_marker(frame, nearest_account.point, theme.palette().primary, 2.4);
        lines.push(format!(
            "Acct {}",
            chart.denomination.format_value(nearest_account.value, 2)
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

fn draw_hover_marker(frame: &mut canvas::Frame, point: Point, color: Color, half_size: f32) {
    frame.fill_rectangle(
        Point::new(point.x - half_size, point.y - half_size),
        Size::new(half_size * 2.0, half_size * 2.0),
        color,
    );
}
