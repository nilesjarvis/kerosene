use super::CandleLayerContext;
use crate::api::Candle;
use crate::chart::fisheye::ProjectedPathPoint;
use crate::chart::model::CandlestickChart;
use crate::chart::price_range::{VisiblePriceStats, visible_price_stats};
use crate::helpers::format_price;
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Secondary Comparison Series Rendering
// ---------------------------------------------------------------------------

const SECONDARY_LINE_WIDTH: f32 = 1.4;
const SECONDARY_AXIS_LABEL_STEPS: usize = 3;

impl CandlestickChart {
    pub(super) fn draw_secondary_series<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let Some(series) = self.secondary_series.as_ref() else {
            return;
        };
        let Some(stats) = self.secondary_visible_price_stats(ctx) else {
            return;
        };

        let color = secondary_series_color(ctx.theme);
        let price_to_y =
            |price| self.price_to_y_with(price, stats.price_hi, stats.price_range, ctx.price_h);
        let points = secondary_close_points(
            &series.candles,
            ctx.state,
            ctx.chart_w,
            |ts| self.timestamp_to_x(ts, ctx.state, ctx.chart_w),
            &price_to_y,
        );
        if points.len() < 2 {
            return;
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
            .with_color(color)
            .with_width(SECONDARY_LINE_WIDTH);
        ctx.fisheye
            .stroke_projected_path_points(frame, &projected, stroke);
    }

    pub(super) fn draw_secondary_price_axis_labels<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let Some(series) = self.secondary_series.as_ref() else {
            return;
        };
        let Some(stats) = self.secondary_visible_price_stats(ctx) else {
            return;
        };

        let color = secondary_series_color(ctx.theme);
        let label = if let Some(last) = series.candles.last() {
            format!("{} {}", series.symbol_label, format_price(last.close))
        } else {
            series.symbol_label.clone()
        };

        frame.fill_text(canvas::Text {
            content: label,
            position: Point::new(ctx.chart_w - 6.0, 10.0),
            color,
            size: iced::Pixels(11.0),
            align_x: alignment::Horizontal::Right.into(),
            align_y: alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..canvas::Text::default()
        });

        for i in 0..=SECONDARY_AXIS_LABEL_STEPS {
            let frac = i as f32 / SECONDARY_AXIS_LABEL_STEPS as f32;
            let y = frac * ctx.price_h;
            let price_val = if self.inverted {
                stats.price_lo + (frac as f64) * stats.price_range
            } else {
                stats.price_hi - (frac as f64) * stats.price_range
            };
            frame.fill_text(canvas::Text {
                content: format_price(price_val),
                position: Point::new(ctx.chart_w - 6.0, y),
                color: Color { a: 0.72, ..color },
                size: iced::Pixels(10.0),
                align_x: alignment::Horizontal::Right.into(),
                align_y: alignment::Vertical::Center,
                font: crate::app_fonts::monospace_font(),
                ..canvas::Text::default()
            });
        }
    }

    fn secondary_visible_price_stats<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
    ) -> Option<VisiblePriceStats>
    where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let series = self.secondary_series.as_ref()?;
        let left_ts = self.x_to_timestamp(0.0, ctx.state, ctx.chart_w)?;
        let right_ts = self.x_to_timestamp(ctx.chart_w, ctx.state, ctx.chart_w)?;
        secondary_visible_price_stats(
            &series.candles,
            left_ts.min(right_ts),
            left_ts.max(right_ts),
        )
    }
}

fn secondary_series_color(theme: &iced::Theme) -> Color {
    Color {
        a: 0.88,
        ..theme.palette().primary
    }
}

fn secondary_visible_price_stats(
    candles: &[Candle],
    start_time_ms: u64,
    end_time_ms: u64,
) -> Option<VisiblePriceStats> {
    if end_time_ms <= start_time_ms {
        return None;
    }
    let visible = candles
        .iter()
        .filter(|candle| candle.close_time >= start_time_ms && candle.open_time <= end_time_ms)
        .cloned()
        .collect::<Vec<_>>();
    visible_price_stats(&visible, true, 1.0, 0.0)
}

fn secondary_close_points<X, Y>(
    candles: &[Candle],
    state: &crate::chart::ChartState,
    chart_w: f32,
    timestamp_to_x: X,
    price_to_y: &Y,
) -> Vec<Point>
where
    X: Fn(u64) -> Option<f32>,
    Y: Fn(f64) -> f32,
{
    if candles.is_empty() || chart_w <= 0.0 || !chart_w.is_finite() {
        return Vec::new();
    }

    let half_step = state.candle_width.max(1.0) * 0.5;
    candles
        .iter()
        .filter_map(|candle| {
            let x = timestamp_to_x(candle.open_time)?;
            (x + half_step >= 0.0 && x - half_step <= chart_w)
                .then(|| Point::new(x, price_to_y(candle.close)))
        })
        .collect()
}
