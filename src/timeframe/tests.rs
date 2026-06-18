use super::metadata::{ALL_TIMEFRAMES, API_STRS, CONFIG_STRS, DURATIONS_MS, LABELS, LOOKBACKS_MS};
use super::{HYDROMANCER_TIMEFRAME_OPTIONS, TIMEFRAME_OPTIONS, Timeframe, chart_timeframe_options};

#[test]
fn timeframe_arrays_round_trip_config_strings() {
    for (idx, timeframe) in ALL_TIMEFRAMES.iter().copied().enumerate() {
        assert_eq!(timeframe.index(), idx);
        assert_eq!(
            Timeframe::from_config_str_opt(CONFIG_STRS[idx]),
            Some(timeframe)
        );
        assert_eq!(timeframe.config_str(), CONFIG_STRS[idx]);
        assert_eq!(timeframe.api_str(), API_STRS[idx]);
        assert_eq!(timeframe.label(), LABELS[idx]);
        assert_eq!(timeframe.duration_ms(), DURATIONS_MS[idx]);
        assert_eq!(timeframe.lookback_ms(), LOOKBACKS_MS[idx]);
        assert!(timeframe.lookback_ms() >= timeframe.duration_ms());
    }
}

#[test]
fn invalid_config_timeframe_defaults_to_h1() {
    assert_eq!(Timeframe::from_config_str("missing"), Timeframe::H1);
}

#[test]
fn toolbar_timeframe_options_are_supported_timeframes() {
    for option in TIMEFRAME_OPTIONS {
        assert!(ALL_TIMEFRAMES.contains(option));
    }
}

#[test]
fn default_toolbar_timeframe_options_hide_one_second() {
    assert!(!TIMEFRAME_OPTIONS.contains(&Timeframe::S1));
    assert_eq!(chart_timeframe_options(false), TIMEFRAME_OPTIONS);
}

#[test]
fn hydromancer_toolbar_timeframe_options_include_one_second() {
    assert!(HYDROMANCER_TIMEFRAME_OPTIONS.contains(&Timeframe::S1));
    assert_eq!(chart_timeframe_options(true), HYDROMANCER_TIMEFRAME_OPTIONS);
}
