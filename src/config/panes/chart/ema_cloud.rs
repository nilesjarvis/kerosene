use serde::{Deserialize, Deserializer, Serialize};

use crate::chart_indicator::ChartIndicatorId;
use crate::timeframe::Timeframe;

pub const MAX_EMA_CLOUDS: usize = 8;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EmaCloudTimeframe {
    #[default]
    Chart,
    Hour,
    Day,
    Week,
    Month,
}

impl EmaCloudTimeframe {
    pub const ALL: [Self; 5] = [Self::Chart, Self::Hour, Self::Day, Self::Week, Self::Month];

    pub fn source(self) -> Option<Timeframe> {
        match self {
            Self::Chart => None,
            Self::Hour => Some(Timeframe::H1),
            Self::Day => Some(Timeframe::D1),
            Self::Week => Some(Timeframe::W1),
            Self::Month => Some(Timeframe::Mo1),
        }
    }

    pub fn group(self) -> &'static str {
        match self {
            Self::Chart => "chart_timeframe",
            Self::Hour => "hourly",
            Self::Day => "daily",
            Self::Week => "weekly",
            Self::Month => "monthly",
        }
    }
}

impl std::fmt::Display for EmaCloudTimeframe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Chart => "Chart TF",
            Self::Hour => "1 hour",
            Self::Day => "1 day",
            Self::Week => "1 week",
            Self::Month => "1 month",
        })
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EmaCloudColor {
    #[default]
    Trend,
    Primary,
    Warning,
    Secondary,
}

impl EmaCloudColor {
    pub const ALL: [Self; 4] = [Self::Trend, Self::Primary, Self::Warning, Self::Secondary];

    pub fn color(self, theme: &iced::Theme, bullish: bool) -> iced::Color {
        let palette = theme.extended_palette();
        match self {
            Self::Trend if bullish => palette.success.base.color,
            Self::Trend => palette.danger.base.color,
            Self::Primary => palette.primary.base.color,
            Self::Warning => palette.warning.base.color,
            Self::Secondary => palette.secondary.strong.color,
        }
    }
}

impl std::fmt::Display for EmaCloudColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Trend => "Bull / Bear",
            Self::Primary => "Primary",
            Self::Warning => "Warning",
            Self::Secondary => "Secondary",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmaCloudPeriod {
    Fast,
    Slow,
}

/// A filled band between two close-price EMAs, independent of the line slots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct EmaCloudConfig {
    pub id: u64,
    pub enabled: bool,
    pub fast_period: usize,
    pub slow_period: usize,
    pub timeframe: EmaCloudTimeframe,
    pub color: EmaCloudColor,
    pub opacity: u8,
}

impl Default for EmaCloudConfig {
    fn default() -> Self {
        Self {
            id: 0,
            enabled: true,
            fast_period: 20,
            slow_period: 50,
            timeframe: EmaCloudTimeframe::Chart,
            color: EmaCloudColor::Trend,
            opacity: 20,
        }
    }
}

impl EmaCloudConfig {
    pub fn is_valid(&self) -> bool {
        [self.fast_period, self.slow_period]
            .into_iter()
            .all(|period| (1..=ChartIndicatorId::MAX_MOVING_AVERAGE_PERIOD).contains(&period))
            && self.opacity <= 100
    }

    pub fn label(&self) -> String {
        format!(
            "{} EMA cloud {}/{}",
            self.timeframe, self.fast_period, self.slow_period
        )
    }
}

pub(super) fn deserialize_ema_clouds<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<EmaCloudConfig>, D::Error> {
    let clouds = Vec::<EmaCloudConfig>::deserialize(deserializer)?;
    let mut ids = std::collections::HashSet::new();
    Ok(clouds
        .into_iter()
        .filter(|cloud| cloud.is_valid() && ids.insert(cloud.id))
        .take(MAX_EMA_CLOUDS)
        .collect())
}
