use super::fisheye::{ChartFisheye, ProjectedPathPoint};
use crate::config::{ChartCrosshairStyle, normalize_chart_crosshair_scale};
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Size, Theme};

// ---------------------------------------------------------------------------
// Crosshair Style Rendering
// ---------------------------------------------------------------------------

const CLASSIC_ALPHA: f32 = 0.25;
const GUIDE_ALPHA: f32 = 0.18;
const CLASSIC_WIDTH: f32 = 0.5;
const GUIDE_WIDTH: f32 = 0.65;
const SHAPE_WIDTH: f32 = 1.2;
const HUD_ALPHA: f32 = 0.78;
const HUD_GUIDE_ALPHA: f32 = 0.34;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CrosshairStyleRender {
    pub(crate) style: ChartCrosshairStyle,
    pub(crate) guide_lines_enabled: bool,
    pub(crate) crosshair_scale: f32,
    pub(crate) position: Point,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) fisheye: ChartFisheye,
    pub(crate) accent_color: Option<Color>,
}

#[derive(Debug, Clone, Copy)]
struct GuideRender {
    style: ChartCrosshairStyle,
    position: Point,
    size: Size,
    scale: f32,
    fisheye: ChartFisheye,
    accent_color: Option<Color>,
}

pub(crate) fn draw_crosshair_style(
    frame: &mut canvas::Frame,
    theme: &Theme,
    render: CrosshairStyleRender,
) {
    let CrosshairStyleRender {
        style,
        guide_lines_enabled,
        crosshair_scale,
        position,
        width,
        height,
        fisheye,
        accent_color,
    } = render;
    let style = style.normalized();

    if width <= 0.0
        || height <= 0.0
        || !width.is_finite()
        || !height.is_finite()
        || !position.x.is_finite()
        || !position.y.is_finite()
    {
        return;
    }
    let scale = effective_style_scale(width, height, crosshair_scale);

    let clip = Rectangle {
        x: 0.0,
        y: 0.0,
        width,
        height,
    };

    frame.with_clip(clip, |frame| {
        if guide_lines_enabled {
            draw_guides(
                frame,
                theme,
                GuideRender {
                    style,
                    position,
                    size: Size::new(width, height),
                    scale,
                    fisheye,
                    accent_color,
                },
            );
        }
        match style {
            ChartCrosshairStyle::Classic if !guide_lines_enabled => {
                draw_classic_reticle(frame, theme, position, scale, fisheye);
            }
            ChartCrosshairStyle::Classic => {}
            ChartCrosshairStyle::Circle => {
                let radius = scaled_value(scale, 22.0, 9.0);
                fisheye.stroke_projected_circle(
                    frame,
                    position,
                    radius,
                    shape_stroke(theme, style, scale),
                );
            }
            ChartCrosshairStyle::Scope => {
                draw_scope_reticle(frame, theme, position, scale, fisheye);
            }
            ChartCrosshairStyle::Rangefinder => {
                draw_rangefinder_reticle(frame, theme, position, scale, fisheye);
            }
            ChartCrosshairStyle::Hud => {
                draw_hud_reticle(frame, position, scale, fisheye, accent_color);
            }
            ChartCrosshairStyle::Target => {
                draw_target_reticle(frame, theme, position, scale, fisheye);
            }
            ChartCrosshairStyle::Rectangle => {
                let size = scaled_size(scale, 58.0, 36.0);
                draw_rectangle(frame, theme, position, size, scale, fisheye);
            }
            ChartCrosshairStyle::StackedRectangles => {
                unreachable!("legacy crosshair style is normalized")
            }
        }
    });
}

fn draw_guides(frame: &mut canvas::Frame, theme: &Theme, render: GuideRender) {
    let GuideRender {
        style,
        position,
        size,
        scale,
        fisheye,
        accent_color,
    } = render;
    let stroke = canvas::Stroke::default()
        .with_color(guide_line_color(theme, style, accent_color))
        .with_width(guide_line_width(style) * scale.clamp(0.75, 2.0));

    fisheye.stroke_projected_line(
        frame,
        Point::new(0.0, position.y),
        Point::new(size.width, position.y),
        stroke,
    );
    fisheye.stroke_projected_line(
        frame,
        Point::new(position.x, 0.0),
        Point::new(position.x, size.height),
        stroke,
    );
}

fn draw_classic_reticle(
    frame: &mut canvas::Frame,
    theme: &Theme,
    center: Point,
    scale: f32,
    fisheye: ChartFisheye,
) {
    let gap = (4.0 * scale).max(2.5);
    let line_end = (18.0 * scale).max(8.0);
    let stroke = shape_stroke(theme, ChartCrosshairStyle::Classic, scale);

    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x - line_end, center.y),
        Point::new(center.x - gap, center.y),
    );
    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x + gap, center.y),
        Point::new(center.x + line_end, center.y),
    );
    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x, center.y - line_end),
        Point::new(center.x, center.y - gap),
    );
    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x, center.y + gap),
        Point::new(center.x, center.y + line_end),
    );
}

fn draw_scope_reticle(
    frame: &mut canvas::Frame,
    theme: &Theme,
    center: Point,
    scale: f32,
    fisheye: ChartFisheye,
) {
    let outer_radius = (29.0 * scale).max(12.0);
    let inner_radius = (7.0 * scale).max(3.0);
    let center_gap = (9.0 * scale).max(4.0);
    let line_end = outer_radius - 4.0 * scale;
    let stroke = shape_stroke(theme, ChartCrosshairStyle::Scope, scale);

    fisheye.stroke_projected_circle(frame, center, outer_radius, stroke);
    fisheye.stroke_projected_circle(frame, center, inner_radius, stroke);

    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x - line_end, center.y),
        Point::new(center.x - center_gap, center.y),
    );
    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x + center_gap, center.y),
        Point::new(center.x + line_end, center.y),
    );
    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x, center.y - line_end),
        Point::new(center.x, center.y - center_gap),
    );
    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x, center.y + center_gap),
        Point::new(center.x, center.y + line_end),
    );

    fisheye.fill_projected_circle(
        frame,
        center,
        (1.9 * scale).max(1.2),
        guide_color(theme, ChartCrosshairStyle::Scope),
    );
}

fn draw_rangefinder_reticle(
    frame: &mut canvas::Frame,
    theme: &Theme,
    center: Point,
    scale: f32,
    fisheye: ChartFisheye,
) {
    let axis_span = (68.0 * scale).max(28.0);
    let bracket_x = (42.0 * scale).max(18.0);
    let bracket_y = (31.0 * scale).max(14.0);
    let bracket_len = (15.0 * scale).max(7.0);
    let stroke = shape_stroke(theme, ChartCrosshairStyle::Rangefinder, scale);

    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x - axis_span, center.y),
        Point::new(center.x + axis_span, center.y),
    );
    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x, center.y - axis_span),
        Point::new(center.x, center.y + axis_span),
    );

    draw_rangefinder_ticks(frame, fisheye, stroke, center, scale);
    draw_corner_brackets(
        frame,
        fisheye,
        stroke,
        center,
        bracket_x,
        bracket_y,
        bracket_len,
    );
}

fn draw_hud_reticle(
    frame: &mut canvas::Frame,
    center: Point,
    scale: f32,
    fisheye: ChartFisheye,
    accent_color: Option<Color>,
) {
    let stroke = shape_stroke_with_color(hud_accent_color(accent_color, HUD_ALPHA), scale)
        .with_width((1.15 * scale).max(0.8));
    let fine_stroke = stroke.with_width((0.75 * scale).max(0.55));
    let inner_radius = (8.0 * scale).max(4.0);
    let center_gap = (13.0 * scale).max(6.0);
    let wing_span = (34.0 * scale).max(17.0);
    let bracket_x = (48.0 * scale).max(22.0);
    let bracket_y = (35.0 * scale).max(17.0);
    let bracket_len = (16.0 * scale).max(7.0);

    fisheye.stroke_projected_circle(frame, center, inner_radius, stroke);
    fisheye.fill_projected_circle(
        frame,
        center,
        (1.45 * scale).max(0.9),
        hud_accent_color(accent_color, HUD_ALPHA),
    );

    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x - wing_span, center.y),
        Point::new(center.x - center_gap, center.y),
    );
    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x + center_gap, center.y),
        Point::new(center.x + wing_span, center.y),
    );
    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x, center.y - wing_span * 0.7),
        Point::new(center.x, center.y - center_gap),
    );
    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x, center.y + center_gap),
        Point::new(center.x, center.y + wing_span * 0.55),
    );

    draw_hud_side_brackets(
        frame,
        fisheye,
        stroke,
        center,
        bracket_x,
        bracket_y,
        bracket_len,
    );
    draw_hud_pitch_ladder(frame, fisheye, fine_stroke, center, scale);
    stroke_projected_arc(
        frame,
        fisheye,
        fine_stroke,
        center,
        (57.0 * scale).max(27.0),
        std::f32::consts::PI * 0.18,
        std::f32::consts::PI * 0.82,
    );

    let acquisition_center = Point::new(center.x, center.y + (72.0 * scale).max(31.0));
    let acquisition_radius = (18.0 * scale).max(8.0);
    fisheye.stroke_projected_circle(frame, acquisition_center, acquisition_radius, fine_stroke);
    for direction in [
        Point::new(0.0, -1.0),
        Point::new(1.0, 0.0),
        Point::new(0.0, 1.0),
        Point::new(-1.0, 0.0),
    ] {
        let start = Point::new(
            acquisition_center.x + direction.x * (acquisition_radius - 3.0 * scale),
            acquisition_center.y + direction.y * (acquisition_radius - 3.0 * scale),
        );
        let end = Point::new(
            acquisition_center.x + direction.x * (acquisition_radius + 5.0 * scale),
            acquisition_center.y + direction.y * (acquisition_radius + 5.0 * scale),
        );
        stroke_segment(frame, fisheye, fine_stroke, start, end);
    }
}

fn draw_hud_side_brackets(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    stroke: canvas::Stroke<'static>,
    center: Point,
    bracket_x: f32,
    bracket_y: f32,
    bracket_len: f32,
) {
    for direction in [-1.0, 1.0] {
        let upper = Point::new(center.x + direction * bracket_x, center.y - bracket_y);
        let lower = Point::new(
            center.x + direction * bracket_x,
            center.y - bracket_y * 0.38,
        );
        stroke_segment(frame, fisheye, stroke, upper, lower);
        stroke_segment(
            frame,
            fisheye,
            stroke,
            upper,
            Point::new(upper.x - direction * bracket_len, upper.y),
        );
        stroke_segment(
            frame,
            fisheye,
            stroke,
            lower,
            Point::new(lower.x - direction * bracket_len * 0.55, lower.y),
        );
    }
}

fn draw_hud_pitch_ladder(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    stroke: canvas::Stroke<'static>,
    center: Point,
    scale: f32,
) {
    let radius = (76.0 * scale).max(34.0);
    let tick_len = (11.0 * scale).max(5.0);
    for i in 0..9 {
        let angle = std::f32::consts::PI * (0.26 + i as f32 * 0.06);
        let outer = Point::new(
            center.x + angle.cos() * radius,
            center.y + angle.sin() * radius,
        );
        let inner = Point::new(
            center.x + angle.cos() * (radius - tick_len),
            center.y + angle.sin() * (radius - tick_len),
        );
        stroke_segment(frame, fisheye, stroke, outer, inner);
    }
}

fn stroke_projected_arc(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    stroke: canvas::Stroke<'static>,
    center: Point,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
) {
    if radius <= 0.0 || !radius.is_finite() {
        return;
    }

    let samples = 28;
    let mut points = Vec::with_capacity(samples + 1);
    for index in 0..=samples {
        let t = index as f32 / samples as f32;
        let angle = start_angle + (end_angle - start_angle) * t;
        points.push(ProjectedPathPoint {
            point: Point::new(
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
            ),
            starts_segment: index == 0,
        });
    }
    fisheye.stroke_projected_path_points(frame, &points, stroke);
}

fn draw_target_reticle(
    frame: &mut canvas::Frame,
    theme: &Theme,
    center: Point,
    scale: f32,
    fisheye: ChartFisheye,
) {
    let radius = (42.0 * scale).max(19.0);
    fisheye.stroke_projected_circle(
        frame,
        center,
        radius,
        shape_stroke(theme, ChartCrosshairStyle::Target, scale).with_width((5.6 * scale).max(2.4)),
    );

    let stroke = shape_stroke(theme, ChartCrosshairStyle::Target, scale);
    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x - radius, center.y),
        Point::new(center.x + radius, center.y),
    );
    stroke_segment(
        frame,
        fisheye,
        stroke,
        Point::new(center.x, center.y - radius),
        Point::new(center.x, center.y + radius),
    );

    draw_target_ticks(frame, fisheye, stroke, center, scale);
    draw_target_blocks(frame, theme, center, radius, scale, fisheye);
}

fn draw_target_ticks(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    stroke: canvas::Stroke<'static>,
    center: Point,
    scale: f32,
) {
    let offsets = [18.0, 28.0];
    let tick_half = (5.0 * scale).max(2.4);
    for offset in offsets {
        let offset = offset * scale;
        for direction in [-1.0, 1.0] {
            stroke_segment(
                frame,
                fisheye,
                stroke,
                Point::new(center.x + direction * offset, center.y - tick_half),
                Point::new(center.x + direction * offset, center.y + tick_half),
            );
            stroke_segment(
                frame,
                fisheye,
                stroke,
                Point::new(center.x - tick_half, center.y + direction * offset),
                Point::new(center.x + tick_half, center.y + direction * offset),
            );
        }
    }
}

fn draw_target_blocks(
    frame: &mut canvas::Frame,
    theme: &Theme,
    center: Point,
    radius: f32,
    scale: f32,
    fisheye: ChartFisheye,
) {
    let block_width = (8.0 * scale).max(3.8);
    let block_len = (14.0 * scale).max(6.0);
    let point_len = (4.0 * scale).max(2.0);
    let inset = (3.0 * scale).max(1.4);
    let color = guide_color(theme, ChartCrosshairStyle::Target);

    for direction in [
        Point::new(0.0, -1.0),
        Point::new(1.0, 0.0),
        Point::new(0.0, 1.0),
        Point::new(-1.0, 0.0),
    ] {
        let block = target_block_points(
            center,
            direction,
            radius - inset,
            block_width,
            block_len,
            point_len,
        );
        fisheye.fill_projected_polygon(frame, &block, color);
    }
}

fn target_block_points(
    center: Point,
    direction: Point,
    outer_offset: f32,
    width: f32,
    block_len: f32,
    point_len: f32,
) -> [Point; 5] {
    let tangent = Point::new(-direction.y, direction.x);
    let outer_center = Point::new(
        center.x + direction.x * outer_offset,
        center.y + direction.y * outer_offset,
    );
    let inner_center = Point::new(
        outer_center.x - direction.x * block_len,
        outer_center.y - direction.y * block_len,
    );
    let tip = Point::new(
        inner_center.x - direction.x * point_len,
        inner_center.y - direction.y * point_len,
    );

    [
        Point::new(
            outer_center.x + tangent.x * width * 0.5,
            outer_center.y + tangent.y * width * 0.5,
        ),
        Point::new(
            outer_center.x - tangent.x * width * 0.5,
            outer_center.y - tangent.y * width * 0.5,
        ),
        Point::new(
            inner_center.x - tangent.x * width * 0.5,
            inner_center.y - tangent.y * width * 0.5,
        ),
        tip,
        Point::new(
            inner_center.x + tangent.x * width * 0.5,
            inner_center.y + tangent.y * width * 0.5,
        ),
    ]
}

fn draw_rangefinder_ticks(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    stroke: canvas::Stroke<'static>,
    center: Point,
    scale: f32,
) {
    let offsets = [7.0, 13.0, 19.0, 27.0, 35.0];
    for (index, offset) in offsets.into_iter().enumerate() {
        let offset = offset * scale;
        let tick_half = if index == 3 { 7.0 * scale } else { 4.0 * scale };
        for direction in [-1.0, 1.0] {
            stroke_segment(
                frame,
                fisheye,
                stroke,
                Point::new(center.x + direction * offset, center.y - tick_half),
                Point::new(center.x + direction * offset, center.y + tick_half),
            );
            stroke_segment(
                frame,
                fisheye,
                stroke,
                Point::new(center.x - tick_half, center.y + direction * offset),
                Point::new(center.x + tick_half, center.y + direction * offset),
            );
        }
    }
}

fn draw_corner_brackets(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    stroke: canvas::Stroke<'static>,
    center: Point,
    bracket_x: f32,
    bracket_y: f32,
    bracket_len: f32,
) {
    for x_sign in [-1.0, 1.0] {
        for y_sign in [-1.0, 1.0] {
            let corner = Point::new(center.x + x_sign * bracket_x, center.y + y_sign * bracket_y);
            stroke_segment(
                frame,
                fisheye,
                stroke,
                corner,
                Point::new(corner.x - x_sign * bracket_len, corner.y),
            );
            stroke_segment(
                frame,
                fisheye,
                stroke,
                corner,
                Point::new(corner.x, corner.y - y_sign * bracket_len),
            );
        }
    }
}

fn stroke_segment(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    stroke: canvas::Stroke<'static>,
    start: Point,
    end: Point,
) {
    fisheye.stroke_projected_line(frame, start, end, stroke);
}

fn draw_rectangle(
    frame: &mut canvas::Frame,
    theme: &Theme,
    center: Point,
    size: Size,
    scale: f32,
    fisheye: ChartFisheye,
) {
    let top_left = Point::new(center.x - size.width * 0.5, center.y - size.height * 0.5);
    fisheye.stroke_projected_rect(
        frame,
        top_left,
        size,
        shape_stroke(theme, ChartCrosshairStyle::Rectangle, scale),
    );
}

fn shape_stroke(theme: &Theme, style: ChartCrosshairStyle, scale: f32) -> canvas::Stroke<'static> {
    shape_stroke_with_color(guide_color(theme, style), scale)
}

fn shape_stroke_with_color(color: Color, scale: f32) -> canvas::Stroke<'static> {
    canvas::Stroke::default()
        .with_color(color)
        .with_width(SHAPE_WIDTH * scale.max(0.72))
        .with_line_cap(canvas::LineCap::Round)
        .with_line_join(canvas::LineJoin::Round)
}

fn guide_color(theme: &Theme, style: ChartCrosshairStyle) -> Color {
    if matches!(style, ChartCrosshairStyle::Hud) {
        return hud_green(HUD_ALPHA);
    }

    Color {
        a: guide_alpha(style),
        ..theme.palette().text
    }
}

fn guide_line_color(
    theme: &Theme,
    style: ChartCrosshairStyle,
    accent_color: Option<Color>,
) -> Color {
    if matches!(style, ChartCrosshairStyle::Hud) {
        hud_accent_color(accent_color, HUD_GUIDE_ALPHA)
    } else {
        guide_color(theme, style)
    }
}

fn hud_accent_color(accent_color: Option<Color>, alpha: f32) -> Color {
    match accent_color {
        Some(color) => Color { a: alpha, ..color },
        None => hud_green(alpha),
    }
}

fn hud_green(alpha: f32) -> Color {
    Color {
        a: alpha,
        ..Color::from_rgb8(0x50, 0xfa, 0x7b)
    }
}

fn guide_alpha(style: ChartCrosshairStyle) -> f32 {
    match style {
        ChartCrosshairStyle::Classic => CLASSIC_ALPHA,
        ChartCrosshairStyle::Hud => HUD_GUIDE_ALPHA,
        _ => GUIDE_ALPHA,
    }
}

fn guide_line_width(style: ChartCrosshairStyle) -> f32 {
    match style {
        ChartCrosshairStyle::Classic => CLASSIC_WIDTH,
        ChartCrosshairStyle::Hud => 0.85,
        _ => GUIDE_WIDTH,
    }
}

fn scaled_size(scale: f32, target_width: f32, target_height: f32) -> Size {
    Size::new(target_width * scale, target_height * scale)
}

fn scaled_value(scale: f32, target: f32, minimum: f32) -> f32 {
    (target * scale).max(minimum)
}

fn effective_style_scale(width: f32, height: f32, crosshair_scale: f32) -> f32 {
    style_scale(width, height) * normalize_chart_crosshair_scale(crosshair_scale)
}

fn style_scale(width: f32, height: f32) -> f32 {
    (width.min(height) / 80.0).clamp(0.45, 1.0)
}

#[cfg(test)]
mod tests {
    use super::style_scale;

    #[test]
    fn style_scale_keeps_chart_reticles_full_size() {
        assert_eq!(style_scale(1000.0, 600.0), 1.0);
    }

    #[test]
    fn style_scale_compacts_small_previews() {
        assert_eq!(style_scale(80.0, 40.0), 0.5);
        assert_eq!(style_scale(20.0, 20.0), 0.45);
    }
}
