use super::*;
use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary,
    SpotClearinghouseState, UserFeeRates,
};
use crate::account_analytics::{
    IncomeHourlyPayment, IncomeSnapshot, PortfolioBucket, PortfolioHistory,
};
use crate::config::ReadDataProvider;
use crate::portfolio_state::PnlValueDisplayMode;
use std::collections::HashMap;

#[test]
fn portfolio_pnl_value_mode_updates_even_when_pnl_is_hidden() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.hide_pnl = true;
    terminal.portfolio.pnl_value_display_mode = PnlValueDisplayMode::Usd;

    let _ = terminal.update_portfolio_income(Message::SetPortfolioPnlValueMode(
        PnlValueDisplayMode::Percent,
    ));
    assert_eq!(
        terminal.portfolio.pnl_value_display_mode,
        PnlValueDisplayMode::Percent
    );

    let _ = terminal
        .update_portfolio_income(Message::SetPortfolioPnlValueMode(PnlValueDisplayMode::Usd));
    assert_eq!(
        terminal.portfolio.pnl_value_display_mode,
        PnlValueDisplayMode::Usd
    );
}

#[test]
fn stale_portfolio_result_after_newer_same_address_request_is_ignored() {
    let mut terminal = TradingTerminal::boot().0;
    let address = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some(address.to_string());
    let stale_request_id = terminal.portfolio.begin_refresh();
    let current_request_id = terminal.portfolio.begin_refresh();

    let _ = terminal.update_portfolio_income(Message::PortfolioLoaded(
        address.to_string(),
        stale_request_id,
        Box::new(Ok(portfolio_history(1.0))),
    ));

    assert!(terminal.portfolio.loading);
    assert!(terminal.portfolio.data.is_none());

    let _ = terminal.update_portfolio_income(Message::PortfolioLoaded(
        address.to_string(),
        current_request_id,
        Box::new(Ok(portfolio_history(2.0))),
    ));

    assert!(!terminal.portfolio.loading);
    let history = terminal
        .portfolio
        .data
        .as_ref()
        .expect("current portfolio result should apply");
    assert_eq!(history.buckets["day"].account_value_history[0].1, 2.0);
}

#[test]
fn refresh_portfolio_is_coalesced_while_request_is_in_flight() {
    let mut terminal = TradingTerminal::boot().0;
    let address = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some(address.to_string());

    let _ = terminal.update_portfolio_income(Message::RefreshPortfolio);
    assert!(terminal.portfolio.loading);
    let request_id = terminal.portfolio.refresh_request_id;

    let _ = terminal.update_portfolio_income(Message::RefreshPortfolio);

    assert!(terminal.portfolio.loading);
    assert_eq!(terminal.portfolio.refresh_request_id, request_id);
    assert!(terminal.portfolio.refresh_followup_pending);

    let _ = terminal.update_portfolio_income(Message::PortfolioLoaded(
        address.to_string(),
        request_id,
        Box::new(Ok(portfolio_history(2.0))),
    ));

    assert!(terminal.portfolio.loading);
    assert!(!terminal.portfolio.refresh_followup_pending);
    assert_eq!(
        terminal.portfolio.refresh_request_id,
        request_id.saturating_add(2)
    );
    assert_eq!(
        terminal
            .portfolio
            .data
            .as_ref()
            .expect("current portfolio result should apply")
            .buckets["day"]
            .account_value_history[0]
            .1,
        2.0
    );
}

#[test]
fn current_portfolio_result_for_previous_address_finishes_without_applying() {
    let mut terminal = TradingTerminal::boot().0;
    let previous_address = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some("0xdef0000000000000000000000000000000000000".to_string());
    terminal.portfolio.data = Some(portfolio_history(10.0));
    let request_id = terminal.portfolio.begin_refresh();

    let _ = terminal.update_portfolio_income(Message::PortfolioLoaded(
        previous_address.to_string(),
        request_id,
        Box::new(Ok(portfolio_history(1.0))),
    ));

    assert!(!terminal.portfolio.loading);
    assert_eq!(
        terminal
            .portfolio
            .data
            .as_ref()
            .expect("existing portfolio data")
            .buckets["day"]
            .account_value_history[0]
            .1,
        10.0
    );
}

#[test]
fn stale_income_result_after_newer_same_address_request_is_ignored() {
    let mut terminal = TradingTerminal::boot().0;
    let address = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some(address.to_string());
    let stale_request_id = terminal.income.begin_refresh();
    let current_request_id = terminal.income.begin_refresh();

    let _ = terminal.update_portfolio_income(Message::IncomeLoaded(
        address.to_string(),
        stale_request_id,
        Box::new(Ok(income_snapshot(1, 1.0))),
    ));

    assert!(terminal.income.loading);
    assert!(terminal.income.data.is_none());

    let _ = terminal.update_portfolio_income(Message::IncomeLoaded(
        address.to_string(),
        current_request_id,
        Box::new(Ok(income_snapshot(2, 2.0))),
    ));

    assert!(!terminal.income.loading);
    let income = terminal
        .income
        .data
        .as_ref()
        .expect("current income result should apply");
    assert_eq!(income.earned_total, 2.0);
    assert_eq!(income.recent_hourly_payments[0].time, 2);
}

#[test]
fn refresh_income_is_coalesced_while_request_is_in_flight() {
    let mut terminal = TradingTerminal::boot().0;
    let address = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some(address.to_string());
    terminal.set_account_data_for_address_for_test(address, portfolio_margin_account_data());

    let _ = terminal.update_portfolio_income(Message::RefreshIncome);
    assert!(terminal.income.loading);
    let request_id = terminal.income.refresh_request_id;

    let _ = terminal.update_portfolio_income(Message::RefreshIncome);

    assert!(terminal.income.loading);
    assert_eq!(terminal.income.refresh_request_id, request_id);
    assert!(terminal.income.refresh_followup_pending);

    let _ = terminal.update_portfolio_income(Message::IncomeLoaded(
        address.to_string(),
        request_id,
        Box::new(Ok(income_snapshot(2, 2.0))),
    ));

    assert!(terminal.income.loading);
    assert!(!terminal.income.refresh_followup_pending);
    assert_eq!(
        terminal.income.refresh_request_id,
        request_id.saturating_add(2)
    );
    assert_eq!(
        terminal
            .income
            .data
            .as_ref()
            .expect("current income result should apply")
            .earned_total,
        2.0
    );
}

#[test]
fn current_income_result_for_previous_address_finishes_without_applying() {
    let mut terminal = TradingTerminal::boot().0;
    let previous_address = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some("0xdef0000000000000000000000000000000000000".to_string());
    terminal.income.data = Some(income_snapshot(10, 10.0));
    let request_id = terminal.income.begin_refresh();

    let _ = terminal.update_portfolio_income(Message::IncomeLoaded(
        previous_address.to_string(),
        request_id,
        Box::new(Ok(income_snapshot(1, 1.0))),
    ));

    assert!(!terminal.income.loading);
    assert_eq!(
        terminal
            .income
            .data
            .as_ref()
            .expect("existing income data")
            .earned_total,
        10.0
    );
}

#[test]
fn provider_change_invalidates_in_flight_portfolio_and_income_results() {
    let mut terminal = TradingTerminal::boot().0;
    let address = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some(address.to_string());
    terminal.read_data_provider = ReadDataProvider::Hyperliquid;
    terminal.portfolio.data = Some(portfolio_history(10.0));
    terminal.income.data = Some(income_snapshot(10, 10.0));
    let portfolio_request_id = terminal.portfolio.begin_refresh();
    let income_request_id = terminal.income.begin_refresh();

    let _ = terminal.update_preferences(Message::ReadDataProviderChanged(
        ReadDataProvider::Hydromancer,
    ));
    let _ = terminal.update_portfolio_income(Message::PortfolioLoaded(
        address.to_string(),
        portfolio_request_id,
        Box::new(Ok(portfolio_history(1.0))),
    ));
    let _ = terminal.update_portfolio_income(Message::IncomeLoaded(
        address.to_string(),
        income_request_id,
        Box::new(Ok(income_snapshot(1, 1.0))),
    ));

    assert!(!terminal.portfolio.loading);
    assert!(!terminal.income.loading);
    assert_eq!(
        terminal
            .portfolio
            .data
            .as_ref()
            .expect("portfolio data")
            .buckets["day"]
            .account_value_history[0]
            .1,
        10.0
    );
    assert_eq!(
        terminal
            .income
            .data
            .as_ref()
            .expect("income data")
            .earned_total,
        10.0
    );
}

#[test]
fn hydromancer_key_generation_change_invalidates_in_flight_portfolio_and_income_results() {
    let mut terminal = TradingTerminal::boot().0;
    let address = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some(address.to_string());
    terminal.portfolio.data = Some(portfolio_history(10.0));
    terminal.income.data = Some(income_snapshot(10, 10.0));
    let portfolio_request_id = terminal.portfolio.begin_refresh();
    let income_request_id = terminal.income.begin_refresh();

    terminal.bump_hydromancer_key_generation();
    let _ = terminal.update_portfolio_income(Message::PortfolioLoaded(
        address.to_string(),
        portfolio_request_id,
        Box::new(Ok(portfolio_history(1.0))),
    ));
    let _ = terminal.update_portfolio_income(Message::IncomeLoaded(
        address.to_string(),
        income_request_id,
        Box::new(Ok(income_snapshot(1, 1.0))),
    ));

    assert!(!terminal.portfolio.loading);
    assert!(!terminal.income.loading);
    assert_eq!(
        terminal
            .portfolio
            .data
            .as_ref()
            .expect("portfolio data")
            .buckets["day"]
            .account_value_history[0]
            .1,
        10.0
    );
    assert_eq!(
        terminal
            .income
            .data
            .as_ref()
            .expect("income data")
            .earned_total,
        10.0
    );
}

fn portfolio_history(value: f64) -> PortfolioHistory {
    let mut buckets = HashMap::new();
    buckets.insert(
        "day".to_string(),
        PortfolioBucket {
            account_value_history: vec![(1, value)],
            pnl_history: vec![(1, value)],
            vlm: None,
            skipped_invalid_points: 0,
            invalid_vlm: false,
        },
    );
    PortfolioHistory { buckets }
}

fn income_snapshot(time: u64, earned_total: f64) -> IncomeSnapshot {
    IncomeSnapshot {
        earned_total,
        earned_24h: earned_total,
        earned_7d: earned_total,
        earned_30d: earned_total,
        net_yearly_projection: 0.0,
        current_supply_usd: 0.0,
        current_borrow_usd: 0.0,
        health: "healthy".to_string(),
        health_factor: None,
        token_rows: Vec::new(),
        recent_hourly_payments: vec![IncomeHourlyPayment {
            time,
            token_label: "USDC".to_string(),
            supply: earned_total,
            borrow: 0.0,
            net: earned_total,
        }],
        invalid_token_rows: 0,
        invalid_interest_rows: 0,
    }
}

fn portfolio_margin_account_data() -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "0".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "0".to_string(),
            asset_positions: Vec::new(),
        },
        clearinghouses_by_dex: HashMap::new(),
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: true,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: UserFeeRates::default(),
        completeness: AccountDataCompleteness::default(),
        fetched_at_ms: TradingTerminal::now_ms(),
    }
}
