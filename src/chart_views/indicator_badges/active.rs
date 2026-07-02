use crate::chart_state::ChartInstance;

use iced::{Color, Theme};

// ---------------------------------------------------------------------------
// Active Indicator Registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(in crate::chart_views::indicator_badges) struct ActiveIndicator {
    pub(in crate::chart_views::indicator_badges) label: &'static str,
    pub(in crate::chart_views::indicator_badges) key: &'static str,
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
        "tf_sma_50",
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.tf_ema_50,
        "TF 50 EMA",
        "tf_ema_50",
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.tf_sma_200,
        "TF 200 SMA",
        "tf_sma_200",
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.tf_ema_200,
        "TF 200 EMA",
        "tf_ema_200",
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_50h,
        "50h SMA",
        "sma_50h",
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_50h,
        "50h EMA",
        "ema_50h",
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_200h,
        "200h SMA",
        "sma_200h",
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_200h,
        "200h EMA",
        "ema_200h",
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_50d,
        "50d SMA",
        "sma_50d",
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_50d,
        "50d EMA",
        "ema_50d",
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_200d,
        "200d SMA",
        "sma_200d",
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_200d,
        "200d EMA",
        "ema_200d",
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_20w,
        "20w SMA",
        "sma_20w",
        IndicatorColorRole::WeeklyFast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_20w,
        "20w EMA",
        "ema_20w",
        IndicatorColorRole::WeeklyFast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_50w,
        "50w SMA",
        "sma_50w",
        IndicatorColorRole::WeeklySlow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_50w,
        "50w EMA",
        "ema_50w",
        IndicatorColorRole::WeeklySlow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_12m,
        "12M SMA",
        "sma_12m",
        IndicatorColorRole::Monthly,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_12m,
        "12M EMA",
        "ema_12m",
        IndicatorColorRole::Monthly,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_funding_rate,
        "Funding",
        "show_funding_rate",
        IndicatorColorRole::Funding,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_session_indicator,
        "Sessions",
        "show_session_indicator",
        IndicatorColorRole::Sessions,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_volume_profile,
        "Vol Profile",
        "show_volume_profile",
        IndicatorColorRole::VolumeProfile,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_high_low,
        "High/Low",
        "show_high_low",
        IndicatorColorRole::HighLow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_leledc_arrows,
        "Leledc Arrows",
        "show_leledc_arrows",
        IndicatorColorRole::LeledcArrows,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_leledc_levels,
        "Leledc Levels",
        "show_leledc_levels",
        IndicatorColorRole::LeledcLevels,
        theme,
    );

    active
}

fn push_indicator(
    active: &mut Vec<ActiveIndicator>,
    enabled: bool,
    label: &'static str,
    key: &'static str,
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
