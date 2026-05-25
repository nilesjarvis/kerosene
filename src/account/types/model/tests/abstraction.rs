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
