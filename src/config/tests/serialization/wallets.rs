use super::{default_config_value, json_string, remove_field, value_from_json, value_from_str};
use crate::config::{CombinedPortfolioConfig, KeroseneConfig, TrackedWalletConfig};

const ADDRESS: &str = "0x1111111111111111111111111111111111111111";

#[test]
fn combined_portfolio_round_trips_and_legacy_configs_default_empty() {
    let config = KeroseneConfig {
        combined_portfolio: CombinedPortfolioConfig {
            wallets: vec![TrackedWalletConfig {
                address: ADDRESS.to_string(),
                label: "Primary".to_string(),
            }],
            open: true,
            width: 1200.0,
            height: 800.0,
            x: Some(20.0),
            y: Some(40.0),
        },
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "combined portfolio config should serialize");
    let decoded: KeroseneConfig =
        value_from_str(&json, "combined portfolio config should deserialize");
    assert_eq!(decoded.combined_portfolio.wallets.len(), 1);
    assert_eq!(decoded.combined_portfolio.wallets[0].address, ADDRESS);
    assert_eq!(decoded.combined_portfolio.wallets[0].label, "Primary");
    assert!(decoded.combined_portfolio.open);
    assert_eq!(decoded.combined_portfolio.x, Some(20.0));

    let mut legacy = default_config_value();
    remove_field(
        &mut legacy,
        "combined_portfolio",
        "default config should serialize to an object",
    );
    let legacy: KeroseneConfig = value_from_json(legacy, "legacy config should deserialize");
    assert!(legacy.combined_portfolio.wallets.is_empty());
    assert!(!legacy.combined_portfolio.open);
}
