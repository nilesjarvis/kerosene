use super::{
    BAR_RADIUS, BAR_THICKNESS, BULLET_GUTTER, BULLET_HEIGHT, BULLET_WIDTH, CONF_FULL, LABEL_GUTTER,
    LaneRow, MIN_CONF, N_GUTTER, ROW_HEIGHT, SECTION_HEADER_HEIGHT, TOOLTIP_HEIGHT, TOOLTIP_WIDTH,
    VALUE_GUTTER,
};
use crate::helpers;

use iced::widget::canvas::{self, Frame, Path, Stroke, Text};
use iced::{Color, Point, Size, Theme};

pub(super) struct LaneLayout {
    pub(super) width: f32,
    pub(super) bar_left: f32,
    pub(super) mid_x: f32,
    pub(super) half_w: f32,
    pub(super) value_right: Option<f32>,
    pub(super) bullet_left: f32,
    pub(super) n_right: Option<f32>,
}

impl LaneLayout {
    pub(super) fn new(size: Size, compact: bool) -> Self {
        let width = size.width;
        let bar_left = LABEL_GUTTER;
        let (right_block, value_right, n_right, bullet_left) = if compact {
            (BULLET_GUTTER, None, None, width - BULLET_WIDTH - 8.0)
        } else {
            (
                VALUE_GUTTER + BULLET_GUTTER + N_GUTTER,
                Some(width - N_GUTTER - BULLET_GUTTER - 6.0),
                Some(width - 6.0),
                width - N_GUTTER - BULLET_GUTTER + (BULLET_GUTTER - BULLET_WIDTH) * 0.5,
            )
        };
        let bar_right = (width - right_block).max(bar_left + 8.0);
        let mid_x = (bar_left + bar_right) * 0.5;
        let half_w = ((bar_right - bar_left) * 0.5 - 3.0).max(1.0);
        Self {
            width,
            bar_left,
            mid_x,
            half_w,
            value_right,
            bullet_left: bullet_left.max(bar_left),
            n_right,
        }
    }
}

pub(super) fn draw_section_header(
    frame: &mut Frame,
    theme: &Theme,
    label: &str,
    layout: &LaneLayout,
    y: f32,
) {
    let weak = theme.extended_palette().background.weak.text;
    let center_y = y + SECTION_HEADER_HEIGHT * 0.5;
    frame.fill_text(Text {
        content: label.to_string(),
        position: Point::new(6.0, center_y),
        color: weak,
        size: iced::Pixels(9.0),
        align_y: iced::alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..Default::default()
    });
    let rule = Path::line(
        Point::new(layout.bar_left, center_y + 0.5),
        Point::new(layout.width - 6.0, center_y + 0.5),
    );
    frame.stroke(
        &rule,
        Stroke::default()
            .with_color(Color {
                a: 0.12,
                ..theme.palette().text
            })
            .with_width(1.0),
    );
}

pub(super) fn draw_lane_row(
    frame: &mut Frame,
    theme: &Theme,
    layout: &LaneLayout,
    row: &LaneRow,
    scale_max: f32,
    y: f32,
    hovered: bool,
) {
    let weak = theme.extended_palette().background.weak.text;
    let center_y = y + ROW_HEIGHT * 0.5;

    if row.is_best {
        let marker = Path::rounded_rectangle(
            Point::new(2.0, center_y - BAR_THICKNESS * 0.5),
            Size::new(2.0, BAR_THICKNESS),
            1.0.into(),
        );
        frame.fill(&marker, theme.palette().primary);
    }

    let label_color = if row.is_best {
        theme.palette().text
    } else if row.sample_count > 0 {
        Color {
            a: 0.85,
            ..theme.palette().text
        }
    } else {
        weak
    };
    frame.fill_text(Text {
        content: row.label.to_string(),
        position: Point::new(8.0, center_y),
        color: label_color,
        size: iced::Pixels(10.0),
        align_y: iced::alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..Default::default()
    });

    if row.sample_count == 0 {
        draw_absent_dots(frame, theme, layout, center_y);
        if let Some(value_right) = layout.value_right {
            frame.fill_text(Text {
                content: "\u{2014}".to_string(),
                position: Point::new(value_right, center_y),
                color: weak,
                size: iced::Pixels(9.0),
                align_x: iced::alignment::Horizontal::Right.into(),
                align_y: iced::alignment::Vertical::Center,
                font: crate::app_fonts::monospace_font(),
                ..Default::default()
            });
        }
        if let Some(n_right) = layout.n_right {
            frame.fill_text(Text {
                content: "n0".to_string(),
                position: Point::new(n_right, center_y),
                color: weak,
                size: iced::Pixels(9.0),
                align_x: iced::alignment::Horizontal::Right.into(),
                align_y: iced::alignment::Vertical::Center,
                font: crate::app_fonts::monospace_font(),
                ..Default::default()
            });
        }
        return;
    }

    let confidence = if hovered {
        1.0
    } else {
        (row.sample_count as f32 / CONF_FULL).clamp(MIN_CONF, 1.0)
    };

    // Return bar — center-anchored, growing right for gains / left for losses.
    let base = helpers::signed_number_color(row.average_return_pct, theme);
    let magnitude = (row.average_return_pct.abs() as f32 / scale_max).clamp(0.0, 1.0);
    let bar_len = (magnitude * layout.half_w).max(2.0);
    let bar_x = if row.average_return_pct >= 0.0 {
        layout.mid_x
    } else {
        layout.mid_x - bar_len
    };
    let bar_top = center_y - BAR_THICKNESS * 0.5;
    let bar = Path::rounded_rectangle(
        Point::new(bar_x, bar_top),
        Size::new(bar_len, BAR_THICKNESS),
        BAR_RADIUS.into(),
    );
    frame.fill(
        &bar,
        lane_bar_gradient(base, bar_top, BAR_THICKNESS, confidence),
    );

    draw_win_bullet(frame, theme, layout, row.win_rate_pct, center_y, confidence);

    if let Some(value_right) = layout.value_right {
        frame.fill_text(Text {
            content: helpers::format_signed_percent_value(row.average_return_pct),
            position: Point::new(value_right, center_y),
            color: Color {
                a: if hovered { 1.0 } else { 0.9 },
                ..base
            },
            size: iced::Pixels(9.0),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..Default::default()
        });
    }
    if let Some(n_right) = layout.n_right {
        frame.fill_text(Text {
            content: format!("n{}", row.sample_count),
            position: Point::new(n_right, center_y),
            color: weak,
            size: iced::Pixels(9.0),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..Default::default()
        });
    }
}

fn lane_bar_gradient(
    color: Color,
    top_y: f32,
    height: f32,
    confidence: f32,
) -> canvas::gradient::Linear {
    canvas::gradient::Linear::new(Point::new(0.0, top_y), Point::new(0.0, top_y + height))
        .add_stop(
            0.0,
            Color {
                a: 0.95 * confidence,
                ..color
            },
        )
        .add_stop(
            1.0,
            Color {
                a: 0.60 * confidence,
                ..color
            },
        )
}

fn draw_win_bullet(
    frame: &mut Frame,
    theme: &Theme,
    layout: &LaneLayout,
    win_rate_pct: f64,
    center_y: f32,
    confidence: f32,
) {
    let x = layout.bullet_left;
    let top = center_y - BULLET_HEIGHT * 0.5;
    frame.fill_rectangle(
        Point::new(x, top),
        Size::new(BULLET_WIDTH, BULLET_HEIGHT),
        Color {
            a: 0.9,
            ..theme.extended_palette().background.strong.color
        },
    );
    let fill_w = (win_rate_pct.clamp(0.0, 100.0) as f32 / 100.0 * BULLET_WIDTH).max(0.0);
    frame.fill_rectangle(
        Point::new(x, top),
        Size::new(fill_w, BULLET_HEIGHT),
        Color {
            a: (0.55 + 0.45 * confidence).min(1.0),
            ..theme.palette().primary
        },
    );
    let tick_x = x + BULLET_WIDTH * 0.5;
    frame.fill_rectangle(
        Point::new(tick_x - 0.5, top - 1.0),
        Size::new(1.0, BULLET_HEIGHT + 2.0),
        Color {
            a: 0.6,
            ..theme.palette().text
        },
    );
}

fn draw_absent_dots(frame: &mut Frame, theme: &Theme, layout: &LaneLayout, center_y: f32) {
    let color = Color {
        a: 0.12,
        ..theme.palette().text
    };
    for k in 0..4 {
        let x = layout.mid_x + (k as f32 - 1.5) * 7.0;
        frame.fill_rectangle(
            Point::new(x - 1.0, center_y - 1.0),
            Size::new(2.0, 2.0),
            color,
        );
    }
}

pub(super) fn draw_lane_tooltip(
    frame: &mut Frame,
    theme: &Theme,
    size: Size,
    layout: &LaneLayout,
    row: &LaneRow,
    center_y: f32,
) {
    let origin = tooltip_origin(Point::new(layout.mid_x, center_y), size);
    frame.fill_rectangle(
        origin,
        Size::new(TOOLTIP_WIDTH, TOOLTIP_HEIGHT),
        Color {
            a: 0.96,
            ..theme.extended_palette().background.strong.color
        },
    );
    let border = Path::rectangle(origin, Size::new(TOOLTIP_WIDTH, TOOLTIP_HEIGHT));
    frame.stroke(
        &border,
        Stroke::default()
            .with_color(theme.extended_palette().background.weak.color)
            .with_width(1.0),
    );

    let content = if row.sample_count > 0 {
        let mut text = format!(
            "{}\nAvg {}\nWin {:.0}%   n{}",
            row.label,
            helpers::format_signed_percent_value(row.average_return_pct),
            row.win_rate_pct,
            row.sample_count,
        );
        if let Some(dispersion) = row.dispersion_pct {
            text.push_str(&format!("\n\u{00b1}{dispersion:.2}% per session"));
        }
        text
    } else {
        format!("{}\nNo completed sessions", row.label)
    };
    frame.fill_text(Text {
        content,
        position: Point::new(origin.x + 8.0, origin.y + 9.0),
        color: theme.palette().text,
        size: iced::Pixels(10.0),
        font: crate::app_fonts::monospace_font(),
        ..Default::default()
    });
}

pub(super) fn tooltip_origin(point: Point, size: Size) -> Point {
    let x = if point.x + TOOLTIP_WIDTH + 10.0 > size.width {
        point.x - TOOLTIP_WIDTH - 8.0
    } else {
        point.x + 8.0
    }
    .clamp(0.0, (size.width - TOOLTIP_WIDTH).max(0.0));
    let y = if point.y + TOOLTIP_HEIGHT + 10.0 > size.height {
        point.y - TOOLTIP_HEIGHT - 8.0
    } else {
        point.y + 8.0
    }
    .clamp(0.0, (size.height - TOOLTIP_HEIGHT).max(0.0));
    Point::new(x, y)
}
