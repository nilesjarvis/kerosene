mod series;

use super::CandlestickChart;
pub(super) use series::MovingAverageLayer;
use series::{MovingAverageColorRole, MovingAverageSpec};

// ---------------------------------------------------------------------------
// Moving Average Overlay
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_macro_moving_averages<X, Y>(&self, layer: &mut MovingAverageLayer<'_, X, Y>)
    where
        X: Fn(usize) -> f32,
        Y: Fn(f64) -> f32,
    {
        let show_labels = self.macro_indicators.show_labels;

        if self.macro_indicators.tf_sma_50 {
            layer.draw_average(
                &self.candles,
                MovingAverageSpec::sma(
                    &self.candles,
                    50,
                    MovingAverageColorRole::Fast,
                    "TF 50 SMA",
                ),
                show_labels,
            );
        }
        if self.macro_indicators.tf_ema_50 {
            layer.draw_average(
                &self.candles,
                MovingAverageSpec::ema(
                    &self.candles,
                    50,
                    MovingAverageColorRole::Fast,
                    "TF 50 EMA",
                ),
                show_labels,
            );
        }
        if self.macro_indicators.tf_sma_200 {
            layer.draw_average(
                &self.candles,
                MovingAverageSpec::sma(
                    &self.candles,
                    200,
                    MovingAverageColorRole::Slow,
                    "TF 200 SMA",
                ),
                show_labels,
            );
        }
        if self.macro_indicators.tf_ema_200 {
            layer.draw_average(
                &self.candles,
                MovingAverageSpec::ema(
                    &self.candles,
                    200,
                    MovingAverageColorRole::Slow,
                    "TF 200 EMA",
                ),
                show_labels,
            );
        }
        if self.macro_indicators.sma_50d {
            layer.draw_average(
                &self.candles,
                MovingAverageSpec::sma(
                    &self.daily_candles,
                    50,
                    MovingAverageColorRole::Fast,
                    "50d SMA",
                ),
                show_labels,
            );
        }
        if self.macro_indicators.ema_50d {
            layer.draw_average(
                &self.candles,
                MovingAverageSpec::ema(
                    &self.daily_candles,
                    50,
                    MovingAverageColorRole::Fast,
                    "50d EMA",
                ),
                show_labels,
            );
        }
        if self.macro_indicators.sma_200d {
            layer.draw_average(
                &self.candles,
                MovingAverageSpec::sma(
                    &self.daily_candles,
                    200,
                    MovingAverageColorRole::Slow,
                    "200d SMA",
                ),
                show_labels,
            );
        }
        if self.macro_indicators.ema_200d {
            layer.draw_average(
                &self.candles,
                MovingAverageSpec::ema(
                    &self.daily_candles,
                    200,
                    MovingAverageColorRole::Slow,
                    "200d EMA",
                ),
                show_labels,
            );
        }
        if self.macro_indicators.sma_20w {
            layer.draw_average(
                &self.candles,
                MovingAverageSpec::sma(
                    &self.weekly_candles,
                    20,
                    MovingAverageColorRole::WeeklyFast,
                    "20w SMA",
                ),
                show_labels,
            );
        }
        if self.macro_indicators.ema_20w {
            layer.draw_average(
                &self.candles,
                MovingAverageSpec::ema(
                    &self.weekly_candles,
                    20,
                    MovingAverageColorRole::WeeklyFast,
                    "20w EMA",
                ),
                show_labels,
            );
        }
        if self.macro_indicators.sma_50w {
            layer.draw_average(
                &self.candles,
                MovingAverageSpec::sma(
                    &self.weekly_candles,
                    50,
                    MovingAverageColorRole::WeeklySlow,
                    "50w SMA",
                ),
                show_labels,
            );
        }
        if self.macro_indicators.ema_50w {
            layer.draw_average(
                &self.candles,
                MovingAverageSpec::ema(
                    &self.weekly_candles,
                    50,
                    MovingAverageColorRole::WeeklySlow,
                    "50w EMA",
                ),
                show_labels,
            );
        }
        if self.macro_indicators.sma_12m {
            layer.draw_average(
                &self.candles,
                MovingAverageSpec::sma(
                    &self.monthly_candles,
                    12,
                    MovingAverageColorRole::Monthly,
                    "12M SMA",
                ),
                show_labels,
            );
        }
        if self.macro_indicators.ema_12m {
            layer.draw_average(
                &self.candles,
                MovingAverageSpec::ema(
                    &self.monthly_candles,
                    12,
                    MovingAverageColorRole::Monthly,
                    "12M EMA",
                ),
                show_labels,
            );
        }
    }
}
