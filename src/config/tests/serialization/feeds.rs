use super::{default_config_value, json_string, remove_field, value_from_json, value_from_str};
use crate::config::KeroseneConfig;

#[test]
fn telegram_feed_channels_round_trip_and_legacy_defaults_marketfeed() {
    let config = KeroseneConfig {
        telegram_feed_channels: vec!["marketfeed".to_string(), "hyperliquid".to_string()],
        telegram_feed_notifications_enabled: true,
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(
        decoded.telegram_feed_channels,
        vec!["marketfeed".to_string(), "hyperliquid".to_string()]
    );
    assert!(decoded.telegram_feed_notifications_enabled);

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "telegram_feed_channels",
        "config should serialize to object",
    );
    remove_field(
        &mut legacy,
        "telegram_feed_notifications_enabled",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert_eq!(decoded_legacy.telegram_feed_channels, vec!["marketfeed"]);
    assert!(!decoded_legacy.telegram_feed_notifications_enabled);
}
