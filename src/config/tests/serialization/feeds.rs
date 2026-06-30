use super::{default_config_value, json_string, remove_field, value_from_json, value_from_str};
use crate::config::{KeroseneConfig, XFeedConfig};
use crate::telegram_feed::TelegramFeedPrivateChannelConfig;
use crate::x_feed::XFeedSource;

#[test]
fn telegram_feed_channels_round_trip_and_legacy_defaults_marketfeed() {
    let config = KeroseneConfig {
        telegram_feed_channels: vec!["marketfeed".to_string(), "hyperliquid".to_string()],
        telegram_feed_private_channels: vec![TelegramFeedPrivateChannelConfig {
            peer_id: 123456,
            title: "Private Macro".to_string(),
        }],
        telegram_feed_notifications_enabled: true,
        telegram_feed_include_outcome_markets: false,
        telegram_feed_onboarding_dismissed: true,
        telegram_feed_fast_mode_enabled: true,
        telegram_feed_fast_api_id: Some(12345),
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(
        decoded.telegram_feed_channels,
        vec!["marketfeed".to_string(), "hyperliquid".to_string()]
    );
    assert_eq!(
        decoded.telegram_feed_private_channels,
        vec![TelegramFeedPrivateChannelConfig {
            peer_id: 123456,
            title: "Private Macro".to_string(),
        }]
    );
    assert!(decoded.telegram_feed_notifications_enabled);
    assert!(decoded.telegram_feed_onboarding_dismissed);
    assert!(decoded.telegram_feed_fast_mode_enabled);
    assert_eq!(decoded.telegram_feed_fast_api_id, Some(12345));

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
    remove_field(
        &mut legacy,
        "telegram_feed_private_channels",
        "config should serialize to object",
    );
    remove_field(
        &mut legacy,
        "telegram_feed_fast_mode_enabled",
        "config should serialize to object",
    );
    remove_field(
        &mut legacy,
        "telegram_feed_fast_api_id",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert_eq!(decoded_legacy.telegram_feed_channels, vec!["marketfeed"]);
    assert!(decoded_legacy.telegram_feed_private_channels.is_empty());
    assert!(!decoded_legacy.telegram_feed_notifications_enabled);
    assert!(!decoded_legacy.telegram_feed_onboarding_dismissed);
    assert!(!decoded_legacy.telegram_feed_fast_mode_enabled);
    assert_eq!(decoded_legacy.telegram_feed_fast_api_id, None);
}

#[test]
fn x_feed_configs_round_trip_and_default_empty() {
    let config = KeroseneConfig {
        x_feeds: vec![
            XFeedConfig {
                id: 7,
                source: XFeedSource::Following,
            },
            XFeedConfig {
                id: 8,
                source: XFeedSource::List {
                    id: "123".to_string(),
                    name: "Macro".to_string(),
                    private: false,
                },
            },
        ],
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");

    assert_eq!(decoded.x_feeds, config.x_feeds);

    let mut legacy = default_config_value();
    remove_field(&mut legacy, "x_feeds", "config should serialize to object");
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert!(decoded_legacy.x_feeds.is_empty());
}
