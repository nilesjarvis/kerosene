use super::fisheye::{ChartFisheye, ProjectedPathPoint};
use crate::config::{ChartCrosshairStyle, normalize_chart_crosshair_scale};
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Size, Theme, alignment};

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
const RACING_HUD_ALPHA: f32 = 0.88;
const RACING_HUD_MUTED_ALPHA: f32 = 0.26;
const RACING_HUD_MAX_CURSOR_SPEED_PX_PER_S: f32 = 2_400.0;

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
    pub(crate) racing_hud_metrics: Option<RacingHudMetrics>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RacingHudMetrics {
    current_size: Option<f64>,
    max_size: Option<f64>,
    cursor_speed_px_per_s: Option<f32>,
}

impl RacingHudMetrics {
    pub(crate) fn new(
        current_size: Option<f64>,
        max_size: Option<f64>,
        cursor_speed_px_per_s: Option<f32>,
    ) -> Self {
        Self {
            current_size: current_size.and_then(nonnegative_finite_value),
            max_size: max_size.and_then(positive_finite_value),
            cursor_speed_px_per_s: cursor_speed_px_per_s.and_then(nonnegative_finite_f32),
        }
    }

    pub(crate) fn preview() -> Self {
        Self::new(Some(2.5), Some(5.0), Some(940.0))
    }

    fn usage_ratio(self) -> Option<f32> {
        let current_size = self.current_size?;
        let max_size = self.max_size?;
        let ratio = current_size / max_size;
        ratio.is_finite().then_some(ratio.max(0.0) as f32)
    }

    fn cursor_speed_ratio(self) -> f32 {
        self.cursor_speed_px_per_s
            .map(|speed| speed / RACING_HUD_MAX_CURSOR_SPEED_PX_PER_S)
            .filter(|ratio| ratio.is_finite())
            .unwrap_or(0.0)
            .max(0.0)
    }
}

impl Default for RacingHudMetrics {
    fn default() -> Self {
        Self::new(None, None, None)
    }
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
        racing_hud_metrics,
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
            ChartCrosshairStyle::RacingHud => {
                draw_racing_hud_reticle(
                    frame,
                    theme,
                    RacingHudReticleRender {
                        center: position,
                        area: Size::new(width, height),
                        scale,
                        fisheye,
                        accent_color,
                        metrics: racing_hud_metrics.unwrap_or_default(),
                    },
                );
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

#[derive(Debug, Clone, Copy)]
struct RacingHudReticleRender {
    center: Point,
    area: Size,
    scale: f32,
    fisheye: ChartFisheye,
    accent_color: Option<Color>,
    metrics: RacingHudMetrics,
}

fn draw_racing_hud_reticle(
    frame: &mut canvas::Frame,
    theme: &Theme,
    render: RacingHudReticleRender,
) {
    let RacingHudReticleRender {
        center,
        area,
        scale,
        fisheye,
        accent_color,
        metrics,
    } = render;
    let radius = racing_gauge_radius(area, scale);
    let gauge_scale = (radius / 38.0).clamp(0.32, scale.max(0.32));
    let preferred_offset = (65.0 * gauge_scale).max(radius + 15.0 * gauge_scale);
    let min_offset = radius + (9.0 * gauge_scale).max(5.0);
    let max_offset = ((area.width - 6.0) * 0.5 - radius).max(0.0);
    let can_draw_pair = max_offset >= min_offset;
    let offset = preferred_offset.min(max_offset).max(min_offset);
    let cluster_half = offset + radius;
    let cluster_x = if can_draw_pair {
        clamp_center_axis(center.x, area.width, cluster_half, 3.0)
    } else {
        clamp_center_axis(center.x, area.width, radius, 3.0)
    };
    let cluster_y = clamp_center_axis(center.y + 2.0 * gauge_scale, area.height, radius, 3.0);
    let cluster_center = Point::new(cluster_x, cluster_y);
    let accent = hud_accent_color(accent_color, RACING_HUD_ALPHA);
    let muted = hud_accent_color(accent_color, RACING_HUD_MUTED_ALPHA);
    let usage_ratio = metrics.usage_ratio();
    let size_needle_t = usage_ratio.unwrap_or(0.0).clamp(0.0, 1.0);
    let speed_needle_t = metrics.cursor_speed_ratio().clamp(0.0, 1.0);
    let size_value = format_racing_hud_size(metrics.current_size);
    let speed_value = format_racing_hud_speed(metrics.cursor_speed_px_per_s);

    let (left_center, right_center) = if can_draw_pair {
        let left_center = Point::new(cluster_x - offset, cluster_y);
        let right_center = Point::new(cluster_x + offset, cluster_y);
        let bridge_stroke = canvas::Stroke::default()
            .with_color(muted)
            .with_width((1.2 * gauge_scale).max(0.8))
            .with_line_cap(canvas::LineCap::Round);
        let bridge_gap = (20.0 * gauge_scale).max(9.0).min(offset * 0.45);
        stroke_segment(
            frame,
            fisheye,
            bridge_stroke,
            Point::new(left_center.x + radius * 0.82, cluster_y),
            Point::new(cluster_center.x - bridge_gap, cluster_y),
        );
        stroke_segment(
            frame,
            fisheye,
            bridge_stroke,
            Point::new(cluster_center.x + bridge_gap, cluster_y),
            Point::new(right_center.x - radius * 0.82, cluster_y),
        );
        (left_center, right_center)
    } else {
        (cluster_center, cluster_center)
    };

    draw_racing_hud_gauge(
        frame,
        theme,
        fisheye,
        RacingGaugeRender {
            center: left_center,
            radius,
            scale: gauge_scale,
            title: "SIZE",
            value: &size_value,
            unit: "COIN",
            needle_t: size_needle_t,
            accent,
        },
    );
    if can_draw_pair {
        draw_racing_hud_gauge(
            frame,
            theme,
            fisheye,
            RacingGaugeRender {
                center: right_center,
                radius,
                scale: gauge_scale,
                title: "SPEED",
                value: &speed_value,
                unit: "PX/S",
                needle_t: speed_needle_t,
                accent,
            },
        );
    }
}

fn racing_gauge_radius(area: Size, scale: f32) -> f32 {
    let desired = (38.0 * scale).max(17.0);
    let fit = ((area.width.min(area.height) - 6.0) * 0.5).max(0.0);
    let fit = if fit.is_finite() { fit } else { desired };
    desired.min(fit).max(8.0)
}

fn clamp_center_axis(value: f32, extent: f32, half_size: f32, padding: f32) -> f32 {
    if extent.is_finite() && extent >= half_size * 2.0 + padding * 2.0 {
        value.clamp(half_size + padding, extent - half_size - padding)
    } else if extent.is_finite() && extent > 0.0 {
        extent * 0.5
    } else {
        value
    }
}

struct RacingGaugeRender<'a> {
    center: Point,
    radius: f32,
    scale: f32,
    title: &'a str,
    value: &'a str,
    unit: &'a str,
    needle_t: f32,
    accent: Color,
}

fn draw_racing_hud_gauge(
    frame: &mut canvas::Frame,
    theme: &Theme,
    fisheye: ChartFisheye,
    render: RacingGaugeRender<'_>,
) {
    let RacingGaugeRender {
        center,
        radius,
        scale,
        title,
        value,
        unit,
        needle_t,
        accent,
    } = render;
    let panel_fill = Color {
        a: 0.16,
        ..theme.extended_palette().background.strong.color
    };
    let muted = Color {
        a: RACING_HUD_MUTED_ALPHA,
        ..accent
    };
    let danger = Color {
        a: 0.78,
        ..theme.palette().danger
    };
    let fine_stroke = canvas::Stroke::default()
        .with_color(muted)
        .with_width((0.9 * scale).max(0.6))
        .with_line_cap(canvas::LineCap::Round);
    let outer_stroke = canvas::Stroke::default()
        .with_color(accent)
        .with_width((1.4 * scale).max(0.95))
        .with_line_cap(canvas::LineCap::Round)
        .with_line_join(canvas::LineJoin::Round);

    fisheye.fill_projected_circle(frame, center, radius, panel_fill);
    fisheye.stroke_projected_circle(frame, center, radius, outer_stroke);
    fisheye.stroke_projected_circle(frame, center, radius * 0.79, fine_stroke);

    let start_angle = std::f32::consts::PI * 0.72;
    let end_angle = std::f32::consts::PI * 2.28;
    let angle_span = end_angle - start_angle;
    let needle_t = needle_t.clamp(0.0, 1.0);
    let active_end = start_angle + angle_span * needle_t;
    let arc_radius = radius * 0.67;
    let arc_stroke = outer_stroke.with_width((2.7 * scale).max(1.35));
    stroke_projected_arc(
        frame,
        fisheye,
        fine_stroke,
        center,
        arc_radius,
        start_angle,
        end_angle,
    );
    stroke_projected_arc(
        frame,
        fisheye,
        arc_stroke,
        center,
        arc_radius,
        start_angle,
        active_end,
    );
    stroke_projected_arc(
        frame,
        fisheye,
        arc_stroke.with_color(danger),
        center,
        arc_radius,
        start_angle + angle_span * 0.84,
        end_angle,
    );
    draw_racing_gauge_ticks(
        frame,
        fisheye,
        RacingGaugeTicksRender {
            center,
            radius,
            scale,
            start_angle,
            angle_span,
            color: muted,
        },
    );
    draw_racing_gauge_needle(frame, fisheye, center, radius, scale, active_end, accent);
    draw_racing_gauge_text(
        frame,
        fisheye,
        RacingGaugeTextRender {
            center,
            radius,
            scale,
            title,
            value,
            unit,
            color: accent,
        },
    );
}

#[derive(Debug, Clone, Copy)]
struct RacingGaugeTicksRender {
    center: Point,
    radius: f32,
    scale: f32,
    start_angle: f32,
    angle_span: f32,
    color: Color,
}

fn draw_racing_gauge_ticks(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    render: RacingGaugeTicksRender,
) {
    let RacingGaugeTicksRender {
        center,
        radius,
        scale,
        start_angle,
        angle_span,
        color,
    } = render;
    let tick_stroke = canvas::Stroke::default()
        .with_color(color)
        .with_width((0.85 * scale).max(0.6))
        .with_line_cap(canvas::LineCap::Round);
    for index in 0..=10 {
        let t = index as f32 / 10.0;
        let angle = start_angle + angle_span * t;
        let tick_len = if index % 5 == 0 {
            radius * 0.19
        } else if index % 2 == 0 {
            radius * 0.15
        } else {
            radius * 0.1
        };
        let outer = gauge_point(center, radius * 0.9, angle);
        let inner = gauge_point(center, radius * 0.9 - tick_len, angle);
        stroke_segment(frame, fisheye, tick_stroke, outer, inner);
    }
}

fn draw_racing_gauge_needle(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    center: Point,
    radius: f32,
    scale: f32,
    angle: f32,
    color: Color,
) {
    let needle_end = gauge_point(center, radius * 0.55, angle);
    let needle_tail = gauge_point(center, radius * 0.16, angle + std::f32::consts::PI);
    let needle_stroke = canvas::Stroke::default()
        .with_color(color)
        .with_width((1.9 * scale).max(1.1))
        .with_line_cap(canvas::LineCap::Round);
    stroke_segment(frame, fisheye, needle_stroke, needle_tail, needle_end);
    fisheye.fill_projected_circle(frame, center, (3.4 * scale).max(1.9), color);
    fisheye.fill_projected_circle(
        frame,
        center,
        (1.4 * scale).max(0.9),
        Color {
            a: 0.9,
            ..Color::BLACK
        },
    );
}

#[derive(Debug, Clone, Copy)]
struct RacingGaugeTextRender<'a> {
    center: Point,
    radius: f32,
    scale: f32,
    title: &'a str,
    value: &'a str,
    unit: &'a str,
    color: Color,
}

fn draw_racing_gauge_text(
    frame: &mut canvas::Frame,
    fisheye: ChartFisheye,
    render: RacingGaugeTextRender<'_>,
) {
    let RacingGaugeTextRender {
        center,
        radius,
        scale,
        title,
        value,
        unit,
        color,
    } = render;
    let text_color = Color { a: 0.92, ..color };
    let value_size = (11.6 * scale).clamp(7.0, 11.6);
    let label_size = (7.8 * scale).clamp(5.8, 7.8);
    let value_position = fisheye.project(Point::new(center.x, center.y + radius * 0.23));
    let title_position = fisheye.project(Point::new(center.x, center.y + radius * 0.45));
    let unit_position = fisheye.project(Point::new(center.x, center.y - radius * 0.33));
    frame.fill_text(canvas::Text {
        content: value.to_string(),
        position: value_position,
        color: text_color,
        size: iced::Pixels(value_size),
        align_x: alignment::Horizontal::Center.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
    frame.fill_text(canvas::Text {
        content: title.to_string(),
        position: title_position,
        color: text_color,
        size: iced::Pixels(label_size),
        align_x: alignment::Horizontal::Center.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
    frame.fill_text(canvas::Text {
        content: unit.to_string(),
        position: unit_position,
        color: Color { a: 0.62, ..color },
        size: iced::Pixels(label_size),
        align_x: alignment::Horizontal::Center.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

fn gauge_point(center: Point, radius: f32, angle: f32) -> Point {
    Point::new(
        center.x + angle.cos() * radius,
        center.y + angle.sin() * radius,
    )
}

fn format_racing_hud_size(size: Option<f64>) -> String {
    let Some(size) = size.filter(|value| value.is_finite() && *value >= 0.0) else {
        return "--".to_string();
    };
    let mut label = if size >= 1_000.0 {
        format!("{size:.0}")
    } else if size >= 100.0 {
        format!("{size:.1}")
    } else if size >= 10.0 {
        format!("{size:.2}")
    } else if size >= 1.0 {
        format!("{size:.3}")
    } else {
        format!("{size:.4}")
    };
    while label.contains('.') && label.ends_with('0') {
        label.pop();
    }
    if label.ends_with('.') {
        label.pop();
    }
    label
}

fn format_racing_hud_speed(speed: Option<f32>) -> String {
    let Some(speed) = speed.filter(|value| value.is_finite() && *value >= 0.0) else {
        return "--".to_string();
    };
    if speed >= 1_000.0 {
        format!("{:.1}K", (speed / 1_000.0).min(9.9))
    } else {
        format!("{:.0}", speed)
    }
}

fn positive_finite_value(value: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0).then_some(value)
}

fn nonnegative_finite_value(value: f64) -> Option<f64> {
    (value.is_finite() && value >= 0.0).then_some(value)
}

fn nonnegative_finite_f32(value: f32) -> Option<f32> {
    (value.is_finite() && value >= 0.0).then_some(value)
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
    if style.is_game_hud() {
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
    if style.is_game_hud() {
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
        style if style.is_game_hud() => HUD_GUIDE_ALPHA,
        _ => GUIDE_ALPHA,
    }
}

fn guide_line_width(style: ChartCrosshairStyle) -> f32 {
    match style {
        ChartCrosshairStyle::Classic => CLASSIC_WIDTH,
        style if style.is_game_hud() => 0.85,
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
    use super::{RacingHudMetrics, format_racing_hud_size, format_racing_hud_speed, style_scale};

    #[test]
    fn style_scale_keeps_chart_reticles_full_size() {
        assert_eq!(style_scale(1000.0, 600.0), 1.0);
    }

    #[test]
    fn style_scale_compacts_small_previews() {
        assert_eq!(style_scale(80.0, 40.0), 0.5);
        assert_eq!(style_scale(20.0, 20.0), 0.45);
    }

    #[test]
    fn racing_hud_metrics_report_size_relative_to_max_size() {
        let metrics = RacingHudMetrics::new(Some(2.5), Some(10.0), Some(1_200.0));

        assert_eq!(metrics.usage_ratio(), Some(0.25));
        assert_eq!(metrics.cursor_speed_ratio(), 0.5);
        assert_eq!(format_racing_hud_size(metrics.current_size), "2.5");
        assert_eq!(
            format_racing_hud_speed(metrics.cursor_speed_px_per_s),
            "1.2K"
        );
    }

    #[test]
    fn racing_hud_metrics_handle_missing_or_invalid_max_size() {
        let metrics = RacingHudMetrics::new(Some(2.5), Some(0.0), Some(f32::NAN));

        assert_eq!(metrics.usage_ratio(), None);
        assert_eq!(metrics.cursor_speed_ratio(), 0.0);
        assert_eq!(format_racing_hud_size(metrics.current_size), "2.5");
        assert_eq!(format_racing_hud_speed(metrics.cursor_speed_px_per_s), "--");
    }

    #[test]
    fn default_racing_hud_metrics_do_not_invent_preview_values() {
        let metrics = RacingHudMetrics::default();

        assert_eq!(metrics.usage_ratio(), None);
        assert_eq!(metrics.cursor_speed_ratio(), 0.0);
        assert_eq!(format_racing_hud_size(metrics.current_size), "--");
        assert_eq!(format_racing_hud_speed(metrics.cursor_speed_px_per_s), "--");
    }
}
