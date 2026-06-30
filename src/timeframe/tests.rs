use super::metadata::{ALL_TIMEFRAMES, API_STRS, CONFIG_STRS, DURATIONS_MS, LABELS, LOOKBACKS_MS};
use super::{
    HYDROMANCER_TIMEFRAME_OPTIONS, TIMEFRAME_HOTKEY_OPTIONS, TIMEFRAME_OPTIONS, Timeframe,
    chart_timeframe_options,
};

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
        assert!(timeframe.cache_display_max_age_ms() <= timeframe.lookback_ms());
        assert!(timeframe.cache_display_max_age_ms() >= timeframe.duration_ms());
    }
}

#[test]
fn cache_display_freshness_is_shorter_than_historical_lookback() {
    assert_eq!(Timeframe::M1.cache_display_max_age_ms(), 5 * 60 * 1000);
    assert_eq!(
        Timeframe::H1.cache_display_max_age_ms(),
        3 * Timeframe::H1.duration_ms()
    );
    assert!(Timeframe::D1.cache_display_max_age_ms() < Timeframe::D1.lookback_ms());
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
    assert!(TIMEFRAME_OPTIONS.contains(&Timeframe::Tick));
    assert!(!TIMEFRAME_OPTIONS.contains(&Timeframe::S1));
    assert_eq!(chart_timeframe_options(false), TIMEFRAME_OPTIONS);
}

#[test]
fn hydromancer_toolbar_timeframe_options_include_one_second() {
    assert!(HYDROMANCER_TIMEFRAME_OPTIONS.contains(&Timeframe::Tick));
    assert!(HYDROMANCER_TIMEFRAME_OPTIONS.contains(&Timeframe::S1));
    assert_eq!(chart_timeframe_options(true), HYDROMANCER_TIMEFRAME_OPTIONS);
}

#[test]
fn tick_timeframe_is_realtime_only() {
    assert!(Timeframe::Tick.uses_orderbook_tick_candles());
    assert!(!Timeframe::Tick.uses_candle_backfill());
    assert!(!Timeframe::M1.uses_orderbook_tick_candles());
    assert!(Timeframe::M1.uses_candle_backfill());
}

#[test]
fn timeframe_hotkey_options_preserve_existing_number_row_mapping() {
    assert_eq!(TIMEFRAME_HOTKEY_OPTIONS.first(), Some(&Timeframe::M1));
    assert_eq!(TIMEFRAME_HOTKEY_OPTIONS.last(), Some(&Timeframe::W1));
    assert!(!TIMEFRAME_HOTKEY_OPTIONS.contains(&Timeframe::Tick));
    assert!(!TIMEFRAME_HOTKEY_OPTIONS.contains(&Timeframe::S1));
}
