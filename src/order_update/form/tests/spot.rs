use super::{OrderSizingBasis, TEST_ACCOUNT, TradingTerminal, account_data_with_positions, symbol};
use crate::account::{AccountAbstractionMode, AccountData, SpotBalance};
use crate::api::{ExchangeSymbol, MarketType};
use crate::signing::OrderKind;

fn spot_symbol(key: &str, ticker: &str, sz_decimals: u32) -> ExchangeSymbol {
    ExchangeSymbol {
        ticker: ticker.to_string(),
        category: "spot".to_string(),
        display_name: Some(format!("{ticker}/USDC")),
        sz_decimals,
        ..symbol(key, MarketType::Spot)
    }
}

fn spot_balance(coin: &str, token: Option<u32>, total: &str, hold: &str) -> SpotBalance {
    SpotBalance {
        coin: coin.to_string(),
        token,
        total: total.to_string(),
        hold: hold.to_string(),
        entry_ntl: "0".to_string(),
        supplied: None,
    }
}

fn spot_account_data(
    abstraction: AccountAbstractionMode,
    withdrawable: &str,
    balances: Vec<SpotBalance>,
) -> AccountData {
    let mut data = account_data_with_positions(Vec::new());
    data.account_abstraction = abstraction;
    data.clearinghouse.withdrawable = withdrawable.to_string();
    data.spot.balances = balances;
    data
}

fn spot_terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = "@107".to_string();
    terminal.active_symbol_display = "HYPE/USDC".to_string();
    terminal.exchange_symbols = vec![spot_symbol("@107", "HYPE", 2)];
    terminal.order_kind = OrderKind::Market;
    terminal.order_price.clear();
    terminal
}

#[test]
fn spot_sizing_uses_sellable_base_balance_not_usdc_margin() {
    let terminal = spot_terminal();
    let data = spot_account_data(
        AccountAbstractionMode::Default,
        "5000",
        vec![
            spot_balance("HYPE", Some(107), "100", "25"),
            spot_balance("USDC", Some(0), "50", "0"),
        ],
    );

    let Some(OrderSizingBasis::SpotSellableBalance { sellable_size_coin }) =
        terminal.order_sizing_basis(&data)
    else {
        panic!("expected spot sellable-balance sizing basis");
    };

    assert_eq!(sellable_size_coin, 75.0);
}

#[test]
fn spot_slider_sizes_full_sellable_balance_for_sell_all() {
    let mut terminal = spot_terminal();
    terminal.order_quantity_is_usd = false;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        spot_account_data(
            AccountAbstractionMode::Default,
            "50000",
            vec![spot_balance("HYPE", Some(107), "100", "0")],
        ),
    );

    terminal.handle_order_percentage_changed(100.0);

    assert_eq!(terminal.order_quantity, "100.00");
}

#[test]
fn spot_manual_quantity_maps_percentage_against_sellable_balance() {
    let mut terminal = spot_terminal();
    terminal.order_quantity_is_usd = false;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        spot_account_data(
            AccountAbstractionMode::Default,
            "50000",
            vec![spot_balance("HYPE", Some(107), "200", "0")],
        ),
    );

    terminal.handle_order_quantity_changed("50".to_string());

    assert_eq!(terminal.order_percentage, 25.0);
}

#[test]
fn spot_sizing_floors_sellable_balance_to_size_decimals() {
    let terminal = spot_terminal();
    let data = spot_account_data(
        AccountAbstractionMode::Default,
        "0",
        vec![spot_balance("HYPE", Some(107), "99.999999", "0")],
    );

    let Some(OrderSizingBasis::SpotSellableBalance { sellable_size_coin }) =
        terminal.order_sizing_basis(&data)
    else {
        panic!("expected spot sellable-balance sizing basis");
    };

    assert_eq!(sellable_size_coin, 99.99);
}

#[test]
fn spot_buy_sizing_uses_spot_usdc_not_perp_withdrawable_without_abstraction() {
    let terminal = spot_terminal();
    let data = spot_account_data(
        AccountAbstractionMode::Disabled,
        "5000",
        vec![spot_balance("USDC", Some(0), "1000", "0")],
    );

    let Some(OrderSizingBasis::MarginNotional { max_notional }) =
        terminal.order_sizing_basis(&data)
    else {
        panic!("expected spot USDC sizing basis");
    };

    assert_eq!(max_notional, 1_000.0);
}

#[test]
fn spot_buy_sizing_sees_spot_usdc_for_spot_only_disabled_abstraction_account() {
    let mut terminal = spot_terminal();
    terminal.order_quantity_is_usd = true;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        spot_account_data(
            AccountAbstractionMode::Disabled,
            "0",
            vec![spot_balance("USDC", Some(0), "10000", "0")],
        ),
    );

    terminal.handle_order_percentage_changed(50.0);

    assert_eq!(terminal.order_quantity, "5,000.00");
}

#[test]
fn spot_buy_sizing_keeps_shared_balance_for_abstracted_accounts() {
    let terminal = spot_terminal();
    let data = spot_account_data(
        AccountAbstractionMode::Default,
        "5000",
        vec![spot_balance("USDC", Some(0), "1000", "0")],
    );

    let Some(OrderSizingBasis::MarginNotional { max_notional }) =
        terminal.order_sizing_basis(&data)
    else {
        panic!("expected shared-balance sizing basis");
    };

    assert_eq!(max_notional, 5_000.0);
}

#[test]
fn spot_sizing_falls_back_to_quote_basis_when_base_balance_is_fully_held() {
    let terminal = spot_terminal();
    let data = spot_account_data(
        AccountAbstractionMode::Disabled,
        "0",
        vec![
            spot_balance("HYPE", Some(107), "10", "10"),
            spot_balance("USDC", Some(0), "250", "0"),
        ],
    );

    let Some(OrderSizingBasis::MarginNotional { max_notional }) =
        terminal.order_sizing_basis(&data)
    else {
        panic!("expected spot USDC sizing basis");
    };

    assert_eq!(max_notional, 250.0);
}

#[test]
fn perp_margin_sizing_is_unchanged_when_spot_balances_exist() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    let data = spot_account_data(
        AccountAbstractionMode::Default,
        "1000",
        vec![
            spot_balance("HYPE", Some(107), "100", "0"),
            spot_balance("USDC", Some(0), "2000", "0"),
        ],
    );

    let Some(OrderSizingBasis::MarginNotional { max_notional }) =
        terminal.order_sizing_basis(&data)
    else {
        panic!("expected margin sizing basis");
    };

    assert_eq!(max_notional, 2_000.0);
}

#[test]
fn spot_buy_sizing_uses_the_selected_pairs_quote_token() {
    let mut terminal = spot_terminal();
    terminal.active_symbol = "@232".to_string();
    terminal.active_symbol_display = "HYPE/USDH".to_string();
    terminal.exchange_symbols = vec![ExchangeSymbol {
        collateral_token: Some(360),
        display_name: Some("HYPE/USDH".to_string()),
        ..spot_symbol("@232", "HYPE", 2)
    }];
    let data = spot_account_data(
        AccountAbstractionMode::Disabled,
        "0",
        vec![
            spot_balance("HYPE", Some(150), "2", "0"),
            spot_balance("USDC", Some(0), "50000", "0"),
            spot_balance("USDH", Some(360), "750", "50"),
        ],
    );

    let Some(OrderSizingBasis::MarginNotional { max_notional }) =
        terminal.spot_order_sizing_basis_for_side("@232", &data, true)
    else {
        panic!("expected exact quote-token sizing basis");
    };

    assert_eq!(max_notional, 700.0);
}

#[test]
fn spot_non_usdc_buy_without_quote_token_metadata_fails_closed() {
    let mut terminal = spot_terminal();
    terminal.active_symbol = "@232".to_string();
    terminal.exchange_symbols = vec![ExchangeSymbol {
        display_name: Some("HYPE/USDH".to_string()),
        collateral_token: None,
        ..spot_symbol("@232", "HYPE", 2)
    }];
    let data = spot_account_data(
        AccountAbstractionMode::Disabled,
        "0",
        vec![
            spot_balance("USDC", Some(0), "50000", "0"),
            spot_balance("USDH", Some(360), "750", "0"),
        ],
    );

    assert!(
        terminal
            .spot_order_sizing_basis_for_side("@232", &data, true)
            .is_none()
    );
}

#[test]
fn crypto_quoted_spot_percentage_fails_closed_for_all_denominations() {
    let mut terminal = spot_terminal();
    terminal.active_symbol = "@151".to_string();
    terminal.active_symbol_display = "HYPE/UETH".to_string();
    terminal.exchange_symbols = vec![ExchangeSymbol {
        collateral_token: Some(221),
        display_name: Some("HYPE/UETH".to_string()),
        ..spot_symbol("@151", "HYPE", 2)
    }];
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        spot_account_data(
            AccountAbstractionMode::Disabled,
            "0",
            vec![spot_balance("UETH", Some(221), "10", "0")],
        ),
    );
    terminal.all_mids.insert("@151".to_string(), 2.0);
    terminal
        .all_mids_updated_at_ms
        .insert("@151".to_string(), TradingTerminal::now_ms());

    terminal.order_quantity_is_usd = true;
    terminal.handle_order_percentage_changed(50.0);
    assert_eq!(terminal.order_quantity, "0");
    assert!(terminal.order_quantity_provenance.is_none());

    terminal.order_quantity_is_usd = false;
    terminal.handle_order_percentage_changed(50.0);
    assert_eq!(terminal.order_quantity, "0");
    assert!(terminal.order_quantity_provenance.is_none());
}

#[test]
fn spot_percentage_quantity_is_recomputed_for_the_clicked_side() {
    let mut terminal = spot_terminal();
    terminal.order_quantity_is_usd = false;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        spot_account_data(
            AccountAbstractionMode::Disabled,
            "0",
            vec![
                spot_balance("HYPE", Some(150), "5", "0"),
                spot_balance("USDC", Some(0), "1000", "0"),
            ],
        ),
    );
    terminal.all_mids.insert("@107".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("@107".to_string(), TradingTerminal::now_ms());

    terminal.handle_order_percentage_changed(100.0);
    assert_eq!(terminal.order_quantity, "5.00", "preview uses held base");

    assert!(
        terminal
            .stale_percentage_order_quantity_task("placing an order", true)
            .is_some(),
        "a changed visible quantity must require review"
    );
    assert_eq!(terminal.order_quantity, "10.00", "buy spends quote");
    assert!(
        terminal
            .stale_percentage_order_quantity_task("placing an order", true)
            .is_none(),
        "the reviewed side-specific quantity may submit"
    );

    terminal.handle_order_percentage_changed(100.0);
    assert!(
        terminal
            .stale_percentage_order_quantity_task("placing an order", false)
            .is_none()
    );
    assert_eq!(terminal.order_quantity, "5.00", "sell uses base");
}

#[test]
fn unrelated_account_revision_does_not_invalidate_spot_percentage_size() {
    let mut terminal = spot_terminal();
    terminal.order_quantity_is_usd = false;
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    let mut data = spot_account_data(
        AccountAbstractionMode::Disabled,
        "0",
        vec![
            spot_balance("HYPE", Some(150), "5", "0"),
            spot_balance("USDC", Some(0), "1000", "0"),
        ],
    );
    data.completeness.positions_complete = false;
    data.completeness.positions_actionable = false;
    data.completeness.positions_fetched_at_ms = Some(1);
    data.mark_spot_balances_fetched_at(TradingTerminal::now_ms());
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, data);
    terminal.all_mids.insert("@107".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("@107".to_string(), TradingTerminal::now_ms());

    terminal.handle_order_percentage_changed(50.0);
    terminal.bump_account_data_revision();

    assert!(
        terminal
            .stale_percentage_order_quantity_task("placing an order", false)
            .is_none(),
        "perp-only changes must not block a verified spot percentage order"
    );
}

#[test]
fn spot_balance_revision_change_requires_percentage_reselection() {
    let mut terminal = spot_terminal();
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        spot_account_data(
            AccountAbstractionMode::Disabled,
            "0",
            vec![spot_balance("USDC", Some(0), "1000", "0")],
        ),
    );

    terminal.handle_order_percentage_changed(50.0);
    terminal.bump_spot_balances_revision();

    assert!(
        terminal
            .stale_percentage_order_quantity_task("placing an order", true)
            .is_some()
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| *is_error
                && message.contains("older spot balances"))
    );
}

#[test]
fn stale_spot_balances_block_percentage_submission_even_when_positions_are_fresh() {
    let mut terminal = spot_terminal();
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    let now_ms = TradingTerminal::now_ms();
    let mut data = spot_account_data(
        AccountAbstractionMode::Disabled,
        "0",
        vec![spot_balance("USDC", Some(0), "1000", "0")],
    );
    data.mark_positions_fetched_at(now_ms);
    data.mark_spot_balances_fetched_at(
        now_ms.saturating_sub(AccountData::POSITION_ACTION_MAX_AGE_MS + 1),
    );
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, data);

    terminal.handle_order_percentage_changed(50.0);

    assert!(
        terminal
            .stale_percentage_order_quantity_task("placing an order", true)
            .is_some()
    );
    assert!(terminal.order_status.as_ref().is_some_and(
        |(message, is_error)| *is_error && message.contains("Spot balances are stale")
    ));
}

#[test]
fn incomplete_spot_balances_block_percentage_submission() {
    let mut terminal = spot_terminal();
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    let mut data = spot_account_data(
        AccountAbstractionMode::Disabled,
        "0",
        vec![spot_balance("USDC", Some(0), "1000", "0")],
    );
    data.completeness.spot_balances_complete = false;
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, data);

    terminal.handle_order_percentage_changed(50.0);

    assert!(
        terminal
            .stale_percentage_order_quantity_task("placing an order", true)
            .is_some()
    );
    assert!(terminal.order_status.as_ref().is_some_and(
        |(message, is_error)| *is_error && message.contains("Spot balances may be incomplete")
    ));
}
