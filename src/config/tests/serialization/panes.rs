use super::{json_string, value_from_json};
use crate::config::{KeroseneConfig, PaneKindConfig, PaneLayoutConfig};
use crate::session_data_state::SessionDataLookback;

#[test]
fn legacy_assistant_pane_deserializes_as_unsupported() {
    let layout: PaneLayoutConfig = value_from_json(
        serde_json::json!({"Leaf": "Assistant"}),
        "legacy assistant pane should deserialize",
    );

    assert_eq!(layout, PaneLayoutConfig::Leaf(PaneKindConfig::Unsupported));
}

#[test]
fn telegram_feed_pane_round_trips() {
    let layout = PaneLayoutConfig::Leaf(PaneKindConfig::TelegramFeed);

    let json = json_string(&layout, "telegram feed pane should serialize");
    let decoded: PaneLayoutConfig =
        serde_json::from_str(&json).expect("telegram feed pane should deserialize");

    assert_eq!(decoded, layout);
}

#[test]
fn x_feed_pane_round_trips() {
    let layout = PaneLayoutConfig::Leaf(PaneKindConfig::XFeed);

    let json = json_string(&layout, "x feed pane should serialize");
    let decoded: PaneLayoutConfig =
        serde_json::from_str(&json).expect("x feed pane should deserialize");

    assert_eq!(decoded, layout);
}

#[test]
fn session_data_pane_and_config_round_trip() {
    let layout = PaneLayoutConfig::Leaf(PaneKindConfig::SessionData { id: 42 });

    let json = json_string(&layout, "session data pane should serialize");
    let decoded: PaneLayoutConfig =
        serde_json::from_str(&json).expect("session data pane should deserialize");

    assert_eq!(decoded, layout);

    let mut config = KeroseneConfig::default();
    config.session_data.push(crate::config::SessionDataConfig {
        id: 42,
        symbol: "@107".to_string(),
        lookback: SessionDataLookback::EightWeeks,
    });
    let json = json_string(&config, "session data config should serialize");
    let decoded: KeroseneConfig =
        serde_json::from_str(&json).expect("session data config should deserialize");

    assert_eq!(decoded.session_data.len(), 1);
    assert_eq!(decoded.session_data[0].id, 42);
    assert_eq!(decoded.session_data[0].symbol, "@107");
    assert_eq!(
        decoded.session_data[0].lookback,
        SessionDataLookback::EightWeeks
    );
}

#[test]
fn serialized_config_omits_removed_assistant_settings() {
    let json = json_string(
        &KeroseneConfig::default(),
        "default config should serialize",
    );

    assert!(!json.contains("assistant_api_key"));
    assert!(!json.contains("assistant_model"));
    assert!(!json.contains("assistant_use_account_context"));
    assert!(!json.contains("assistant_allow_code_execution"));
}
