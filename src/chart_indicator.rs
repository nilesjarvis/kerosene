use crate::chart_state::ChartInstance;
use crate::config::MacroIndicatorsConfig;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Chart Indicator Registry
// ---------------------------------------------------------------------------

/// Stable identifiers for visual indicators that can be changed through both
/// the chart UI and bounded workspace actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ChartIndicatorId {
    #[serde(rename = "tf_sma_50")]
    TfSma50,
    #[serde(rename = "tf_ema_50")]
    TfEma50,
    #[serde(rename = "tf_sma_200")]
    TfSma200,
    #[serde(rename = "tf_ema_200")]
    TfEma200,
    #[serde(rename = "sma_50h")]
    Sma50h,
    #[serde(rename = "ema_50h")]
    Ema50h,
    #[serde(rename = "sma_200h")]
    Sma200h,
    #[serde(rename = "ema_200h")]
    Ema200h,
    #[serde(rename = "sma_50d")]
    Sma50d,
    #[serde(rename = "ema_50d")]
    Ema50d,
    #[serde(rename = "sma_200d")]
    Sma200d,
    #[serde(rename = "ema_200d")]
    Ema200d,
    #[serde(rename = "sma_20w")]
    Sma20w,
    #[serde(rename = "ema_20w")]
    Ema20w,
    #[serde(rename = "sma_50w")]
    Sma50w,
    #[serde(rename = "ema_50w")]
    Ema50w,
    #[serde(rename = "sma_12m")]
    Sma12m,
    #[serde(rename = "ema_12m")]
    Ema12m,
    FundingRate,
    Sessions,
    Labels,
    QuickTrade,
    VolumeProfile,
    HighLow,
    LeledcArrows,
    LeledcLevels,
    TradeMarkers,
}

impl ChartIndicatorId {
    pub(crate) const MAX_MOVING_AVERAGE_PERIOD: usize = 5_000;

    pub(crate) const MOVING_AVERAGES: [Self; 18] = [
        Self::TfSma50,
        Self::TfEma50,
        Self::TfSma200,
        Self::TfEma200,
        Self::Sma50h,
        Self::Ema50h,
        Self::Sma200h,
        Self::Ema200h,
        Self::Sma50d,
        Self::Ema50d,
        Self::Sma200d,
        Self::Ema200d,
        Self::Sma20w,
        Self::Ema20w,
        Self::Sma50w,
        Self::Ema50w,
        Self::Sma12m,
        Self::Ema12m,
    ];

    pub(crate) const fn default_period(self) -> Option<usize> {
        match self {
            Self::TfSma50
            | Self::TfEma50
            | Self::Sma50h
            | Self::Ema50h
            | Self::Sma50d
            | Self::Ema50d
            | Self::Sma50w
            | Self::Ema50w => Some(50),
            Self::TfSma200
            | Self::TfEma200
            | Self::Sma200h
            | Self::Ema200h
            | Self::Sma200d
            | Self::Ema200d => Some(200),
            Self::Sma20w | Self::Ema20w => Some(20),
            Self::Sma12m | Self::Ema12m => Some(12),
            _ => None,
        }
    }

    /// Invalid persisted overrides fall back to the original slot's period.
    pub(crate) fn period(self, config: &MacroIndicatorsConfig) -> Option<usize> {
        let default = self.default_period()?;
        Some(
            config
                .moving_average_periods
                .get(self.key())
                .copied()
                .filter(|period| (1..=Self::MAX_MOVING_AVERAGE_PERIOD).contains(period))
                .unwrap_or(default),
        )
    }

    pub(crate) fn moving_average_label(self, config: &MacroIndicatorsConfig) -> Option<String> {
        let period = self.period(config)?;
        let kind = if self.key().contains("ema") {
            "EMA"
        } else {
            "SMA"
        };
        Some(match self.group() {
            "chart_timeframe" => format!("TF {period} {kind}"),
            "hourly" => format!("{period}h {kind}"),
            "daily" => format!("{period}d {kind}"),
            "weekly" => format!("{period}w {kind}"),
            "monthly" => format!("{period}M {kind}"),
            _ => return None,
        })
    }

    #[cfg(test)]
    pub(crate) const ALL: [Self; 27] = [
        Self::TfSma50,
        Self::TfEma50,
        Self::TfSma200,
        Self::TfEma200,
        Self::Sma50h,
        Self::Ema50h,
        Self::Sma200h,
        Self::Ema200h,
        Self::Sma50d,
        Self::Ema50d,
        Self::Sma200d,
        Self::Ema200d,
        Self::Sma20w,
        Self::Ema20w,
        Self::Sma50w,
        Self::Ema50w,
        Self::Sma12m,
        Self::Ema12m,
        Self::FundingRate,
        Self::Sessions,
        Self::Labels,
        Self::QuickTrade,
        Self::VolumeProfile,
        Self::HighLow,
        Self::LeledcArrows,
        Self::LeledcLevels,
        Self::TradeMarkers,
    ];

    /// Indicators safe for the Assistant's reversible chart-visual action.
    /// Presentation-only labels and executable Quick Trade controls stay out.
    pub(crate) const ASSISTANT_VISIBLE: [Self; 25] = [
        Self::TfSma50,
        Self::TfEma50,
        Self::TfSma200,
        Self::TfEma200,
        Self::Sma50h,
        Self::Ema50h,
        Self::Sma200h,
        Self::Ema200h,
        Self::Sma50d,
        Self::Ema50d,
        Self::Sma200d,
        Self::Ema200d,
        Self::Sma20w,
        Self::Ema20w,
        Self::Sma50w,
        Self::Ema50w,
        Self::Sma12m,
        Self::Ema12m,
        Self::FundingRate,
        Self::Sessions,
        Self::VolumeProfile,
        Self::HighLow,
        Self::LeledcArrows,
        Self::LeledcLevels,
        Self::TradeMarkers,
    ];

    pub(crate) const fn key(self) -> &'static str {
        match self {
            Self::TfSma50 => "tf_sma_50",
            Self::TfEma50 => "tf_ema_50",
            Self::TfSma200 => "tf_sma_200",
            Self::TfEma200 => "tf_ema_200",
            Self::Sma50h => "sma_50h",
            Self::Ema50h => "ema_50h",
            Self::Sma200h => "sma_200h",
            Self::Ema200h => "ema_200h",
            Self::Sma50d => "sma_50d",
            Self::Ema50d => "ema_50d",
            Self::Sma200d => "sma_200d",
            Self::Ema200d => "ema_200d",
            Self::Sma20w => "sma_20w",
            Self::Ema20w => "ema_20w",
            Self::Sma50w => "sma_50w",
            Self::Ema50w => "ema_50w",
            Self::Sma12m => "sma_12m",
            Self::Ema12m => "ema_12m",
            Self::FundingRate => "funding_rate",
            Self::Sessions => "sessions",
            Self::Labels => "labels",
            Self::QuickTrade => "quick_trade",
            Self::VolumeProfile => "volume_profile",
            Self::HighLow => "high_low",
            Self::LeledcArrows => "leledc_arrows",
            Self::LeledcLevels => "leledc_levels",
            Self::TradeMarkers => "trade_markers",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::TfSma50 => "50 SMA (chart timeframe)",
            Self::TfEma50 => "50 EMA (chart timeframe)",
            Self::TfSma200 => "200 SMA (chart timeframe)",
            Self::TfEma200 => "200 EMA (chart timeframe)",
            Self::Sma50h => "50-hour SMA",
            Self::Ema50h => "50-hour EMA",
            Self::Sma200h => "200-hour SMA",
            Self::Ema200h => "200-hour EMA",
            Self::Sma50d => "50-day SMA",
            Self::Ema50d => "50-day EMA",
            Self::Sma200d => "200-day SMA",
            Self::Ema200d => "200-day EMA",
            Self::Sma20w => "20-week SMA",
            Self::Ema20w => "20-week EMA",
            Self::Sma50w => "50-week SMA",
            Self::Ema50w => "50-week EMA",
            Self::Sma12m => "12-month SMA",
            Self::Ema12m => "12-month EMA",
            Self::FundingRate => "Funding rate",
            Self::Sessions => "Market sessions",
            Self::Labels => "Indicator labels",
            Self::QuickTrade => "Quick Trade controls",
            Self::VolumeProfile => "Volume profile",
            Self::HighLow => "High/low",
            Self::LeledcArrows => "Leledc arrows",
            Self::LeledcLevels => "Leledc levels",
            Self::TradeMarkers => "Trade markers",
        }
    }

    pub(crate) const fn group(self) -> &'static str {
        match self {
            Self::TfSma50 | Self::TfEma50 | Self::TfSma200 | Self::TfEma200 => "chart_timeframe",
            Self::Sma50h | Self::Ema50h | Self::Sma200h | Self::Ema200h => "hourly",
            Self::Sma50d | Self::Ema50d | Self::Sma200d | Self::Ema200d => "daily",
            Self::Sma20w | Self::Ema20w | Self::Sma50w | Self::Ema50w => "weekly",
            Self::Sma12m | Self::Ema12m => "monthly",
            Self::FundingRate => "funding",
            Self::Sessions => "sessions",
            Self::Labels => "presentation",
            Self::QuickTrade => "trading_controls",
            Self::VolumeProfile => "volume",
            Self::HighLow => "price",
            Self::LeledcArrows | Self::LeledcLevels => "leledc",
            Self::TradeMarkers => "activity",
        }
    }

    pub(crate) const fn aliases(self) -> &'static [&'static str] {
        match self {
            Self::TfSma50 => &["50 SMA", "current-timeframe 50 SMA"],
            Self::TfEma50 => &["50 EMA", "current-timeframe 50 EMA"],
            Self::TfSma200 => &["200 SMA", "current-timeframe 200 SMA"],
            Self::TfEma200 => &["200 EMA", "current-timeframe 200 EMA"],
            Self::Sma50h => &["50h SMA", "hourly 50 SMA"],
            Self::Ema50h => &["50h EMA", "hourly 50 EMA"],
            Self::Sma200h => &["200h SMA", "hourly 200 SMA"],
            Self::Ema200h => &["200h EMA", "hourly 200 EMA"],
            Self::Sma50d => &["50d SMA", "daily 50 SMA"],
            Self::Ema50d => &["50d EMA", "daily 50 EMA"],
            Self::Sma200d => &["200d SMA", "daily 200 SMA"],
            Self::Ema200d => &["200d EMA", "daily 200 EMA"],
            Self::Sma20w => &["20w SMA", "weekly 20 SMA"],
            Self::Ema20w => &["20w EMA", "weekly 20 EMA"],
            Self::Sma50w => &["50w SMA", "weekly 50 SMA"],
            Self::Ema50w => &["50w EMA", "weekly 50 EMA"],
            Self::Sma12m => &["12m SMA", "monthly 12 SMA"],
            Self::Ema12m => &["12m EMA", "monthly 12 EMA"],
            Self::FundingRate => &["funding", "funding panel"],
            Self::Sessions => &["sessions", "market sessions"],
            Self::Labels => &["labels", "indicator labels"],
            Self::QuickTrade => &["quick trade", "chart trading controls"],
            Self::VolumeProfile => &["volume profile", "vol profile"],
            Self::HighLow => &["high low", "high/low"],
            Self::LeledcArrows => &["Leledc arrows", "exhaustion arrows"],
            Self::LeledcLevels => &["Leledc levels", "exhaustion levels"],
            Self::TradeMarkers => &["trades", "fills", "trade dots"],
        }
    }

    pub(crate) const fn requires_hydromancer(self) -> bool {
        matches!(self, Self::FundingRate)
    }

    pub(crate) const fn is_macro(self) -> bool {
        !matches!(self, Self::TradeMarkers)
    }

    pub(crate) fn is_enabled(self, instance: &ChartInstance) -> bool {
        if self == Self::TradeMarkers {
            instance.chart.show_trade_markers
        } else {
            self.is_enabled_in_config(&instance.macro_indicators)
        }
    }

    pub(crate) fn is_enabled_in_config(self, indicators: &MacroIndicatorsConfig) -> bool {
        match self {
            Self::TfSma50 => indicators.tf_sma_50,
            Self::TfEma50 => indicators.tf_ema_50,
            Self::TfSma200 => indicators.tf_sma_200,
            Self::TfEma200 => indicators.tf_ema_200,
            Self::Sma50h => indicators.sma_50h,
            Self::Ema50h => indicators.ema_50h,
            Self::Sma200h => indicators.sma_200h,
            Self::Ema200h => indicators.ema_200h,
            Self::Sma50d => indicators.sma_50d,
            Self::Ema50d => indicators.ema_50d,
            Self::Sma200d => indicators.sma_200d,
            Self::Ema200d => indicators.ema_200d,
            Self::Sma20w => indicators.sma_20w,
            Self::Ema20w => indicators.ema_20w,
            Self::Sma50w => indicators.sma_50w,
            Self::Ema50w => indicators.ema_50w,
            Self::Sma12m => indicators.sma_12m,
            Self::Ema12m => indicators.ema_12m,
            Self::FundingRate => indicators.show_funding_rate,
            Self::Sessions => indicators.show_session_indicator,
            Self::Labels => indicators.show_labels,
            Self::QuickTrade => indicators.show_quick_trade,
            Self::VolumeProfile => indicators.show_volume_profile,
            Self::HighLow => indicators.show_high_low,
            Self::LeledcArrows => indicators.show_leledc_arrows,
            Self::LeledcLevels => indicators.show_leledc_levels,
            Self::TradeMarkers => false,
        }
    }

    /// Sets the requested state and returns whether anything changed.
    pub(crate) fn set_enabled(self, instance: &mut ChartInstance, enabled: bool) -> bool {
        if self.is_enabled(instance) == enabled {
            return false;
        }

        let indicators = &mut instance.macro_indicators;
        match self {
            Self::TfSma50 => indicators.tf_sma_50 = enabled,
            Self::TfEma50 => indicators.tf_ema_50 = enabled,
            Self::TfSma200 => indicators.tf_sma_200 = enabled,
            Self::TfEma200 => indicators.tf_ema_200 = enabled,
            Self::Sma50h => indicators.sma_50h = enabled,
            Self::Ema50h => indicators.ema_50h = enabled,
            Self::Sma200h => indicators.sma_200h = enabled,
            Self::Ema200h => indicators.ema_200h = enabled,
            Self::Sma50d => indicators.sma_50d = enabled,
            Self::Ema50d => indicators.ema_50d = enabled,
            Self::Sma200d => indicators.sma_200d = enabled,
            Self::Ema200d => indicators.ema_200d = enabled,
            Self::Sma20w => indicators.sma_20w = enabled,
            Self::Ema20w => indicators.ema_20w = enabled,
            Self::Sma50w => indicators.sma_50w = enabled,
            Self::Ema50w => indicators.ema_50w = enabled,
            Self::Sma12m => indicators.sma_12m = enabled,
            Self::Ema12m => indicators.ema_12m = enabled,
            Self::FundingRate => indicators.show_funding_rate = enabled,
            Self::Sessions => indicators.show_session_indicator = enabled,
            Self::Labels => indicators.show_labels = enabled,
            Self::QuickTrade => indicators.show_quick_trade = enabled,
            Self::VolumeProfile => indicators.show_volume_profile = enabled,
            Self::HighLow => indicators.show_high_low = enabled,
            Self::LeledcArrows => indicators.show_leledc_arrows = enabled,
            Self::LeledcLevels => indicators.show_leledc_levels = enabled,
            Self::TradeMarkers => instance.chart.show_trade_markers = enabled,
        }
        true
    }
}

/// Retain the normal visible history plus enough warm-up candles for custom periods.
pub(crate) fn moving_average_lookback_ms(
    config: &MacroIndicatorsConfig,
    timeframe: crate::timeframe::Timeframe,
    group: &str,
) -> u64 {
    let period = ChartIndicatorId::MOVING_AVERAGES
        .into_iter()
        .filter(|key| {
            key.group() == group
                && key.is_enabled_in_config(config)
                && config.moving_average_periods.contains_key(key.key())
        })
        .filter_map(|key| key.period(config))
        .max()
        .unwrap_or(0) as u64;
    if period == 0 {
        return timeframe.lookback_ms();
    }
    let duration = timeframe.duration_ms();
    let visible_candles = timeframe.lookback_ms() / duration;
    timeframe.lookback_ms().max(
        duration.saturating_mul(
            (period.saturating_mul(3) + visible_candles)
                .min(ChartIndicatorId::MAX_MOVING_AVERAGE_PERIOD as u64),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

    #[test]
    fn every_indicator_has_a_unique_stable_key() {
        let keys = ChartIndicatorId::ALL
            .iter()
            .map(|indicator| indicator.key())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(keys.len(), ChartIndicatorId::ALL.len());
    }

    #[test]
    fn serde_wire_ids_match_registry_keys() {
        for indicator in ChartIndicatorId::ALL {
            let serialized = serde_json::to_value(indicator).expect("serialize indicator id");
            assert_eq!(serialized, indicator.key());
            let decoded: ChartIndicatorId =
                serde_json::from_value(serialized).expect("deserialize indicator id");
            assert_eq!(decoded, indicator);
        }
    }

    #[test]
    fn setting_an_indicator_is_idempotent() {
        let mut instance = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
        assert!(ChartIndicatorId::TfEma50.set_enabled(&mut instance, true));
        assert!(!ChartIndicatorId::TfEma50.set_enabled(&mut instance, true));
        assert!(ChartIndicatorId::TfEma50.is_enabled(&instance));
    }
    #[test]
    fn moving_average_defaults_and_overrides_are_independent() {
        let mut config = MacroIndicatorsConfig::default();
        for key in ChartIndicatorId::ALL {
            assert_eq!(key.period(&config), key.default_period());
        }
        config.moving_average_periods.insert("tf_ema_50".into(), 21);
        assert_eq!(ChartIndicatorId::TfEma50.period(&config), Some(21));
        assert_eq!(ChartIndicatorId::TfSma50.period(&config), Some(50));
        assert_eq!(
            ChartIndicatorId::TfEma50
                .moving_average_label(&config)
                .as_deref(),
            Some("TF 21 EMA")
        );
        config.moving_average_periods.insert("sma_200d".into(), 55);
        assert_eq!(
            ChartIndicatorId::Sma200d
                .moving_average_label(&config)
                .as_deref(),
            Some("55d SMA")
        );
        for invalid in [0, 5_001, usize::MAX] {
            config
                .moving_average_periods
                .insert("tf_ema_50".into(), invalid);
            assert_eq!(ChartIndicatorId::TfEma50.period(&config), Some(50));
        }
    }

    #[test]
    fn moving_average_history_is_bounded_and_only_expands_enabled_sources() {
        let tf = Timeframe::H1;
        let mut config = MacroIndicatorsConfig::default();
        config
            .moving_average_periods
            .insert("tf_ema_50".into(), 1_000);
        assert_eq!(
            moving_average_lookback_ms(&config, tf, "chart_timeframe"),
            tf.lookback_ms()
        );
        config.tf_ema_50 = true;
        assert_eq!(
            moving_average_lookback_ms(&config, tf, "chart_timeframe"),
            tf.lookback_ms() + 3_000 * tf.duration_ms()
        );
        assert_eq!(
            moving_average_lookback_ms(&config, tf, "hourly"),
            tf.lookback_ms()
        );
        config
            .moving_average_periods
            .insert("tf_ema_50".into(), 5_000);
        assert_eq!(
            moving_average_lookback_ms(&config, tf, "chart_timeframe"),
            5_000 * tf.duration_ms()
        );
    }
}
