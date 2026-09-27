use super::*;
use crate::app_state::TradingTerminal;
use crate::order_execution::{CancelIntent, OrderSurface, TicketOrderPlaceIntent};
use crate::signing::ExchangeOrderKind;

// Public mainnet outcomeMeta rows captured 2026-09-17. No Skew API, account,
// credentials, or live orders are involved in these tests.
fn skew_symbols() -> Vec<ExchangeSymbol> {
    let metadata = serde_json::from_str(include_str!("fixtures/skew.json"))
        .expect("valid public Skew metadata fixture");
    let mut symbols = Vec::new();
    append_outcome_symbols(&mut symbols, metadata);
    symbols
}

#[test]
fn skew_binary_templates_preserve_identity_and_render_both_conditions() {
    let symbols = skew_symbols();
    assert_eq!(symbols.len(), 8);
    let yes = symbol_by_key_or_panic(&symbols, "#28960");
    let no = symbol_by_key_or_panic(&symbols, "#28961");
    assert_eq!(yes.asset_index, 100_028_960);
    assert_eq!(no.asset_index, 100_028_961);
    assert_eq!(yes.ticker, "OUT2896-YES");
    assert_eq!(no.ticker, "OUT2896-NO");
    assert_eq!(yes.sz_decimals, 0);
    assert_eq!(yes.market_type, MarketType::Outcome);
    assert!(yes.is_user_selectable_market());
    assert_eq!(
        yes.display_name.as_deref(),
        Some("Skew | YES: BTC is above 74,950 at 2026-09-18 06:00 UTC")
    );
    assert_eq!(
        no.display_name.as_deref(),
        Some("Skew | NO: BTC is at or below 74,950 at 2026-09-18 06:00 UTC")
    );
    let info = outcome_by_key_or_panic(&symbols, "#28960");
    assert_eq!(info.venue.as_deref(), Some("skew"));
    assert_eq!(info.quote_symbol, "USDC");
    assert_eq!(info.quote_token_index, Some(crate::api::USDC_TOKEN_INDEX));
    assert_eq!(info.class.as_deref(), Some("priceBinary"));
    assert_eq!(info.expiry.as_deref(), Some("20260918-0600"));
    assert_eq!(info.target_price.as_deref(), Some("74950"));
    assert_eq!(info.side_name, "Yes");
    assert!(yes.keywords.iter().any(|keyword| keyword == "skew.trade"));
    let expiry_ms = chrono::NaiveDateTime::parse_from_str("20260918-0600", "%Y%m%d-%H%M")
        .expect("valid expiry")
        .and_utc()
        .timestamp_millis() as u64;
    assert_eq!(
        info.time_left_label(expiry_ms - 3_600_000).as_deref(),
        Some("1h")
    );
    assert_eq!(info.time_left_label(expiry_ms).as_deref(), Some("expired"));
}

#[test]
fn skew_index_underlyings_and_settlement_sources_are_not_perp_mark_prices() {
    let symbols = skew_symbols();
    let nasdaq = outcome_by_key_or_panic(&symbols, "#29040");
    assert_eq!(nasdaq.underlying.as_deref(), Some("xyz:XYZ100"));
    assert_eq!(
        nasdaq.side_condition_short_label(),
        "xyz:XYZ100 is above 28,630"
    );
    assert_eq!(
        nasdaq.settlement_source_label().as_deref(),
        Some("Settlement: the Pyth US100 index price | 90s window ending at expiry")
    );
    assert_eq!(
        outcome_by_key_or_panic(&symbols, "#29100")
            .underlying
            .as_deref(),
        Some("xyz:SP500")
    );
    assert_eq!(
        outcome_by_key_or_panic(&symbols, "#37700")
            .settlement_source_label()
            .as_deref(),
        Some("Settlement: the Hyperliquid BTC perp trade | 90s window ending at expiry")
    );
}

#[test]
fn skew_venue_round_trips_and_legacy_cache_without_venue_still_loads() {
    let symbols = skew_symbols();
    let original = symbol_by_key_or_panic(&symbols, "#28960");
    let mut value = serde_json::to_value(original).expect("serialize symbol");
    let decoded: ExchangeSymbol =
        serde_json::from_value(value.clone()).expect("deserialize symbol");
    let mut cached_expected = original.clone();
    cached_expected
        .outcome
        .as_mut()
        .expect("outcome")
        .contract
        .verified = false;
    assert_eq!(decoded, cached_expected);
    assert!(
        decoded
            .outcome
            .as_ref()
            .expect("outcome")
            .trading_block_reason(0)
            .is_some()
    );
    value["outcome"]
        .as_object_mut()
        .expect("outcome object")
        .remove("venue");
    value["outcome"]
        .as_object_mut()
        .expect("outcome object")
        .remove("contract");
    let legacy: ExchangeSymbol = serde_json::from_value(value).expect("old cache still readable");
    let legacy_info = legacy.outcome.expect("outcome metadata");
    assert!(legacy_info.venue.is_none());
    assert!(!legacy_info.contract.verified);
}

#[test]
fn skew_metadata_routes_buy_sell_and_cancel_to_exact_side_assets() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = skew_symbols();
    for symbol in &mut terminal.exchange_symbols {
        symbol
            .outcome
            .as_mut()
            .expect("outcome")
            .contract
            .deadline_ms = Some(TradingTerminal::now_ms() + 60_000);
    }
    for (key, asset) in [("#28960", 100_028_960), ("#28961", 100_028_961)] {
        terminal.all_mids.insert(key.to_string(), 0.42);
        terminal
            .all_mids_updated_at_ms
            .insert(key.to_string(), TradingTerminal::now_ms());
        for is_buy in [true, false] {
            let intent = TradingTerminal::ticket_order_place_intent(TicketOrderPlaceIntent {
                surface: OrderSurface::Ticket,
                symbol_key: key.to_string(),
                is_buy,
                order_kind: ExchangeOrderKind::Limit,
                price_input: "0.42123456".to_string(),
                quantity_input: "30".to_string(),
                quantity_is_usd: true,
                reduce_only: true,
            });
            let prepared = terminal
                .prepare_place_order(intent)
                .expect("prepare Skew order");
            assert_eq!(prepared.asset, asset);
            assert_eq!(prepared.symbol_key, key);
            assert_eq!(prepared.is_buy, is_buy);
            assert_eq!(prepared.market_type, MarketType::Outcome);
            assert_eq!(prepared.price, "0.42123");
            assert_eq!(prepared.size, "30");
            assert!(!prepared.reduce_only);
        }
        let cancel = terminal
            .prepare_cancel_order(CancelIntent {
                surface: OrderSurface::Cancel,
                symbol_key: key.to_string(),
                oid: 42,
            })
            .expect("prepare Skew cancellation");
        assert_eq!(cancel.asset, asset);
        assert_eq!(cancel.oid, 42);
    }
}

#[test]
fn binary_template_aliases_do_not_reinterpret_other_templates() {
    let mut metadata: OutcomeMetaResponse =
        serde_json::from_str(include_str!("fixtures/skew.json")).expect("valid metadata");
    metadata.outcomes.truncate(1);
    metadata.outcomes[0].name = "template:priceTouch".to_string();
    let mut symbols = Vec::new();
    append_outcome_symbols(&mut symbols, metadata);
    let info = outcome_by_key_or_panic(&symbols, "#28960");
    assert!(info.class.is_none());
    assert!(info.target_price.is_none());
    assert!(info.settlement_source_label().is_none());
}

#[test]
fn skew_orders_keep_outcome_price_size_and_exact_mid_validation() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = skew_symbols();
    // Exercise price and size validation independently of the captured expiry.
    for symbol in &mut terminal.exchange_symbols {
        symbol
            .outcome
            .as_mut()
            .expect("outcome")
            .contract
            .deadline_ms = Some(TradingTerminal::now_ms() + 60_000);
    }
    // An underlying BTC mid must never price an outcome order.
    terminal.all_mids.insert("BTC".to_string(), 76_000.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());
    for (kind, price, quantity, expected_error) in [
        (
            ExchangeOrderKind::Limit,
            "1.1",
            "30",
            "Outcome prices must be between",
        ),
        (ExchangeOrderKind::Limit, "0.42", "30.5", "whole-contract"),
        (ExchangeOrderKind::Market, "", "30", "No mid price"),
    ] {
        let intent = TradingTerminal::ticket_order_place_intent(TicketOrderPlaceIntent {
            surface: OrderSurface::Ticket,
            symbol_key: "#28960".to_string(),
            is_buy: true,
            order_kind: kind,
            price_input: price.to_string(),
            quantity_input: quantity.to_string(),
            quantity_is_usd: false,
            reduce_only: false,
        });
        let error = terminal
            .prepare_place_order(intent)
            .expect_err("invalid Skew order");
        assert!(
            error.contains(expected_error),
            "unexpected validation: {error}"
        );
    }
}

#[test]
fn skew_settlement_details_require_a_valid_published_source_and_window() {
    let symbols = skew_symbols();
    let mut info = outcome_by_key_or_panic(&symbols, "#28960").clone();
    for description in [
        "priceDescription:Pyth|seconds:0",
        "priceDescription:Pyth|seconds:invalid",
        "priceDescription:|seconds:90",
        "seconds:90",
    ] {
        info.description = description.to_string();
        assert!(info.settlement_source_label().is_none());
    }
}
