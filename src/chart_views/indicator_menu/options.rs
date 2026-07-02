use super::IndicatorOption;
use crate::config::MacroIndicatorsConfig;

// ---------------------------------------------------------------------------
// Indicator Menu Options
// ---------------------------------------------------------------------------

pub(super) fn timeframe_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 4] {
    [
        IndicatorOption {
            label: "50 SMA",
            key: "tf_sma_50",
            checked: indicators.tf_sma_50,
        },
        IndicatorOption {
            label: "50 EMA",
            key: "tf_ema_50",
            checked: indicators.tf_ema_50,
        },
        IndicatorOption {
            label: "200 SMA",
            key: "tf_sma_200",
            checked: indicators.tf_sma_200,
        },
        IndicatorOption {
            label: "200 EMA",
            key: "tf_ema_200",
            checked: indicators.tf_ema_200,
        },
    ]
}

pub(super) fn hourly_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 4] {
    [
        IndicatorOption {
            label: "50 SMA",
            key: "sma_50h",
            checked: indicators.sma_50h,
        },
        IndicatorOption {
            label: "50 EMA",
            key: "ema_50h",
            checked: indicators.ema_50h,
        },
        IndicatorOption {
            label: "200 SMA",
            key: "sma_200h",
            checked: indicators.sma_200h,
        },
        IndicatorOption {
            label: "200 EMA",
            key: "ema_200h",
            checked: indicators.ema_200h,
        },
    ]
}

pub(super) fn daily_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 4] {
    [
        IndicatorOption {
            label: "50 SMA",
            key: "sma_50d",
            checked: indicators.sma_50d,
        },
        IndicatorOption {
            label: "50 EMA",
            key: "ema_50d",
            checked: indicators.ema_50d,
        },
        IndicatorOption {
            label: "200 SMA",
            key: "sma_200d",
            checked: indicators.sma_200d,
        },
        IndicatorOption {
            label: "200 EMA",
            key: "ema_200d",
            checked: indicators.ema_200d,
        },
    ]
}

pub(super) fn weekly_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 4] {
    [
        IndicatorOption {
            label: "20 SMA",
            key: "sma_20w",
            checked: indicators.sma_20w,
        },
        IndicatorOption {
            label: "20 EMA",
            key: "ema_20w",
            checked: indicators.ema_20w,
        },
        IndicatorOption {
            label: "50 SMA",
            key: "sma_50w",
            checked: indicators.sma_50w,
        },
        IndicatorOption {
            label: "50 EMA",
            key: "ema_50w",
            checked: indicators.ema_50w,
        },
    ]
}

pub(super) fn monthly_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 2] {
    [
        IndicatorOption {
            label: "12 SMA",
            key: "sma_12m",
            checked: indicators.sma_12m,
        },
        IndicatorOption {
            label: "12 EMA",
            key: "ema_12m",
            checked: indicators.ema_12m,
        },
    ]
}

pub(super) fn footer_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 3] {
    [
        IndicatorOption {
            label: "Funding",
            key: "show_funding_rate",
            checked: indicators.show_funding_rate,
        },
        IndicatorOption {
            label: "Sessions",
            key: "show_session_indicator",
            checked: indicators.show_session_indicator,
        },
        IndicatorOption {
            label: "Labels",
            key: "show_labels",
            checked: indicators.show_labels,
        },
    ]
}

pub(super) fn price_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 1] {
    [IndicatorOption {
        label: "High/Low",
        key: "show_high_low",
        checked: indicators.show_high_low,
    }]
}

pub(super) fn volume_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 1] {
    [IndicatorOption {
        label: "Profile",
        key: "show_volume_profile",
        checked: indicators.show_volume_profile,
    }]
}

pub(super) fn leledc_options(indicators: &MacroIndicatorsConfig) -> [IndicatorOption; 2] {
    [
        IndicatorOption {
            label: "Arrows",
            key: "show_leledc_arrows",
            checked: indicators.show_leledc_arrows,
        },
        IndicatorOption {
            label: "Levels",
            key: "show_leledc_levels",
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
    options.extend(price_options(indicators));
    options.extend(volume_options(indicators));
    options.extend(leledc_options(indicators));
    options
}
