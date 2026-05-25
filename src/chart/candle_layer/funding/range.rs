use crate::chart::ChartState;
use crate::chart::model::{CandlestickChart, FUNDING_RATE_ANNUALIZATION_FACTOR};

// ---------------------------------------------------------------------------
// Funding Display Range
// ---------------------------------------------------------------------------

const FUNDING_RANGE_PADDING: f64 = 1.12;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::chart) struct FundingDisplayRange {
    pub(super) lo: f64,
    pub(super) hi: f64,
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
    pub(in crate::chart) fn funding_display_range(
        &self,
        state: &ChartState,
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

    pub(super) fn funding_display_range_from_max_abs(
        &self,
        max_abs: f64,
        state: &ChartState,
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

    pub(super) fn display_funding_rate(&self, hourly_rate: f64) -> f64 {
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
