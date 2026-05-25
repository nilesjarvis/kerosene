use super::{
    DisplayDenominationConfig, KeroseneConfig, default_config_value, json_string, remove_field,
    value_from_json, value_from_str,
};

#[test]
fn display_denomination_round_trips_and_legacy_defaults_usd() {
    let config = KeroseneConfig {
        display_denomination: DisplayDenominationConfig::eur(),
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(
        decoded.display_denomination,
        DisplayDenominationConfig::eur()
    );

    let hype_config = KeroseneConfig {
        display_denomination: DisplayDenominationConfig::hype(),
        ..KeroseneConfig::default()
    };
    let hype_json = json_string(&hype_config, "config should serialize");
    let decoded_hype: KeroseneConfig = value_from_str(&hype_json, "config should deserialize");
    assert_eq!(
        decoded_hype.display_denomination,
        DisplayDenominationConfig::hype()
    );

    let btc_config = KeroseneConfig {
        display_denomination: DisplayDenominationConfig::btc(),
        ..KeroseneConfig::default()
    };
    let btc_json = json_string(&btc_config, "config should serialize");
    let decoded_btc: KeroseneConfig = value_from_str(&btc_json, "config should deserialize");
    assert_eq!(
        decoded_btc.display_denomination,
        DisplayDenominationConfig::btc()
    );

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "display_denomination",
        "config should serialize to object",
    );
    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert_eq!(
        decoded_legacy.display_denomination,
        DisplayDenominationConfig::Usd
    );
}
