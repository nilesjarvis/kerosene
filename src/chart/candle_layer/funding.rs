use super::CandleLayerContext;
use crate::chart::model::{
    CandlestickChart, FUNDING_MODE_BUTTON_HEIGHT, FUNDING_MODE_BUTTON_WIDTH, FUNDING_MODE_BUTTON_X,
    FUNDING_MODE_BUTTON_Y_OFFSET, FUNDING_PLOT_BOTTOM_PADDING, FUNDING_PLOT_TOP_PADDING,
    FUNDING_RATE_ANNUALIZATION_FACTOR,
};
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Size};

// ---------------------------------------------------------------------------
// Funding Rate Panel Rendering
// ---------------------------------------------------------------------------

const FUNDING_RANGE_PADDING: f64 = 1.12;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::chart) struct FundingDisplayRange {
    lo: f64,
    hi: f64,
}

impl FundingDisplayRange {
    pub(in crate::chart) fn span(self) -> f64 {
        self.hi - self.lo
    }

    fn finite_span(self) -> Option<f64> {
        let span = self.span();
        (self.lo.is_finite() && self.hi.is_finite() && span.is_finite() && span > 0.0)
            .then_some(span)
    }

    pub(in crate::chart) fn rate_to_y(self, rate: f64, plot_top: f32, plot_bottom: f32) -> f32 {
        let plot_h = plot_bottom - plot_top;
        let Some(span) = self.finite_span() else {
            return (plot_top + plot_bottom) * 0.5;
        };
        if plot_h <= 0.0 || !rate.is_finite() {
            return (plot_top + plot_bottom) * 0.5;
        }
        let y = plot_top as f64 + ((self.hi - rate) / span) * f64::from(plot_h);
        if y.is_finite() {
            y as f32
        } else {
            (plot_top + plot_bottom) * 0.5
        }
    }

    pub(in crate::chart) fn y_to_rate(self, y: f32, plot_top: f32, plot_bottom: f32) -> f64 {
        let plot_h = plot_bottom - plot_top;
        let Some(span) = self.finite_span() else {
            return 0.0;
        };
        if plot_h <= 0.0 || !y.is_finite() {
            return (self.hi + self.lo) * 0.5;
        }
        let ratio = f64::from(y - plot_top) / f64::from(plot_h);
        let rate = self.hi - ratio * span;
        if rate.is_finite() {
            rate
        } else {
            (self.hi + self.lo) * 0.5
        }
    }
}

impl CandlestickChart {
    pub(super) fn draw_funding_panel<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        if ctx.funding_panel_h <= 0.0 {
            return;
        }

        let panel_y = ctx.chart_h;
        frame.fill_rectangle(
            Point::new(0.0, panel_y),
            Size::new(ctx.chart_w, ctx.funding_panel_h),
            Color {
                a: 0.025,
                ..ctx.theme.palette().text
            },
        );

        let separator =
            canvas::Path::line(Point::new(0.0, panel_y), Point::new(ctx.chart_w, panel_y));
        frame.stroke(
            &separator,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.14,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );
        let plot_top = panel_y + FUNDING_PLOT_TOP_PADDING;
        let plot_bottom = panel_y + ctx.funding_panel_h - FUNDING_PLOT_BOTTOM_PADDING;
        if plot_bottom <= plot_top {
            self.draw_funding_panel_chrome(ctx, frame, panel_y);
            return;
        }

        let visible: Vec<_> = self
            .funding_rates
            .iter()
            .filter_map(|point| {
                let x = self.timestamp_to_x(point.time_ms, ctx.state, ctx.chart_w)?;
                (x >= -ctx.step && x <= ctx.chart_w + ctx.step).then_some((x, point))
            })
            .collect();

        if visible.is_empty() {
            if self.funding_rates.is_empty() {
                self.draw_funding_status(ctx, frame, panel_y);
            } else {
                self.draw_funding_message(ctx, frame, panel_y, "No funding in view", false);
            }
            self.draw_funding_panel_chrome(ctx, frame, panel_y);
            return;
        }

        let max_abs = visible
            .iter()
            .map(|(_, point)| self.display_funding_rate(point.rate).abs())
            .fold(0.0_f64, f64::max);
        if max_abs <= 0.0 {
            self.draw_funding_message(ctx, frame, panel_y, "Funding flat", false);
            self.draw_funding_panel_chrome(ctx, frame, panel_y);
            return;
        }

        let Some(display_range) = self.funding_display_range_from_max_abs(max_abs, ctx.state)
        else {
            self.draw_funding_message(ctx, frame, panel_y, "Funding flat", false);
            self.draw_funding_panel_chrome(ctx, frame, panel_y);
            return;
        };
        let baseline_y = display_range.rate_to_y(0.0, plot_top, plot_bottom);
        if baseline_y >= plot_top && baseline_y <= plot_bottom {
            let baseline = canvas::Path::line(
                Point::new(0.0, baseline_y),
                Point::new(ctx.chart_w, baseline_y),
            );
            frame.stroke(
                &baseline,
                canvas::Stroke::default()
                    .with_color(Color {
                        a: 0.10,
                        ..ctx.theme.palette().text
                    })
                    .with_width(1.0),
            );
        }

        let bar_w = (ctx.step * 0.34).clamp(1.0, 4.0);
        let stride = (visible.len() / 2_000).max(1);

        frame.with_clip(
            Rectangle {
                x: 0.0,
                y: plot_top,
                width: ctx.chart_w,
                height: plot_bottom - plot_top,
            },
            |frame| {
                for (x, point) in visible.iter().step_by(stride).copied() {
                    let display_rate = self.display_funding_rate(point.rate);
                    let y = display_range.rate_to_y(display_rate, plot_top, plot_bottom);
                    let top = y.min(baseline_y);
                    let height = (baseline_y - y).abs().max(1.0);
                    let color = if display_rate >= 0.0 {
                        ctx.candle_bull_color
                    } else {
                        ctx.candle_bear_color
                    };
                    frame.fill_rectangle(
                        Point::new(x - bar_w * 0.5, top),
                        Size::new(bar_w, height),
                        Color { a: 0.78, ..color },
                    );
                }
            },
        );

        self.draw_funding_axis_label(ctx, frame, plot_top, display_range.hi);
        if baseline_y >= plot_top + 8.0 && baseline_y <= plot_bottom - 8.0 {
            self.draw_funding_axis_label(ctx, frame, baseline_y, 0.0);
        }
        self.draw_funding_axis_label(ctx, frame, plot_bottom, display_range.lo);
        self.draw_funding_panel_chrome(ctx, frame, panel_y);
    }

    pub(in crate::chart) fn funding_display_range(
        &self,
        state: &crate::chart::ChartState,
        chart_w: f32,
        step: f32,
    ) -> Option<FundingDisplayRange> {
        let max_abs = self
            .funding_rates
            .iter()
            .filter_map(|point| {
                let x = self.timestamp_to_x(point.time_ms, state, chart_w)?;
                (x >= -step && x <= chart_w + step)
                    .then_some(self.display_funding_rate(point.rate).abs())
            })
            .fold(0.0_f64, f64::max);
        self.funding_display_range_from_max_abs(max_abs, state)
    }

    fn funding_display_range_from_max_abs(
        &self,
        max_abs: f64,
        state: &crate::chart::ChartState,
    ) -> Option<FundingDisplayRange> {
        if max_abs <= 0.0 || !max_abs.is_finite() {
            return None;
        }

        if state.funding_y_scale <= 0.0 || !state.funding_y_scale.is_finite() {
            return None;
        }

        let half_range = max_abs * FUNDING_RANGE_PADDING * state.funding_y_scale;
        if half_range <= 0.0 || !half_range.is_finite() {
            return None;
        }
        let half_range = half_range.max(f64::EPSILON);
        let center = self.display_funding_rate(state.funding_y_offset);
        let lo = center - half_range;
        let hi = center + half_range;
        let range = FundingDisplayRange { lo, hi };
        if center.is_finite() && range.finite_span().is_some() {
            Some(range)
        } else {
            None
        }
    }

    fn draw_funding_mode_button<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        panel_y: f32,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let origin = Point::new(
            FUNDING_MODE_BUTTON_X,
            panel_y + FUNDING_MODE_BUTTON_Y_OFFSET,
        );
        let size = Size::new(FUNDING_MODE_BUTTON_WIDTH, FUNDING_MODE_BUTTON_HEIGHT);
        let bg = if self.funding_annualized {
            Color {
                a: 0.20,
                ..ctx.theme.palette().primary
            }
        } else {
            Color {
                a: 0.10,
                ..ctx.theme.palette().text
            }
        };
        frame.fill_rectangle(origin, size, bg);
        let border = canvas::Path::rectangle(origin, size);
        frame.stroke(
            &border,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.18,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );
        frame.fill_text(canvas::Text {
            content: if self.funding_annualized {
                "APR".to_string()
            } else {
                "1H".to_string()
            },
            position: Point::new(origin.x + size.width * 0.5, origin.y + size.height * 0.5),
            color: Color {
                a: 0.82,
                ..ctx.theme.palette().text
            },
            size: iced::Pixels(9.0),
            align_x: alignment::Horizontal::Center.into(),
            align_y: alignment::Vertical::Center,
            font: iced::Font::MONOSPACE,
            ..canvas::Text::default()
        });
    }

    fn draw_funding_panel_chrome<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        panel_y: f32,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        for offset in [-8.0_f32, 0.0, 8.0] {
            frame.fill_rectangle(
                Point::new(ctx.chart_w * 0.5 + offset - 2.5, panel_y + 2.0),
                Size::new(5.0, 1.0),
                Color {
                    a: 0.28,
                    ..ctx.theme.palette().text
                },
            );
        }
        self.draw_funding_mode_button(ctx, frame, panel_y);
    }

    fn draw_funding_status<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        panel_y: f32,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let (label, is_error) = self
            .funding_status
            .as_ref()
            .map(|(label, is_error)| (label.as_str(), *is_error))
            .unwrap_or(("Funding waiting for data", false));
        self.draw_funding_message(ctx, frame, panel_y, label, is_error);
    }

    fn draw_funding_message<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        panel_y: f32,
        label: &str,
        is_error: bool,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let color = if is_error {
            ctx.theme.palette().danger
        } else {
            Color {
                a: 0.45,
                ..ctx.theme.palette().text
            }
        };
        frame.fill_text(canvas::Text {
            content: label.to_string(),
            position: Point::new(
                FUNDING_MODE_BUTTON_X + FUNDING_MODE_BUTTON_WIDTH + 8.0,
                panel_y + ctx.funding_panel_h * 0.5,
            ),
            color,
            size: iced::Pixels(10.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: iced::Font::MONOSPACE,
            ..canvas::Text::default()
        });
    }

    fn draw_funding_axis_label<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        y: f32,
        rate: f64,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        frame.fill_text(canvas::Text {
            content: format_funding_rate_percent(rate, self.funding_annualized),
            position: Point::new(ctx.chart_w + 6.0, y),
            color: Color {
                a: 0.42,
                ..ctx.theme.palette().text
            },
            size: iced::Pixels(9.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: iced::Font::MONOSPACE,
            ..canvas::Text::default()
        });
    }

    pub(in crate::chart) fn display_funding_rate(&self, hourly_rate: f64) -> f64 {
        if self.funding_annualized {
            hourly_rate * FUNDING_RATE_ANNUALIZATION_FACTOR
        } else {
            hourly_rate
        }
    }
}

pub(in crate::chart) fn format_funding_rate_percent(rate: f64, annualized: bool) -> String {
    if annualized {
        format!("{:+.2}%", rate * 100.0)
    } else {
        format!("{:+.5}%", rate * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Candle;
    use crate::chart::model::DEFAULT_FUNDING_PANEL_HEIGHT;
    use crate::chart::state::ChartState;
    use crate::hydromancer_api::FundingRatePoint;

    fn candle(open_time: u64) -> Candle {
        Candle {
            open_time,
            close_time: open_time + 59_999,
            open: 1.0,
            high: 1.0,
            low: 1.0,
            close: 1.0,
            volume: 1.0,
        }
    }

    fn chart_with_funding() -> CandlestickChart {
        let mut chart = CandlestickChart::new(1);
        chart.set_candles(vec![candle(1_000), candle(61_000), candle(121_000)]);
        chart.set_funding_history(vec![FundingRatePoint {
            time_ms: 61_000,
            rate: 0.01,
        }]);
        chart
    }

    #[test]
    fn zoomed_funding_range_maps_values_beyond_plot_without_clamping() {
        let chart = chart_with_funding();
        let state = ChartState {
            funding_y_scale: 0.5,
            ..ChartState::default()
        };

        let range = chart
            .funding_display_range(&state, 400.0, 12.0)
            .expect("funding range");
        let y = range.rate_to_y(0.01, 24.0, 80.0);

        assert!(
            y < 24.0,
            "zoomed funding point should map above the plot, got {y}"
        );
    }

    #[test]
    fn funding_range_uses_offset_as_visible_center() {
        let chart = chart_with_funding();
        let state = ChartState {
            funding_y_offset: 0.002,
            ..ChartState::default()
        };

        let range = chart
            .funding_display_range(&state, 400.0, 12.0)
            .expect("funding range");

        assert!(((range.hi + range.lo) * 0.5 - 0.002).abs() < 1e-12);
    }

    #[test]
    fn oversized_funding_range_is_rejected() {
        let chart = chart_with_funding();
        let state = ChartState::default();

        let range = chart.funding_display_range_from_max_abs(1.7e308, &state);

        assert!(range.is_none());
    }

    #[test]
    fn invalid_funding_range_falls_back_to_finite_coordinates() {
        let range = FundingDisplayRange {
            lo: f64::NEG_INFINITY,
            hi: f64::INFINITY,
        };

        assert_eq!(range.rate_to_y(0.0, 24.0, 80.0), 52.0);
        assert_eq!(range.y_to_rate(52.0, 24.0, 80.0), 0.0);
    }

    #[test]
    fn default_funding_plot_uses_space_behind_mode_button() {
        let button_bottom = FUNDING_MODE_BUTTON_Y_OFFSET + FUNDING_MODE_BUTTON_HEIGHT;
        let plot_h =
            DEFAULT_FUNDING_PANEL_HEIGHT - FUNDING_PLOT_TOP_PADDING - FUNDING_PLOT_BOTTOM_PADDING;

        assert!(FUNDING_PLOT_TOP_PADDING < button_bottom);
        assert!(plot_h >= 40.0, "default funding plot height was {plot_h}");
    }
}
