use super::CandleLayerContext;
use crate::chart::model::{CandlestickChart, EarningsMarker};
use crate::chart::state::ChartState;
use crate::helpers::text_color_for_bg;
use iced::widget::canvas;
use iced::{Color, Point, alignment};

// ---------------------------------------------------------------------------
// Earnings Marker Rendering
// ---------------------------------------------------------------------------

const EARNINGS_LINE_ALPHA: f32 = 0.2;
const EARNINGS_DOT_ALPHA: f32 = 0.88;
pub(in crate::chart) const EARNINGS_DOT_RADIUS: f32 = 9.5;
const EARNINGS_DOT_BOTTOM_PADDING: f32 = 11.0;
const EARNINGS_DOT_HIT_RADIUS: f32 = 12.0;
const EARNINGS_DOT_LABEL_SIZE: f32 = 7.0;

impl CandlestickChart {
    pub(super) fn draw_earnings_markers<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        if self.earnings_markers.is_empty() || ctx.chart_w <= 0.0 || ctx.price_h <= 0.0 {
            return;
        }

        let positions =
            visible_earnings_marker_xs(&self.earnings_markers, ctx.chart_w, |time_ms| {
                self.timestamp_to_x(time_ms, ctx.state, ctx.chart_w)
            });
        if positions.is_empty() {
            return;
        }

        let marker_color = ctx.theme.palette().primary;
        let line_color = Color {
            a: EARNINGS_LINE_ALPHA,
            ..marker_color
        };
        let dot_color = Color {
            a: EARNINGS_DOT_ALPHA,
            ..marker_color
        };
        let dot_y = earnings_marker_dot_y(ctx.price_h);

        for (x, marker) in positions {
            ctx.fisheye.stroke_projected_line_without_edge_blur(
                frame,
                Point::new(x, 0.0),
                Point::new(x, ctx.price_h),
                canvas::Stroke::default()
                    .with_color(line_color)
                    .with_width(1.0),
            );
            let marker_center = Point::new(x, dot_y);
            ctx.fisheye.fill_projected_circle_without_edge_blur(
                frame,
                marker_center,
                EARNINGS_DOT_RADIUS,
                dot_color,
            );
            ctx.fisheye.stroke_projected_circle_without_edge_blur(
                frame,
                marker_center,
                EARNINGS_DOT_RADIUS,
                canvas::Stroke::default()
                    .with_color(Color {
                        a: 0.34,
                        ..ctx.theme.palette().text
                    })
                    .with_width(0.75),
            );
            if let Some(label) = compact_earnings_form_label(&marker.form) {
                let visual_center = ctx.fisheye.project(marker_center);
                frame.fill_text(canvas::Text {
                    content: label,
                    position: visual_center,
                    color: text_color_for_bg(dot_color),
                    size: iced::Pixels(EARNINGS_DOT_LABEL_SIZE),
                    align_x: alignment::Horizontal::Center.into(),
                    align_y: alignment::Vertical::Center,
                    font: crate::app_fonts::monospace_font(),
                    ..canvas::Text::default()
                });
            }
        }
    }

    pub(in crate::chart) fn hit_test_earnings_marker_at(
        &self,
        state: &ChartState,
        pos: Point,
        chart_w: f32,
        price_h: f32,
    ) -> Option<u64> {
        if self.earnings_markers.is_empty()
            || chart_w <= 0.0
            || price_h <= 0.0
            || !pos.x.is_finite()
            || !pos.y.is_finite()
            || pos.x < -EARNINGS_DOT_HIT_RADIUS
            || pos.x > chart_w + EARNINGS_DOT_HIT_RADIUS
            || pos.y < -EARNINGS_DOT_HIT_RADIUS
            || pos.y > price_h + EARNINGS_DOT_HIT_RADIUS
        {
            return None;
        }

        let dot_y = earnings_marker_dot_y(price_h);
        let hit_radius_sq = EARNINGS_DOT_HIT_RADIUS * EARNINGS_DOT_HIT_RADIUS;
        let mut best: Option<(u64, f32)> = None;
        for (x, marker) in visible_earnings_marker_xs(&self.earnings_markers, chart_w, |time_ms| {
            self.timestamp_to_x(time_ms, state, chart_w)
        }) {
            let dx = pos.x - x;
            let dy = pos.y - dot_y;
            let distance_sq = dx * dx + dy * dy;
            if distance_sq <= hit_radius_sq
                && best.is_none_or(|(_, best_distance_sq)| distance_sq < best_distance_sq)
            {
                best = Some((marker.time_ms, distance_sq));
            }
        }
        best.map(|(time_ms, _)| time_ms)
    }
}

pub(in crate::chart) fn earnings_marker_dot_y(price_h: f32) -> f32 {
    (price_h - EARNINGS_DOT_BOTTOM_PADDING).max(EARNINGS_DOT_RADIUS)
}

fn compact_earnings_form_label(form: &str) -> Option<String> {
    let form = form
        .trim()
        .split_once('/')
        .map_or(form.trim(), |(base, _)| base);
    let compact = form
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .take(4)
        .map(|ch| ch.to_ascii_uppercase())
        .collect::<String>();
    (!compact.is_empty()).then_some(compact)
}

fn visible_earnings_marker_xs<F>(
    markers: &[EarningsMarker],
    chart_w: f32,
    mut timestamp_to_x: F,
) -> Vec<(f32, &EarningsMarker)>
where
    F: FnMut(u64) -> Option<f32>,
{
    if chart_w <= 0.0 || !chart_w.is_finite() {
        return Vec::new();
    }

    markers
        .iter()
        .filter_map(|marker| {
            let x = timestamp_to_x(marker.time_ms)?;
            (x.is_finite() && x >= 0.0 && x <= chart_w).then_some((x, marker))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(time_ms: u64) -> EarningsMarker {
        EarningsMarker {
            time_ms,
            cik: 1_652_044,
            form: "8-K".to_string(),
            filing_date: String::new(),
            accession_number: String::new(),
            primary_document: String::new(),
            quarter_label: None,
            filing_summary: None,
            filing_summary_status: None,
            filing_summary_loading: false,
        }
    }

    #[test]
    fn visible_earnings_marker_xs_filters_outside_plot_bounds() {
        let markers = vec![marker(1), marker(2), marker(3), marker(4)];

        let positions = visible_earnings_marker_xs(&markers, 100.0, |time_ms| match time_ms {
            1 => Some(-1.0),
            2 => Some(25.0),
            3 => Some(101.0),
            4 => Some(100.0),
            _ => None,
        });

        assert_eq!(
            positions
                .into_iter()
                .map(|(x, marker)| (x, marker.time_ms))
                .collect::<Vec<_>>(),
            vec![(25.0, 2), (100.0, 4)]
        );
    }

    #[test]
    fn hit_test_earnings_marker_uses_expanded_dot_target() {
        let mut chart = CandlestickChart::new(1);
        chart.set_candles(vec![
            crate::api::Candle {
                open_time: 1_000,
                close_time: 1_999,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 10.0,
            },
            crate::api::Candle {
                open_time: 2_000,
                close_time: 2_999,
                open: 101.0,
                high: 102.0,
                low: 100.0,
                close: 101.0,
                volume: 10.0,
            },
        ]);
        chart.set_earnings_markers(vec![marker(2_000)]);
        let state = ChartState::default();
        let chart_w = 400.0;
        let price_h = 160.0;
        let marker_x = chart
            .timestamp_to_x(2_000, &state, chart_w)
            .expect("marker x");
        let marker_y = earnings_marker_dot_y(price_h);

        assert_eq!(
            chart.hit_test_earnings_marker_at(
                &state,
                Point::new(marker_x + 10.0, marker_y),
                chart_w,
                price_h,
            ),
            Some(2_000)
        );
        assert_eq!(
            chart.hit_test_earnings_marker_at(
                &state,
                Point::new(marker_x + 15.0, marker_y),
                chart_w,
                price_h,
            ),
            None
        );
    }

    #[test]
    fn compact_earnings_form_label_fits_common_sec_forms() {
        assert_eq!(compact_earnings_form_label("8-K").as_deref(), Some("8K"));
        assert_eq!(compact_earnings_form_label("10-Q").as_deref(), Some("10Q"));
        assert_eq!(
            compact_earnings_form_label("10-K/A").as_deref(),
            Some("10K")
        );
        assert_eq!(compact_earnings_form_label(" ").as_deref(), None);
    }
}
