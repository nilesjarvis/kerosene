use super::*;

#[test]
fn account_abstraction_mode_parses_known_api_values() {
    assert_eq!(
        AccountAbstractionMode::from_api_value("portfolioMargin"),
        AccountAbstractionMode::PortfolioMargin
    );
    assert_eq!(
        AccountAbstractionMode::from_api_value("unifiedAccount"),
        AccountAbstractionMode::UnifiedAccount
    );
    assert_eq!(
        AccountAbstractionMode::from_api_value("dexAbstraction"),
        AccountAbstractionMode::DexAbstraction
    );
}

#[test]
fn unknown_account_abstraction_debug_redacts_external_value() {
    let raw = "private-account-mode-sentinel";
    let mode = AccountAbstractionMode::from_api_value(raw);

    let rendered = format!("{mode:?}");

    assert!(rendered.contains("Unknown"), "{rendered}");
    assert!(rendered.contains("<redacted>"), "{rendered}");
    assert!(!rendered.contains(raw), "{rendered}");
    assert_eq!(mode, AccountAbstractionMode::Unknown(raw.to_string()));
}
