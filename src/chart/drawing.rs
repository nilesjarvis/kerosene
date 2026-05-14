use iced::widget::canvas;
use iced::{Color, Point, Size, alignment};

pub(super) struct AxisBadgeStyle {
    pub(super) char_width: f32,
    pub(super) padding_width: f32,
    pub(super) height: f32,
    pub(super) text_size: f32,
    pub(super) text_color: Color,
}

pub(super) struct SegmentedHLineStyle {
    pub(super) segment_len: f32,
    pub(super) gap_len: f32,
    pub(super) offset: f32,
    pub(super) color: Color,
    pub(super) width: f32,
}

pub(super) fn stroke_segmented_hline(
    frame: &mut canvas::Frame,
    chart_w: f32,
    y: f32,
    segment_len: f32,
    gap_len: f32,
    color: Color,
    width: f32,
) {
    stroke_segmented_hline_with_offset(
        frame,
        chart_w,
        y,
        SegmentedHLineStyle {
            segment_len,
            gap_len,
            offset: 0.0,
            color,
            width,
        },
    );
}

pub(super) fn stroke_segmented_hline_with_offset(
    frame: &mut canvas::Frame,
    chart_w: f32,
    y: f32,
    style: SegmentedHLineStyle,
) {
    if chart_w <= 0.0 || style.segment_len <= 0.0 {
        return;
    }

    let stride = (style.segment_len + style.gap_len).max(style.segment_len);
    let phase = style.offset.rem_euclid(stride);
    let mut x = phase - stride;
    while x < chart_w {
        let start = x.max(0.0);
        let end = (x + style.segment_len).min(chart_w);
        if end > start {
            let seg = canvas::Path::line(Point::new(start, y), Point::new(end, y));
            frame.stroke(
                &seg,
                canvas::Stroke::default()
                    .with_color(style.color)
                    .with_width(style.width),
            );
        }
        x += stride;
    }
}

pub(super) fn fill_right_axis_badge(
    frame: &mut canvas::Frame,
    chart_w: f32,
    center_y: f32,
    label: String,
    background: Color,
    style: AxisBadgeStyle,
) {
    let badge_w = label.len() as f32 * style.char_width + style.padding_width;
    let badge_x = chart_w + 1.0;
    let badge_y = center_y - style.height * 0.5;

    frame.fill_rectangle(
        Point::new(badge_x, badge_y),
        Size::new(badge_w, style.height),
        background,
    );
    frame.fill_text(canvas::Text {
        content: label,
        position: Point::new(badge_x + 4.0, center_y),
        color: style.text_color,
        size: iced::Pixels(style.text_size),
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Center,
        font: iced::Font::MONOSPACE,
        ..canvas::Text::default()
    });
}
