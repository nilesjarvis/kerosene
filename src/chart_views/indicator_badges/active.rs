use crate::chart_indicator::ChartIndicatorId;
use crate::chart_state::ChartInstance;

use iced::{Color, Theme};

// ---------------------------------------------------------------------------
// Active Indicator Registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(in crate::chart_views::indicator_badges) struct ActiveIndicator {
    pub(in crate::chart_views::indicator_badges) label: &'static str,
    pub(in crate::chart_views::indicator_badges) key: ChartIndicatorId,
    pub(in crate::chart_views::indicator_badges) color: Color,
}

#[derive(Debug, Clone, Copy)]
enum IndicatorColorRole {
    Fast,
    Slow,
    WeeklyFast,
    WeeklySlow,
    Monthly,
    Funding,
    Sessions,
    QuickTrade,
    VolumeProfile,
    HighLow,
    LeledcArrows,
    LeledcLevels,
}

impl IndicatorColorRole {
    fn color(self, theme: &Theme) -> Color {
        let extended = theme.extended_palette();

        match self {
            Self::Fast => extended.warning.base.color,
            Self::Slow => extended.primary.base.color,
            Self::WeeklyFast => extended.success.base.color,
            Self::WeeklySlow | Self::Funding => extended.secondary.strong.color,
            Self::Monthly | Self::LeledcArrows => extended.danger.base.color,
            Self::Sessions => extended.warning.base.color,
            Self::QuickTrade => extended.success.base.color,
            Self::VolumeProfile => theme.palette().primary,
            Self::HighLow => extended.background.weak.text,
            Self::LeledcLevels => extended.success.base.color,
        }
    }
}

pub(in crate::chart_views::indicator_badges) fn active_chart_indicators(
    instance: &ChartInstance,
    theme: &Theme,
) -> Vec<ActiveIndicator> {
    let indicators = &instance.macro_indicators;
    let mut active = Vec::new();

    push_indicator(
        &mut active,
        indicators.tf_sma_50,
        "TF 50 SMA",
        ChartIndicatorId::TfSma50,
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.tf_ema_50,
        "TF 50 EMA",
        ChartIndicatorId::TfEma50,
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.tf_sma_200,
        "TF 200 SMA",
        ChartIndicatorId::TfSma200,
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.tf_ema_200,
        "TF 200 EMA",
        ChartIndicatorId::TfEma200,
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_50h,
        "50h SMA",
        ChartIndicatorId::Sma50h,
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_50h,
        "50h EMA",
        ChartIndicatorId::Ema50h,
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_200h,
        "200h SMA",
        ChartIndicatorId::Sma200h,
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_200h,
        "200h EMA",
        ChartIndicatorId::Ema200h,
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_50d,
        "50d SMA",
        ChartIndicatorId::Sma50d,
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_50d,
        "50d EMA",
        ChartIndicatorId::Ema50d,
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_200d,
        "200d SMA",
        ChartIndicatorId::Sma200d,
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_200d,
        "200d EMA",
        ChartIndicatorId::Ema200d,
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_20w,
        "20w SMA",
        ChartIndicatorId::Sma20w,
        IndicatorColorRole::WeeklyFast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_20w,
        "20w EMA",
        ChartIndicatorId::Ema20w,
        IndicatorColorRole::WeeklyFast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_50w,
        "50w SMA",
        ChartIndicatorId::Sma50w,
        IndicatorColorRole::WeeklySlow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_50w,
        "50w EMA",
        ChartIndicatorId::Ema50w,
        IndicatorColorRole::WeeklySlow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_12m,
        "12M SMA",
        ChartIndicatorId::Sma12m,
        IndicatorColorRole::Monthly,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_12m,
        "12M EMA",
        ChartIndicatorId::Ema12m,
        IndicatorColorRole::Monthly,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_funding_rate,
        "Funding",
        ChartIndicatorId::FundingRate,
        IndicatorColorRole::Funding,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_session_indicator,
        "Sessions",
        ChartIndicatorId::Sessions,
        IndicatorColorRole::Sessions,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_quick_trade,
        "Quick Trade",
        ChartIndicatorId::QuickTrade,
        IndicatorColorRole::QuickTrade,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_volume_profile,
        "Vol Profile",
        ChartIndicatorId::VolumeProfile,
        IndicatorColorRole::VolumeProfile,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_high_low,
        "High/Low",
        ChartIndicatorId::HighLow,
        IndicatorColorRole::HighLow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_leledc_arrows,
        "Leledc Arrows",
        ChartIndicatorId::LeledcArrows,
        IndicatorColorRole::LeledcArrows,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_leledc_levels,
        "Leledc Levels",
        ChartIndicatorId::LeledcLevels,
        IndicatorColorRole::LeledcLevels,
        theme,
    );

    active
}

fn push_indicator(
    active: &mut Vec<ActiveIndicator>,
    enabled: bool,
    label: &'static str,
    key: ChartIndicatorId,
    color_role: IndicatorColorRole,
    theme: &Theme,
) {
    if enabled {
        active.push(ActiveIndicator {
            label,
            key,
            color: color_role.color(theme),
        });
    }
}
