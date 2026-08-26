use super::IndicatorOption;
use crate::chart_indicator::ChartIndicatorId;
use crate::config::MacroIndicatorsConfig;

// ---------------------------------------------------------------------------
// Indicator Menu Options
// ---------------------------------------------------------------------------

pub(super) fn timeframe_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 4] {
    [
        IndicatorOption {
            label: "50 SMA",
            key: ChartIndicatorId::TfSma50,
            checked: indicators.tf_sma_50,
        },
        IndicatorOption {
            label: "50 EMA",
            key: ChartIndicatorId::TfEma50,
            checked: indicators.tf_ema_50,
        },
        IndicatorOption {
            label: "200 SMA",
            key: ChartIndicatorId::TfSma200,
            checked: indicators.tf_sma_200,
        },
        IndicatorOption {
            label: "200 EMA",
            key: ChartIndicatorId::TfEma200,
            checked: indicators.tf_ema_200,
        },
    ]
}

pub(super) fn hourly_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 4] {
    [
        IndicatorOption {
            label: "50 SMA",
            key: ChartIndicatorId::Sma50h,
            checked: indicators.sma_50h,
        },
        IndicatorOption {
            label: "50 EMA",
            key: ChartIndicatorId::Ema50h,
            checked: indicators.ema_50h,
        },
        IndicatorOption {
            label: "200 SMA",
            key: ChartIndicatorId::Sma200h,
            checked: indicators.sma_200h,
        },
        IndicatorOption {
            label: "200 EMA",
            key: ChartIndicatorId::Ema200h,
            checked: indicators.ema_200h,
        },
    ]
}

pub(super) fn daily_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 4] {
    [
        IndicatorOption {
            label: "50 SMA",
            key: ChartIndicatorId::Sma50d,
            checked: indicators.sma_50d,
        },
        IndicatorOption {
            label: "50 EMA",
            key: ChartIndicatorId::Ema50d,
            checked: indicators.ema_50d,
        },
        IndicatorOption {
            label: "200 SMA",
            key: ChartIndicatorId::Sma200d,
            checked: indicators.sma_200d,
        },
        IndicatorOption {
            label: "200 EMA",
            key: ChartIndicatorId::Ema200d,
            checked: indicators.ema_200d,
        },
    ]
}

pub(super) fn weekly_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 4] {
    [
        IndicatorOption {
            label: "20 SMA",
            key: ChartIndicatorId::Sma20w,
            checked: indicators.sma_20w,
        },
        IndicatorOption {
            label: "20 EMA",
            key: ChartIndicatorId::Ema20w,
            checked: indicators.ema_20w,
        },
        IndicatorOption {
            label: "50 SMA",
            key: ChartIndicatorId::Sma50w,
            checked: indicators.sma_50w,
        },
        IndicatorOption {
            label: "50 EMA",
            key: ChartIndicatorId::Ema50w,
            checked: indicators.ema_50w,
        },
    ]
}

pub(super) fn monthly_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 2] {
    [
        IndicatorOption {
            label: "12 SMA",
            key: ChartIndicatorId::Sma12m,
            checked: indicators.sma_12m,
        },
        IndicatorOption {
            label: "12 EMA",
            key: ChartIndicatorId::Ema12m,
            checked: indicators.ema_12m,
        },
    ]
}

pub(super) fn footer_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 3] {
    [
        IndicatorOption {
            label: "Funding",
            key: ChartIndicatorId::FundingRate,
            checked: indicators.show_funding_rate,
        },
        IndicatorOption {
            label: "Sessions",
            key: ChartIndicatorId::Sessions,
            checked: indicators.show_session_indicator,
        },
        IndicatorOption {
            label: "Labels",
            key: ChartIndicatorId::Labels,
            checked: indicators.show_labels,
        },
    ]
}

pub(super) fn price_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 1] {
    [IndicatorOption {
        label: "High/Low",
        key: ChartIndicatorId::HighLow,
        checked: indicators.show_high_low,
    }]
}

pub(super) fn quick_trade_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 1] {
    [IndicatorOption {
        label: "Quick Trade",
        key: ChartIndicatorId::QuickTrade,
        checked: indicators.show_quick_trade,
    }]
}

pub(super) fn volume_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 1] {
    [IndicatorOption {
        label: "Profile",
        key: ChartIndicatorId::VolumeProfile,
        checked: indicators.show_volume_profile,
    }]
}

pub(super) fn leledc_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 2] {
    [
        IndicatorOption {
            label: "Arrows",
            key: ChartIndicatorId::LeledcArrows,
            checked: indicators.show_leledc_arrows,
        },
        IndicatorOption {
            label: "Levels",
            key: ChartIndicatorId::LeledcLevels,
            checked: indicators.show_leledc_levels,
        },
    ]
}

#[cfg(test)]
pub(super) fn all_indicator_options(indicators: &MacroIndicatorsConfig) -> Vec<IndicatorOption> {
    let mut options = Vec::new();
    options.extend(timeframe_options(indicators));
    options.extend(hourly_options(indicators));
    options.extend(daily_options(indicators));
    options.extend(weekly_options(indicators));
    options.extend(monthly_options(indicators));
    options.extend(footer_options(indicators));
    options.extend(quick_trade_options(indicators));
    options.extend(price_options(indicators));
    options.extend(volume_options(indicators));
    options.extend(leledc_options(indicators));
    options
}
