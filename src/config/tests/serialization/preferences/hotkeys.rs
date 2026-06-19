use super::super::super::config_warning_guard;
use super::{
    HotkeyPrefixConfig, KeroseneConfig, default_config_value, json_string, object_mut,
    remove_field, value_from_json, value_from_str,
};
use crate::config::{HotkeyAction, take_config_warnings};

#[test]
fn chart_timeframe_hotkey_prefix_round_trips_and_legacy_defaults_none() {
    let config = KeroseneConfig {
        chart_timeframe_hotkey_prefix: Some(HotkeyPrefixConfig {
            shift: false,
            ctrl: false,
            alt: false,
            logo: true,
        }),
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(
        decoded.chart_timeframe_hotkey_prefix,
        config.chart_timeframe_hotkey_prefix
    );

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "chart_timeframe_hotkey_prefix",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert_eq!(decoded_legacy.chart_timeframe_hotkey_prefix, None);
}

#[test]
fn invalid_hotkey_entries_are_dropped_without_echoing_payloads() {
    let _warning_guard = config_warning_guard();
    let mut config = default_config_value();
    object_mut(&mut config, "config should serialize to object").insert(
        "hotkeys".to_string(),
        serde_json::json!([
            {
                "action": "OpenAlfred",
                "key": "Space",
                "shift": false,
                "ctrl": true,
                "alt": false,
                "logo": false
            },
            {
                "action": "FutureAction",
                "key": "F",
                "shift": false,
                "ctrl": false,
                "alt": true,
                "logo": false
            },
            {
                "action": { "SwitchAccount": { "secret_id": "acct-secret-id" } },
                "key": "1",
                "shift": false,
                "ctrl": true,
                "alt": false,
                "logo": false
            }
        ]),
    );

    let decoded: KeroseneConfig = value_from_json(
        config,
        "config with future hotkey action should deserialize",
    );

    assert_eq!(decoded.hotkeys.len(), 2);
    assert!(
        decoded
            .hotkeys
            .iter()
            .any(|hotkey| hotkey.action == HotkeyAction::OpenAlfred)
    );
    assert!(decoded.hotkeys.iter().any(|hotkey| matches!(
        hotkey.action,
        HotkeyAction::SwitchAccount { ref secret_id } if secret_id == "acct-secret-id"
    )));

    let warnings = take_config_warnings();
    assert!(
        warnings
            .iter()
            .any(|warning| warning == "Invalid hotkey entry in config; dropping hotkey")
    );
    assert!(
        !warnings
            .iter()
            .any(|warning| warning.contains("FutureAction") || warning.contains("acct-secret-id"))
    );
}
