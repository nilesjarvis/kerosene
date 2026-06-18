use super::CandleLayerContext;
use crate::api::Candle;
use crate::chart::fisheye::ProjectedPathPoint;
use crate::chart::model::CandlestickChart;
use iced::widget::canvas;
use iced::{Color, Point};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Line Series Rendering
//
// Renders the main price series as a single close-price line with a layered
// area fade beneath it, in place of candlesticks. Volume bars are drawn
// separately by the candle layer so they remain visible in both modes.
// ---------------------------------------------------------------------------

/// Maximum alpha applied at the top of the price-region area fill.
const AREA_FILL_TOP_ALPHA: f32 = 0.24;
/// Number of solid masks used to approximate the area-fill fade.
const AREA_FILL_LAYERS: usize = 64;
/// Minimum fraction of the price region used for the area-fill fade.
const AREA_FILL_MIN_FADE_RATIO: f32 = 0.55;
/// Minimum pixel height used for the area-fill fade when space permits.
const AREA_FILL_MIN_FADE_PX: f32 = 120.0;
/// Stroke width of the close-price line.
const LINE_WIDTH: f32 = 1.5;

impl CandlestickChart {
    pub(super) fn draw_line_series<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let points = visible_close_points(
            &self.candles,
            ctx.first_vis,
            ctx.last_vis,
            ctx.candle_w,
            ctx.chart_w,
            ctx.idx_to_cx,
            ctx.price_to_y,
        );
        if points.len() < 2 {
            return;
        }

        let (line_color, accent) = line_series_colors(self, ctx.theme);

        // Area fill: a polygon bounded above by the close-price line and below
        // by the price-region baseline. Work entirely in projected (frame) space
        // so the vertical gradient axis stays anchored to the painted geometry
        // when the fisheye lens distorts the baseline (with the lens off,
        // `project` is the identity and this is the plain unprojected polygon).
        let baseline_y = ctx.price_h;
        let projected_line: Vec<Point> = points
            .iter()
            .map(|point| ctx.fisheye.project(*point))
            .collect();
        let first_base = ctx.fisheye.project(Point::new(points[0].x, baseline_y));
        let last_base = ctx
            .fisheye
            .project(Point::new(points[points.len() - 1].x, baseline_y));
        let top_y = projected_line
            .iter()
            .map(|point| point.y)
            .fold(f32::INFINITY, f32::min);
        let bottom_y = first_base.y.max(last_base.y);
        if bottom_y > top_y {
            let mut area_points = Vec::with_capacity(projected_line.len() + 2);
            area_points.push(first_base);
            area_points.extend(projected_line.iter().copied());
            area_points.push(last_base);

            // Area fade: approximate the vertical gradient with nested solid
            // masks. iced's canvas gradient shader dithers after
            // premultiplication; with dark accents fading to transparent on
            // light themes, that can read as a color shelf instead of a clean
            // fade. Solid masks avoid that shader path while preserving the
            // smooth translucent area under the line.
            draw_line_area_fade(frame, &area_points, accent, top_y, bottom_y, ctx.price_h);
        }

        let projected: Vec<ProjectedPathPoint> = points
            .iter()
            .enumerate()
            .map(|(index, point)| ProjectedPathPoint {
                point: *point,
                starts_segment: index == 0,
            })
            .collect();
        let stroke = canvas::Stroke::default()
            .with_color(line_color)
            .with_width(LINE_WIDTH);
        ctx.fisheye
            .stroke_projected_path_points(frame, &projected, stroke);
    }
}

pub(in crate::chart) fn line_series_stroke_color(
    chart: &CandlestickChart,
    theme: &iced::Theme,
) -> Color {
    chart.chart_line_color.unwrap_or(theme.palette().text)
}

fn line_series_colors(chart: &CandlestickChart, theme: &iced::Theme) -> (Color, Color) {
    (
        line_series_stroke_color(chart, theme),
        chart
            .chart_line_gradient_color
            .or(chart.chart_line_color)
            .unwrap_or(theme.extended_palette().primary.base.color),
    )
}

fn draw_line_area_fade(
    frame: &mut canvas::Frame,
    area_points: &[Point],
    accent: Color,
    top_y: f32,
    bottom_y: f32,
    price_h: f32,
) {
    let Some((start_y, end_y)) = line_area_fade_bounds(top_y, bottom_y, price_h) else {
        return;
    };
    let fade_h = end_y - start_y;
    if fade_h <= f32::EPSILON {
        return;
    }

    let color = Color {
        a: line_area_layer_alpha(),
        ..accent
    };

    for layer in 0..AREA_FILL_LAYERS {
        let t = (layer + 1) as f32 / (AREA_FILL_LAYERS + 1) as f32;
        let clip_y = start_y + fade_h * t;
        let clipped = clip_polygon_to_max_y(area_points, clip_y);
        fill_polygon(frame, &clipped, color);
    }
}

fn line_area_fade_bounds(top_y: f32, bottom_y: f32, price_h: f32) -> Option<(f32, f32)> {
    if !top_y.is_finite()
        || !bottom_y.is_finite()
        || !price_h.is_finite()
        || bottom_y <= top_y
        || price_h <= 0.0
    {
        return None;
    }

    let min_fade_h =
        (price_h * AREA_FILL_MIN_FADE_RATIO).max(AREA_FILL_MIN_FADE_PX.min(price_h.max(1.0)));
    let start_y = top_y.min(bottom_y - min_fade_h).min(bottom_y - 1.0);
    let end_y = bottom_y.max(start_y + 1.0);

    Some((start_y, end_y))
}

fn line_area_layer_alpha() -> f32 {
    1.0 - (1.0 - AREA_FILL_TOP_ALPHA).powf(1.0 / AREA_FILL_LAYERS as f32)
}

fn clip_polygon_to_max_y(points: &[Point], max_y: f32) -> Vec<Point> {
    if points.len() < 3 || !max_y.is_finite() {
        return Vec::new();
    }

    let mut clipped = Vec::with_capacity(points.len() + 2);
    let mut previous = *points.last().unwrap_or(&Point::ORIGIN);
    let mut previous_inside = previous.y <= max_y;

    for current in points.iter().copied() {
        let current_inside = current.y <= max_y;
        if current_inside != previous_inside {
            clipped.push(segment_y_intersection(previous, current, max_y));
        }
        if current_inside {
            clipped.push(current);
        }

        previous = current;
        previous_inside = current_inside;
    }

    clipped
}

fn segment_y_intersection(start: Point, end: Point, y: f32) -> Point {
    let dy = end.y - start.y;
    if dy.abs() <= f32::EPSILON {
        return Point::new(start.x, y);
    }

    let t = ((y - start.y) / dy).clamp(0.0, 1.0);
    Point::new(start.x + (end.x - start.x) * t, y)
}

fn fill_polygon(frame: &mut canvas::Frame, points: &[Point], color: Color) {
    if points.len() < 3 {
        return;
    }

    let path = canvas::Path::new(|path| {
        path.move_to(points[0]);
        for point in &points[1..] {
            path.line_to(*point);
        }
        path.close();
    });
    frame.fill(&path, color);
}

/// Build the visible close-price polyline points (chart-space, pre-projection),
/// skipping candles whose body falls entirely outside the horizontal plot.
fn visible_close_points<IdxToCx, PriceToY>(
    candles: &[Candle],
    first_vis: usize,
    last_vis: usize,
    candle_w: f32,
    chart_w: f32,
    idx_to_cx: &IdxToCx,
    price_to_y: &PriceToY,
) -> Vec<Point>
where
    IdxToCx: Fn(usize) -> f32,
    PriceToY: Fn(f64) -> f32,
{
    if candles.is_empty() || first_vis > last_vis {
        return Vec::new();
    }
    let last_vis = last_vis.min(candles.len() - 1);
    if first_vis > last_vis {
        return Vec::new();
    }

    let mut points = Vec::with_capacity(last_vis - first_vis + 1);
    for (relative_index, candle) in candles[first_vis..=last_vis].iter().enumerate() {
        let i = first_vis + relative_index;
        let cx = idx_to_cx(i);
        if cx + candle_w * 0.5 < 0.0 || cx - candle_w * 0.5 > chart_w {
            continue;
        }
        points.push(Point::new(cx, price_to_y(candle.close)));
    }
    points
}
