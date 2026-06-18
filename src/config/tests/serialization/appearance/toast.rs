use super::super::super::config_warning_guard;
use super::{
    KeroseneConfig, ToastPosition, default_config_value, json_string, object_mut, value_from_json,
    value_from_str,
};
use crate::config::take_config_warnings;

#[test]
fn toast_settings_round_trip() {
    let config = KeroseneConfig {
        toast_position: ToastPosition::BottomLeft,
        toast_animations_enabled: false,
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(decoded.toast_position, ToastPosition::BottomLeft);
    assert!(!decoded.toast_animations_enabled);
}

#[test]
fn toast_settings_legacy_defaults() {
    let mut legacy = default_config_value();
    let object = object_mut(&mut legacy, "config should serialize to object");
    object.remove("toast_position");
    object.remove("toast_animations_enabled");

    let decoded: KeroseneConfig = value_from_json(legacy, "legacy config should deserialize");
    assert_eq!(decoded.toast_position, ToastPosition::default());
    assert_eq!(decoded.toast_position, ToastPosition::TopRight);
    assert!(decoded.toast_animations_enabled);
}

#[test]
fn toast_position_unknown_value_defaults_with_warning() {
    let _warning_guard = config_warning_guard();
    let mut config = default_config_value();
    object_mut(&mut config, "config should serialize to object").insert(
        "toast_position".to_string(),
        serde_json::json!("FutureCorner"),
    );

    let decoded: KeroseneConfig = value_from_json(config, "future toast config should deserialize");

    assert_eq!(decoded.toast_position, ToastPosition::TopRight);
    assert!(
        take_config_warnings()
            .iter()
            .any(|warning| warning.contains("Unknown toast position \"FutureCorner\""))
    );
}
