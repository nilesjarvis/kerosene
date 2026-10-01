use super::super::indicators::{calculate_ema, calculate_sma};
use crate::api::Candle;
use crate::chart::fisheye::{ChartFisheye, ProjectedPathPoint};
use crate::chart_indicator::ChartIndicatorId;
use crate::config::MacroIndicatorsConfig;
use iced::widget::canvas;
use iced::{Color, Point, Size, Theme, alignment};

mod points;

use points::{AveragePointContext, visible_average_points};

// ---------------------------------------------------------------------------
// Moving Average Series Drawing
// ---------------------------------------------------------------------------

const EMA_DASH: [f32; 2] = [4.0, 4.0];

pub(in crate::chart) struct MovingAverageLayer<'a, X, Y>
where
    X: Fn(usize) -> f32,
    Y: Fn(f64) -> f32,
{
    pub(in crate::chart) frame: &'a mut canvas::Frame,
    pub(in crate::chart) theme: &'a Theme,
    pub(in crate::chart) first_vis: usize,
    pub(in crate::chart) last_vis: usize,
    pub(in crate::chart) chart_w: f32,
    pub(in crate::chart) candle_w: f32,
    pub(in crate::chart) fisheye: ChartFisheye,
    pub(in crate::chart) idx_to_cx: &'a X,
    pub(in crate::chart) price_to_y: &'a Y,
}

pub(super) struct MovingAverageSpec<'a> {
    source_candles: &'a [Candle],
    period: usize,
    use_ema: bool,
    color_role: MovingAverageColorRole,
    label: String,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum MovingAverageColorRole {
    Fast,
    Slow,
    WeeklyFast,
    WeeklySlow,
    Monthly,
}

impl MovingAverageColorRole {
    fn color(self, theme: &Theme) -> Color {
        let extended = theme.extended_palette();

        match self {
            Self::Fast => extended.warning.base.color,
            Self::Slow => extended.primary.base.color,
            Self::WeeklyFast => extended.success.base.color,
            Self::WeeklySlow => extended.secondary.strong.color,
            Self::Monthly => extended.danger.base.color,
        }
    }
}

impl<'a> MovingAverageSpec<'a> {
    pub(super) fn new(
        source_candles: &'a [Candle],
        key: ChartIndicatorId,
        color_role: MovingAverageColorRole,
        config: &MacroIndicatorsConfig,
    ) -> Option<Self> {
        Some(Self {
            source_candles,
            period: key.period(config)?,
            use_ema: key.key().contains("ema"),
            color_role,
            label: key.moving_average_label(config)?,
        })
    }
}

impl<'a, X, Y> MovingAverageLayer<'a, X, Y>
where
    X: Fn(usize) -> f32,
    Y: Fn(f64) -> f32,
{
    pub(super) fn draw_average(
        &mut self,
        chart_candles: &[Candle],
        spec: MovingAverageSpec<'_>,
        show_labels: bool,
    ) {
        let series = if spec.use_ema {
            calculate_ema(spec.source_candles, spec.period)
        } else {
            calculate_sma(spec.source_candles, spec.period)
        };
        let dash_segments = if spec.use_ema { &EMA_DASH[..] } else { &[] };
        let color = spec.color_role.color(self.theme);
        self.draw_series(
            chart_candles,
            &series,
            color,
            &spec.label,
            dash_segments,
            show_labels,
        );
    }

    fn draw_series(
        &mut self,
        chart_candles: &[Candle],
        ma_series: &[(u64, f64)],
        color: Color,
        label: &str,
        dash_segments: &[f32],
        show_labels: bool,
    ) {
        if ma_series.is_empty() || chart_candles.is_empty() {
            return;
        }

        let path_points = visible_average_points(AveragePointContext {
            chart_candles,
            ma_series,
            first_vis: self.first_vis,
            last_vis: self.last_vis,
            chart_w: self.chart_w,
            candle_w: self.candle_w,
            idx_to_cx: self.idx_to_cx,
            price_to_y: self.price_to_y,
        });
        let mut projected_points = Vec::new();
        let mut last_pt = None;

        for path_point in path_points {
            projected_points.push(ProjectedPathPoint {
                point: path_point.point,
                starts_segment: path_point.starts_segment,
            });
            last_pt = Some(path_point.point);
        }

        let stroke_color = Color { a: 0.7, ..color };
        let mut stroke = canvas::Stroke::default()
            .with_color(stroke_color)
            .with_width(2.0);
        if !dash_segments.is_empty() {
            stroke.line_dash = canvas::stroke::LineDash {
                segments: dash_segments,
                offset: 0,
            };
        }

        self.fisheye
            .stroke_projected_path_points(self.frame, &projected_points, stroke);

        if show_labels && let Some(pt) = last_pt {
            let pt = self.fisheye.project(pt);
            let label_w = label.len() as f32 * 6.0 + 4.0;
            let label_h = 14.0;
            let label_x = pt.x - label_w;
            let label_y = pt.y - 4.0 - label_h;

            self.frame.fill_rectangle(
                Point::new(label_x, label_y),
                Size::new(label_w, label_h),
                Color {
                    a: 0.6,
                    ..self.theme.extended_palette().background.strong.color
                },
            );

            self.frame.fill_text(canvas::Text {
                content: label.to_string(),
                position: Point::new(pt.x - 2.0, pt.y - 4.0),
                color,
                size: iced::Pixels(11.0),
                align_x: alignment::Horizontal::Right.into(),
                align_y: alignment::Vertical::Bottom,
                font: crate::app_fonts::monospace_font(),
                ..canvas::Text::default()
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_average_specs_use_custom_periods_for_sma_and_ema() {
        let candles = vec![
            Candle::test_flat(1_000, 10.0),
            Candle::test_flat(2_000, 20.0),
            Candle::test_flat(3_000, 50.0),
        ];
        for key in ChartIndicatorId::MOVING_AVERAGES {
            let mut config = MacroIndicatorsConfig::default();
            config
                .moving_average_periods
                .insert(key.key().to_string(), 2);
            let spec = MovingAverageSpec::new(&candles, key, MovingAverageColorRole::Fast, &config)
                .expect("moving average slot");
            assert_eq!(spec.period, 2);
            assert!(spec.label.contains('2'));
            let series = if spec.use_ema {
                calculate_ema(spec.source_candles, spec.period)
            } else {
                calculate_sma(spec.source_candles, spec.period)
            };
            assert_eq!(series.len(), 2);
            assert_eq!(series[0], (2_000, 15.0));
            let expected = if key.key().contains("ema") {
                115.0 / 3.0
            } else {
                35.0
            };
            assert!((series[1].1 - expected).abs() < 1e-10, "{}", key.key());
        }
    }
}
