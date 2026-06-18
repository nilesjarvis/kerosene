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
// Renders the main price series as a single close-price line with a gradient
// area fill beneath it, in place of candlesticks. Volume bars are drawn
// separately by the candle layer so they remain visible in both modes.
// ---------------------------------------------------------------------------

/// Maximum alpha applied at the top of the price-region area fill.
const AREA_FILL_TOP_ALPHA: f32 = 0.24;
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
            let mut builder = canvas::path::Builder::new();
            builder.move_to(first_base);
            builder.line_to(projected_line[0]);
            for point in &projected_line[1..] {
                builder.line_to(*point);
            }
            builder.line_to(last_base);
            builder.close();

            // Vertical gradient: anchor to the painted series area so the
            // strongest tint follows the line instead of a fixed band at the
            // top of the pane. Keep a minimum fade height so extreme moves do
            // not compress the gradient into a hard edge. NOTE: unlike the
            // line stroke below, this gradient fill does not receive the
            // chromatic-aberration / edge-blur passes (those helpers only
            // accept a flat color, not a gradient); the soft translucent fill
            // makes the difference negligible under those optional lenses.
            let gradient = line_area_gradient(accent, top_y, bottom_y, ctx.price_h);
            frame.fill(&builder.build(), gradient);
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

fn line_area_gradient(
    accent: Color,
    top_y: f32,
    bottom_y: f32,
    price_h: f32,
) -> canvas::gradient::Linear {
    let min_fade_h =
        (price_h * AREA_FILL_MIN_FADE_RATIO).max(AREA_FILL_MIN_FADE_PX.min(price_h.max(1.0)));
    let start_y = top_y.min(bottom_y - min_fade_h).min(bottom_y - 1.0);
    let end_y = bottom_y.max(start_y + 1.0);

    canvas::gradient::Linear::new(Point::new(0.0, start_y), Point::new(0.0, end_y))
        .add_stop(
            0.0,
            Color {
                a: AREA_FILL_TOP_ALPHA,
                ..accent
            },
        )
        .add_stop(1.0, Color { a: 0.0, ..accent })
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
