use super::*;

#[test]
fn portfolio_margin_available_uses_after_maintenance_value() {
    let data = account_data_for_available_margin(AccountAbstractionMode::PortfolioMargin, true);

    assert_eq!(data.available_margin_usdc(), Some(55.0));
    assert_eq!(data.available_margin_for_token(360), Some(22.0));
}

#[test]
fn unified_available_uses_spot_balance_after_holds() {
    let data = account_data_for_available_margin(AccountAbstractionMode::UnifiedAccount, false);

    assert_eq!(data.available_margin_usdc(), Some(80.0));
    assert_eq!(data.available_margin_for_token(360), Some(25.0));
}

#[test]
fn dex_abstraction_available_keeps_nonzero_spot_fallback() {
    let data = account_data_for_available_margin(AccountAbstractionMode::DexAbstraction, false);

    assert_eq!(data.available_margin_usdc(), Some(80.0));
}

#[test]
fn default_abstraction_keeps_nonzero_spot_fallback() {
    let data = account_data_for_available_margin(AccountAbstractionMode::Default, false);

    assert_eq!(data.available_margin_usdc(), Some(80.0));
}

#[test]
fn unknown_abstraction_fails_closed_for_available_margin() {
    let data = account_data_for_available_margin(
        AccountAbstractionMode::Unknown("unavailable".to_string()),
        false,
    );

    assert_eq!(data.available_margin_usdc(), None);
    assert_eq!(data.available_margin_for_token(360), None);
}
