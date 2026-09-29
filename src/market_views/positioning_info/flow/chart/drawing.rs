use super::{
    HEADER_GAP, HEADER_HEIGHT, LABEL_WIDTH, MIN_BAR_PX, PositioningFlowChart,
    PositioningFlowChartRow, PositioningFlowKind, ROW_GAP, ROW_HEIGHT, SHOW_LABEL_MIN_WIDTH,
    SHOW_TAG_MIN_WIDTH, SHOW_VALUE_MIN_WIDTH, SIDE_PADDING, TAG_WIDTH, TOOLTIP_ANIMATION_OFFSET_PX,
    VALUE_WIDTH,
};

use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::{Color, Point, Rectangle, Size, Theme};

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

pub(super) struct FlowLayout {
    pub(super) show_value: bool,
    pub(super) show_tag: bool,
    pub(super) plot_right: f32,
    pub(super) center_x: f32,
    pub(super) half_width: f32,
}

impl FlowLayout {
    pub(super) fn new(width: f32) -> Self {
        let show_value = width >= SHOW_VALUE_MIN_WIDTH;
        let show_label = width >= SHOW_LABEL_MIN_WIDTH;
        let show_tag = width >= SHOW_TAG_MIN_WIDTH;

        let label_w = if show_label { LABEL_WIDTH } else { 0.0 };
        let value_w = if show_value { VALUE_WIDTH } else { 0.0 };
        let tag_w = if show_tag { TAG_WIDTH } else { 0.0 };

        let plot_left = SIDE_PADDING + label_w;
        let plot_right = (width - SIDE_PADDING - value_w - tag_w).max(plot_left + 2.0 * MIN_BAR_PX);
        let center_x = (plot_left + plot_right) / 2.0;
        let half_width = (plot_right - plot_left) / 2.0;

        Self {
            show_value,
            show_tag,
            plot_right,
            center_x,
            half_width,
        }
    }
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

impl PositioningFlowChart {
    pub(super) fn draw_header(&self, frame: &mut Frame, theme: &Theme, width: f32) {
        let palette = theme.palette();
        let long_color = palette.success;
        let short_color = palette.danger;
        let muted = theme.extended_palette().background.weak.text;

        let left = SIDE_PADDING;
        let right = (width - SIDE_PADDING).max(left + 2.0);
        let track_w = right - left;
        let y = 4.0;
        let bar_h = 12.0;

        let total = self.long_flow + self.short_flow;
        let long_frac = if total > 0.0 {
            (self.long_flow / total) as f32
        } else {
            0.5
        };
        let split_x = left + track_w * long_frac;

        // Track background.
        frame.fill_rectangle(
            Point::new(left, y),
            Size::new(track_w, bar_h),
            faint(muted, 0.10),
        );
        // Short portion (left) and long portion (right).
        frame.fill_rectangle(
            Point::new(left, y),
            Size::new((split_x - left).max(0.0), bar_h),
            faint(short_color, 0.55),
        );
        frame.fill_rectangle(
            Point::new(split_x, y),
            Size::new((right - split_x).max(0.0), bar_h),
            faint(long_color, 0.55),
        );

        // Side labels and net readout below the track.
        let label_y = y + bar_h + 9.0;
        frame.fill_text(Text {
            content: format!("Shorts {}", self.short_label),
            position: Point::new(left, label_y),
            color: short_color,
            size: iced::Pixels(10.0),
            align_x: Horizontal::Left.into(),
            align_y: Vertical::Center,
            ..Default::default()
        });
        frame.fill_text(Text {
            content: self.net_label.clone(),
            position: Point::new((left + right) / 2.0, label_y),
            color: theme.palette().text,
            size: iced::Pixels(10.0),
            align_x: Horizontal::Center.into(),
            align_y: Vertical::Center,
            ..Default::default()
        });
        frame.fill_text(Text {
            content: format!("Longs {}", self.long_label),
            position: Point::new(right, label_y),
            color: long_color,
            size: iced::Pixels(10.0),
            align_x: Horizontal::Right.into(),
            align_y: Vertical::Center,
            ..Default::default()
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_row(
        &self,
        frame: &mut Frame,
        theme: &Theme,
        layout: &FlowLayout,
        row: &PositioningFlowChartRow,
        top: f32,
        hovered: bool,
        actions_hovered: bool,
    ) {
        let palette = theme.palette();
        let center_y = top + ROW_HEIGHT / 2.0;
        let color = if row.more_long {
            palette.success
        } else {
            palette.danger
        };

        // While the trader actions are hovered the overlay shows the action
        // pill (with its own surface), so the canvas skips the row highlight to
        // avoid stacking box-on-box; a plain cursor hover gets a faint wash.
        if hovered && !actions_hovered {
            frame.fill_rectangle(
                Point::new(SIDE_PADDING / 2.0, top),
                Size::new(layout.plot_right + 200.0, ROW_HEIGHT),
                faint(palette.text, 0.06),
            );
        }

        // Diverging bar.
        let frac = if self.max_magnitude > 0.0 {
            (row.magnitude / self.max_magnitude) as f32
        } else {
            0.0
        };
        let bar_len =
            (frac * layout.half_width).max(if row.magnitude > 0.0 { MIN_BAR_PX } else { 0.0 });
        let bar_h = ROW_HEIGHT - 8.0;
        let bar_y = center_y - bar_h / 2.0;
        if row.more_long {
            frame.fill_rectangle(
                Point::new(layout.center_x, bar_y),
                Size::new(bar_len, bar_h),
                faint(color, 0.85),
            );
        } else {
            frame.fill_rectangle(
                Point::new(layout.center_x - bar_len, bar_y),
                Size::new(bar_len, bar_h),
                faint(color, 0.85),
            );
        }

        // The trader label (and its hover action buttons) live in the widget
        // overlay stacked above the canvas, so it is never drawn here.

        if layout.show_value {
            frame.fill_text(Text {
                content: row.value_text.clone(),
                position: Point::new(layout.plot_right + 6.0, center_y),
                color,
                size: iced::Pixels(11.0),
                align_x: Horizontal::Left.into(),
                align_y: Vertical::Center,
                ..Default::default()
            });
        }

        if layout.show_tag {
            frame.fill_text(Text {
                content: row.kind.label().to_string(),
                position: Point::new(layout.plot_right + 6.0 + VALUE_WIDTH, center_y),
                color: kind_color(row.kind, theme),
                size: iced::Pixels(10.0),
                align_x: Horizontal::Left.into(),
                align_y: Vertical::Center,
                ..Default::default()
            });
        }
    }
}

pub(super) fn draw_tooltip(
    frame: &mut Frame,
    theme: &Theme,
    bounds: Rectangle,
    layout: &FlowLayout,
    row: &PositioningFlowChartRow,
    index: usize,
    visibility: f32,
) {
    let visibility = visibility.clamp(0.0, 1.0);
    if row.tooltip.is_empty() || visibility <= 0.0 {
        return;
    }
    let line_h = 14.0;
    let pad = 8.0;
    let title_h = 16.0;
    let label_w: f32 = 64.0;
    let value_w: f32 = 96.0;
    let box_w = pad * 2.0 + label_w + value_w;
    let box_h = pad * 2.0 + title_h + row.tooltip.len() as f32 * line_h;

    let row_top = HEADER_HEIGHT + HEADER_GAP + index as f32 * (ROW_HEIGHT + ROW_GAP);
    let mut y = row_top + ROW_HEIGHT + 2.0;
    if y + box_h > bounds.height {
        y = (row_top - box_h - 2.0).max(0.0);
    }
    y = (y + (1.0 - visibility) * TOOLTIP_ANIMATION_OFFSET_PX)
        .clamp(0.0, (bounds.height - box_h).max(0.0));
    let x = (layout.center_x - box_w / 2.0).clamp(2.0, (bounds.width - box_w - 2.0).max(2.0));

    frame.fill_rectangle(
        Point::new(x, y),
        Size::new(box_w, box_h),
        scale_alpha(theme.extended_palette().background.strong.color, visibility),
    );
    let border = Path::rectangle(Point::new(x, y), Size::new(box_w, box_h));
    frame.stroke(
        &border,
        Stroke::default()
            .with_color(scale_alpha(
                theme.extended_palette().background.weak.color,
                visibility,
            ))
            .with_width(1.0),
    );

    frame.fill_text(Text {
        content: row.label.clone(),
        position: Point::new(x + pad, y + pad),
        color: scale_alpha(theme.palette().text, visibility),
        size: iced::Pixels(11.0),
        align_x: Horizontal::Left.into(),
        align_y: Vertical::Top,
        ..Default::default()
    });

    let mut line_y = y + pad + title_h;
    for (label, value) in &row.tooltip {
        frame.fill_text(Text {
            content: label.clone(),
            position: Point::new(x + pad, line_y),
            color: scale_alpha(theme.extended_palette().background.weak.text, visibility),
            size: iced::Pixels(10.0),
            align_x: Horizontal::Left.into(),
            align_y: Vertical::Top,
            ..Default::default()
        });
        frame.fill_text(Text {
            content: value.clone(),
            position: Point::new(x + box_w - pad, line_y),
            color: scale_alpha(theme.palette().text, visibility),
            size: iced::Pixels(10.0),
            align_x: Horizontal::Right.into(),
            align_y: Vertical::Top,
            ..Default::default()
        });
        line_y += line_h;
    }
}

// ---------------------------------------------------------------------------
// Colors & helpers
// ---------------------------------------------------------------------------

fn faint(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

fn scale_alpha(color: Color, scale: f32) -> Color {
    Color {
        a: color.a * scale.clamp(0.0, 1.0),
        ..color
    }
}

pub(super) fn axis_color(theme: &Theme) -> Color {
    faint(theme.extended_palette().background.weak.text, 0.35)
}

fn kind_color(kind: PositioningFlowKind, theme: &Theme) -> Color {
    match kind {
        PositioningFlowKind::Flip => theme.palette().primary,
        PositioningFlowKind::Add | PositioningFlowKind::Cut => {
            theme.extended_palette().background.weak.text
        }
    }
}
