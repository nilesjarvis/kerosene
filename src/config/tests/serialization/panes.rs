use super::{json_string, value_from_json};
use crate::config::{KeroseneConfig, PaneKindConfig, PaneLayoutConfig};

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
