use super::super::format_position_usd_value;
use super::*;

#[test]
fn position_entry_price_groups_large_wire_values() {
    assert_eq!(
        format_position_entry_price(Some(12345.678), "12345.678"),
        "12,345.678"
    );
    assert_eq!(
        format_position_entry_price(Some(100000.0), "100000"),
        "100,000"
    );
}

#[test]
fn position_entry_price_preserves_small_wire_values() {
    assert_eq!(
        format_position_entry_price(Some(0.00001234), "0.00001234"),
        "0.00001234"
    );
    assert_eq!(format_position_entry_price(None, "100000"), "Invalid");
}

#[test]
fn spot_position_entry_price_rounds_to_two_decimals() {
    assert_eq!(
        format_spot_position_entry_price(Some(72.092843191)),
        "72.09"
    );
    assert_eq!(
        format_spot_position_entry_price(Some(60191.1234)),
        "60,191.12"
    );
    assert_eq!(format_spot_position_entry_price(None), "Invalid");
}

#[test]
fn compact_position_usd_rounds_to_whole_dollars() {
    assert_eq!(
        format_position_usd_value(1234.56, PositionNumberMode::Full),
        "$1,234.56"
    );
    assert_eq!(
        format_position_usd_value(1234.56, PositionNumberMode::Compact),
        "$1,235"
    );
    assert_eq!(
        format_position_usd_value(-1234.56, PositionNumberMode::Compact),
        "-$1,235"
    );
    assert_eq!(
        format_position_usd_value(0.5, PositionNumberMode::Compact),
        "$1"
    );
    assert_eq!(
        format_position_usd_value(532_023.0, PositionNumberMode::Compact),
        "$500k"
    );
}

#[test]
fn compact_signed_amount_rounds_to_whole_values() {
    let denomination = crate::denomination::DisplayDenominationContext::default();
    assert_eq!(
        format_position_signed_amount(&denomination, 12.34, PositionNumberMode::Full),
        "+$12.34"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, 12345.67, PositionNumberMode::Full),
        "+$12,345.67"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, -1234567.89, PositionNumberMode::Full),
        "-$1,234,567.89"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, 12.56, PositionNumberMode::Compact),
        "+$13"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, -12.56, PositionNumberMode::Compact),
        "-$13"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, 12345.67, PositionNumberMode::Compact),
        "+$12k"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, 532_023.0, PositionNumberMode::Compact),
        "+$500k"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, -1234567.89, PositionNumberMode::Compact),
        "-$1.2M"
    );
    assert_eq!(
        format_position_signed_amount(&denomination, -0.49, PositionNumberMode::Compact),
        "$0"
    );
}

#[test]
fn compact_position_size_trims_unneeded_zeroes() {
    let terminal = crate::app_state::TradingTerminal::boot().0;

    assert_eq!(
        terminal.display_position_size("BTC", 1.0, PositionNumberMode::Compact),
        "1"
    );
    assert_eq!(
        terminal.display_position_size("BTC", 1.25, PositionNumberMode::Compact),
        "1.25"
    );
    assert_eq!(format_position_compact_number(12_500.0), "13k");
    assert_eq!(format_position_compact_number(532_023.0), "500k");
}

#[test]
fn spot_position_size_keeps_small_position_precision() {
    let mut terminal = crate::app_state::TradingTerminal::boot().0;
    terminal.exchange_symbols.push(spot_symbol("@107", "HYPE"));
    terminal.exchange_symbols.push(spot_symbol("@151", "UETH"));

    assert_eq!(
        terminal.display_position_size("@107", 6.7491729032, PositionNumberMode::Full),
        "6.7492"
    );
    assert_eq!(
        terminal.display_position_size("@107", 6.7491729032, PositionNumberMode::Compact),
        "6.7492"
    );
    assert_eq!(
        terminal.display_position_size("@107", 2.0, PositionNumberMode::Full),
        "2"
    );
    // Regression: a ~$10 UETH position (min spot notional) used to round to
    // two decimals and display as "0".
    assert_eq!(
        terminal.display_position_size("@151", 0.0037, PositionNumberMode::Full),
        "0.0037"
    );
    assert_eq!(
        terminal.display_position_size("@151", 0.0037, PositionNumberMode::Compact),
        "0.0037"
    );
}

#[test]
fn bitcoin_spot_position_size_keeps_existing_precision() {
    let mut terminal = crate::app_state::TradingTerminal::boot().0;
    terminal.exchange_symbols.push(spot_symbol("@142", "UBTC"));

    assert_eq!(
        terminal.display_position_size("@142", 0.12345678, PositionNumberMode::Full),
        "0.1235"
    );
    assert_eq!(
        terminal.display_position_size("@142", 0.12345678, PositionNumberMode::Compact),
        "0.1235"
    );
}

#[test]
fn spot_size_rounding_does_not_change_perps_or_hip4_outcomes() {
    let mut terminal = crate::app_state::TradingTerminal::boot().0;
    terminal.exchange_symbols.push(outcome_symbol("#950"));

    assert_eq!(
        terminal.display_position_size("ETH", 6.7491729032, PositionNumberMode::Full),
        "6.7492"
    );
    assert_eq!(
        terminal.display_position_size("ETH", 6.7491729032, PositionNumberMode::Compact),
        "6.7492"
    );
    assert_eq!(
        terminal.display_position_size("#950", 6.7491729032, PositionNumberMode::Full),
        "7"
    );
    assert_eq!(
        terminal.display_position_size("#950", 6.7491729032, PositionNumberMode::Compact),
        "7"
    );
}

fn outcome_symbol(key: &str) -> crate::api::ExchangeSymbol {
    crate::api::ExchangeSymbol {
        key: key.to_string(),
        ticker: "OUT95-YES".to_string(),
        category: "outcome".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        market_type: crate::api::MarketType::Outcome,
        outcome: Some(crate::api::OutcomeSymbolInfo {
            outcome_id: 95,
            question_id: None,
            question_name: Some("Will BTC close green?".to_string()),
            question_description: None,
            question_class: None,
            question_underlying: None,
            question_expiry: None,
            question_price_thresholds: Vec::new(),
            question_period: None,
            question_named_outcomes: Vec::new(),
            question_settled_named_outcomes: Vec::new(),
            question_fallback_outcome: None,
            bucket_index: None,
            is_question_fallback: false,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "Recurring".to_string(),
            description: "Will BTC close green?".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDH".to_string(),
            quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
            encoding: 950,
        }),
    }
}

fn spot_symbol(key: &str, ticker: &str) -> crate::api::ExchangeSymbol {
    crate::api::ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "spot".to_string(),
        display_name: Some(format!("{ticker}/USDC")),
        keywords: Vec::new(),
        asset_index: 10_000,
        collateral_token: None,
        sz_decimals: 5,
        max_leverage: 1,
        only_isolated: false,
        market_type: crate::api::MarketType::Spot,
        outcome: None,
    }
}

fn spot_position_without_cost_basis(coin: &str) -> crate::account::AssetPosition {
    crate::account::AssetPosition {
        position: crate::account::Position {
            coin: coin.to_string(),
            szi: "2".to_string(),
            entry_px: String::new(),
            position_value: "200".to_string(),
            unrealized_pnl: String::new(),
            liquidation_px: None,
            leverage: crate::account::PositionLeverage {
                leverage_type: "spot".to_string(),
                value: 1,
            },
            margin_used: String::new(),
            cum_funding: None,
        },
        liquidation_px: None,
    }
}

#[test]
fn position_row_symbol_label_resolves_outcome_coins_without_synthetic_ticker() {
    let mut terminal = crate::app_state::TradingTerminal::boot().0;
    terminal.exchange_symbols.push(outcome_symbol("#950"));
    terminal.exchange_symbols.push(spot_symbol("@107", "HYPE"));
    terminal
        .outcome_display_labels
        .insert("#960".to_string(), "NO: Will ETH close red?".to_string());

    assert_eq!(
        terminal.position_row_symbol_label("#950"),
        "YES: Will BTC close green?"
    );
    // Expired markets resolve through the persisted label cache, not the
    // raw "#NNN" key.
    assert_eq!(
        terminal.position_row_symbol_label("#960"),
        "NO: Will ETH close red?"
    );
    assert_eq!(terminal.position_row_symbol_label("@107"), "HYPE");
    assert_eq!(terminal.position_row_symbol_label("BTC"), "BTC");
}

#[test]
fn spot_position_symbol_uses_ticker_for_label_and_logo() {
    let mut terminal = crate::app_state::TradingTerminal::boot().0;
    terminal.exchange_symbols.push(spot_symbol("@142", "UBTC"));

    assert_eq!(terminal.position_row_symbol_label("@142"), "UBTC");
    let icon_key = terminal.position_row_symbol_icon_key("@142");
    assert_eq!(icon_key, "UBTC");
    assert!(crate::helpers::symbol_svg_logo(icon_key).is_some());
}

#[test]
fn position_row_symbol_label_splits_hip3_exchange_from_asset() {
    let terminal = crate::app_state::TradingTerminal::boot().0;

    assert_eq!(terminal.position_row_symbol_label("xyz:NVDA"), "NVDA");
    assert_eq!(
        terminal.position_row_symbol_exchange_label("xyz:NVDA"),
        Some("xyz".to_string())
    );
    assert_eq!(terminal.position_row_symbol_exchange_label("BTC"), None);
    assert_eq!(terminal.position_row_symbol_exchange_label("@107"), None);
}

#[test]
fn spot_position_without_cost_basis_displays_unavailable_pnl() {
    let mut terminal = crate::app_state::TradingTerminal::boot().0;
    terminal.exchange_symbols.push(spot_symbol("@142", "UBTC"));
    terminal.all_mids.insert("@142".to_string(), 100.0);
    terminal.all_mids_updated_at_ms.insert(
        "@142".to_string(),
        crate::app_state::TradingTerminal::now_ms(),
    );
    let data = terminal.position_row_data(&spot_position_without_cost_basis("@142"));
    let denomination = crate::denomination::DisplayDenominationContext::default();

    let displays =
        terminal.position_row_pnl_displays(&data, &denomination, PositionNumberMode::Full);

    assert_eq!(displays.value, "$200.00");
    assert_eq!(displays.upnl, "-");
    assert_eq!(displays.funding, "-");
    assert_eq!(displays.total, "-");
}

#[test]
fn projected_size_label_keeps_magnitude_for_same_side_changes() {
    let terminal = crate::app_state::TradingTerminal::boot().0;

    assert_eq!(
        terminal.projected_position_size_label("BTC", 1.0, 1.0, PositionNumberMode::Compact),
        "2"
    );
    assert_eq!(
        terminal.projected_position_size_label("BTC", 2.0, -0.5, PositionNumberMode::Compact),
        "1.5"
    );
}

#[test]
fn projected_size_label_marks_flat_and_flipped_positions() {
    let terminal = crate::app_state::TradingTerminal::boot().0;

    assert_eq!(
        terminal.projected_position_size_label("BTC", 1.0, -1.0, PositionNumberMode::Compact),
        "0"
    );
    // An oversized opposite-side order reverses the position; magnitude alone
    // would render "1" for both sides of the flip.
    assert_eq!(
        terminal.projected_position_size_label("BTC", 1.0, -2.0, PositionNumberMode::Compact),
        "1 (Short)"
    );
    assert_eq!(
        terminal.projected_position_size_label("BTC", -1.0, 3.0, PositionNumberMode::Compact),
        "2 (Long)"
    );
}
