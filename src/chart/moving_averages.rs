mod series;

use super::CandlestickChart;
use crate::api::Candle;
use crate::chart_indicator::ChartIndicatorId;
pub(super) use series::MovingAverageLayer;
use series::{MovingAverageColorRole, MovingAverageSpec};

// ---------------------------------------------------------------------------
// Moving Average Overlay
// ---------------------------------------------------------------------------

/// Limit overlays to the time covered by their source. In particular, a
/// historical cache can render before the live tail request succeeds.
pub(super) fn candles_through_average_history<'a>(
    displayed: &'a [Candle],
    source: &[Candle],
) -> &'a [Candle] {
    let end = source.last().map_or(0, |last| {
        displayed.partition_point(|c| c.open_time <= last.close_time)
    });
    &displayed[..end]
}

impl CandlestickChart {
    pub(super) fn draw_macro_moving_averages<X, Y>(&self, layer: &mut MovingAverageLayer<'_, X, Y>)
    where
        X: Fn(usize) -> f32,
        Y: Fn(f64) -> f32,
    {
        for key in ChartIndicatorId::MOVING_AVERAGES {
            if !key.is_enabled_in_config(&self.macro_indicators) {
                continue;
            }
            let source: &[Candle] = match key.group() {
                "chart_timeframe" => &self.candles,
                "hourly" => &self.hourly_candles,
                "daily" => &self.daily_candles,
                "weekly" => &self.weekly_candles,
                "monthly" => &self.monthly_candles,
                _ => continue,
            };
            let color = match key {
                ChartIndicatorId::TfSma50
                | ChartIndicatorId::TfEma50
                | ChartIndicatorId::Sma50h
                | ChartIndicatorId::Ema50h
                | ChartIndicatorId::Sma50d
                | ChartIndicatorId::Ema50d => MovingAverageColorRole::Fast,
                ChartIndicatorId::TfSma200
                | ChartIndicatorId::TfEma200
                | ChartIndicatorId::Sma200h
                | ChartIndicatorId::Ema200h
                | ChartIndicatorId::Sma200d
                | ChartIndicatorId::Ema200d => MovingAverageColorRole::Slow,
                ChartIndicatorId::Sma20w | ChartIndicatorId::Ema20w => {
                    MovingAverageColorRole::WeeklyFast
                }
                ChartIndicatorId::Sma50w | ChartIndicatorId::Ema50w => {
                    MovingAverageColorRole::WeeklySlow
                }
                ChartIndicatorId::Sma12m | ChartIndicatorId::Ema12m => {
                    MovingAverageColorRole::Monthly
                }
                _ => continue,
            };
            if let Some(spec) = MovingAverageSpec::new(source, key, color, &self.macro_indicators) {
                layer.draw_average(&self.candles, spec, self.macro_indicators.show_labels);
            }
        }
    }
}
