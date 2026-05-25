use super::*;

#[test]
fn chart_timeframe_prefix_round_trips_modifiers() {
    let prefix = HotkeyPrefixConfig {
        shift: false,
        ctrl: false,
        alt: false,
        logo: true,
    };

    let loaded = round_trip_prefix_or_panic(&prefix);

    assert_eq!(loaded, prefix);
}
