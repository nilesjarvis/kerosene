use super::*;

#[test]
fn portfolio_margin_shared_total_prefers_spot_portfolio_value() {
    let data = account_data_for_shared_total(AccountAbstractionMode::PortfolioMargin, true);

    assert_eq!(
        shared_account_total_value(&data, || Some(100.0)),
        Some(100.0)
    );
}

#[test]
fn shared_non_portfolio_total_uses_spot_fallback_when_larger() {
    let data = account_data_for_shared_total(AccountAbstractionMode::DexAbstraction, false);

    assert_eq!(
        shared_account_total_value(&data, || Some(100.0)),
        Some(100.0)
    );
}

#[test]
fn shared_token_total_uses_selected_collateral_balance() {
    let mut data = account_data_for_shared_total(AccountAbstractionMode::DexAbstraction, false);
    data.spot.balances.push(SpotBalance {
        coin: "XYZC".to_string(),
        token: Some(404),
        total: "2".to_string(),
        hold: "0".to_string(),
        entry_ntl: "25".to_string(),
        supplied: None,
    });

    assert_eq!(
        shared_account_token_total_value(&data, 404, |_| Some(15.0)),
        Some(30.0)
    );
}

#[test]
fn shared_token_total_fails_closed_without_collateral_balance() {
    let data = account_data_for_shared_total(AccountAbstractionMode::DexAbstraction, false);

    assert_eq!(
        shared_account_token_total_value(&data, 404, |_| Some(15.0)),
        None
    );
}

#[test]
fn portfolio_margin_summary_keeps_spot_value_in_selected_hip3_universe() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.market_universe = MarketUniverseConfig::hip3_dex("xyz");
    terminal.muted_tickers.clear();
    let data = account_data_for_shared_total(AccountAbstractionMode::PortfolioMargin, true);

    let values = terminal.connected_summary_values(&data);

    assert_eq!(values.total_value, "100.00");
    assert_eq!(values.available, Some(100.0));
    assert_eq!(values.available_value, "100.00");
}
