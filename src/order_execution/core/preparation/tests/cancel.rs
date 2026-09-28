use super::*;

#[test]
fn prepare_cancel_order_accepts_api_named_spot_pair_and_legacy_indexed_key() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![purr_spot_symbol()];

    // Open orders report the API coin name ("PURR/USDC"), which must
    // resolve so resting spot orders can be canceled from the app.
    let prepared = terminal
        .prepare_cancel_order(CancelIntent {
            surface: OrderSurface::Cancel,
            symbol_key: "PURR/USDC".to_string(),
            oid: 42,
        })
        .expect("cancel by API coin name");
    assert_eq!(prepared.symbol_key, "PURR/USDC");
    assert_eq!(prepared.asset, 10_000);
    assert_eq!(prepared.market_type, MarketType::Spot);

    // State saved before the pair was re-keyed may still send "@0".
    let prepared = terminal
        .prepare_cancel_order(CancelIntent {
            surface: OrderSurface::Cancel,
            symbol_key: "@0".to_string(),
            oid: 42,
        })
        .expect("cancel by legacy indexed key");
    assert_eq!(prepared.symbol_key, "PURR/USDC");
    assert_eq!(prepared.asset, 10_000);
}

#[test]
fn prepare_cancel_order_derives_indexed_spot_asset_without_metadata() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols.clear();

    let prepared = terminal
        .prepare_cancel_order(CancelIntent {
            surface: OrderSurface::Cancel,
            symbol_key: "@107".to_string(),
            oid: 42,
        })
        .expect("indexed spot cancellation should not depend on metadata");

    assert_eq!(prepared.symbol_key, "@107");
    assert_eq!(prepared.asset, 10_107);
    assert_eq!(prepared.oid, 42);
    assert_eq!(prepared.market_type, MarketType::Spot);
}

#[test]
fn prepare_cancel_order_recovers_canonical_purr_without_metadata() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols.clear();

    let prepared = terminal
        .prepare_cancel_order(CancelIntent {
            surface: OrderSurface::Cancel,
            symbol_key: "PURR/USDC".to_string(),
            oid: 42,
        })
        .expect("canonical PURR cancellation should not depend on metadata");

    assert_eq!(prepared.symbol_key, "PURR/USDC");
    assert_eq!(prepared.asset, 10_000);
    assert_eq!(prepared.oid, 42);
    assert_eq!(prepared.market_type, MarketType::Spot);
}

#[test]
fn prepare_cancel_order_rejects_noncanonical_or_overflowing_spot_keys() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols.clear();

    for key in [
        "@",
        "@-1",
        "@+1",
        "@01",
        "@1 ",
        "@1x",
        "@@1",
        "@4294967295",
        "HYPE/USDC",
        "purr/USDC",
    ] {
        let error = terminal
            .prepare_cancel_order(CancelIntent {
                surface: OrderSurface::Cancel,
                symbol_key: key.to_string(),
                oid: 42,
            })
            .expect_err("invalid metadata-free key must fail closed");

        assert_eq!(error, format!("Symbol '{key}' not found"));
    }
}

#[test]
fn non_usd_quoted_spot_can_be_cancelled_but_not_modified() {
    let (mut terminal, _) = TradingTerminal::boot();
    let mut spot = symbol("@55", MarketType::Spot);
    spot.ticker = "UETH".to_string();
    spot.display_name = Some("UETH/UBTC".to_string());
    spot.asset_index = 10_055;
    spot.collateral_token = Some(221);
    terminal.exchange_symbols = vec![spot];
    terminal.all_mids.insert("@55".to_string(), 0.05);
    terminal
        .all_mids_updated_at_ms
        .insert("@55".to_string(), TradingTerminal::now_ms());

    let modify_error = terminal
        .prepare_modify_order(ModifyIntent {
            surface: OrderSurface::Move,
            symbol_key: "@55",
            oid: 42,
            is_buy: true,
            new_price: 0.051,
            original_price: "0.05",
            size: "1",
            invalid_size_message: "Invalid size",
            reduce_only: None,
            reduce_only_missing_message: "Missing reduce-only",
            invalid_price_message: "Invalid price",
        })
        .expect_err("unverified quote accounting must block repricing");
    assert!(modify_error.contains("quote-token USD valuation and accounting"));

    let cancel = terminal
        .prepare_cancel_order(CancelIntent {
            surface: OrderSurface::Cancel,
            symbol_key: "@55".to_string(),
            oid: 42,
        })
        .expect("safety cancellation must remain available");
    assert_eq!(cancel.market_type, MarketType::Spot);
}

#[test]
fn prepare_cancel_order_allows_existing_outcome_orders() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![outcome_symbol("#650")];

    let prepared = terminal
        .prepare_cancel_order(CancelIntent {
            surface: OrderSurface::Cancel,
            symbol_key: "#650".to_string(),
            oid: 42,
        })
        .expect("cancel should be prepared");

    assert_eq!(prepared.symbol_key, "#650");
    assert_eq!(prepared.asset, 7);
    assert_eq!(prepared.oid, 42);
    assert_eq!(prepared.market_type, MarketType::Outcome);
}

#[test]
fn missing_outcome_metadata_only_allows_cancelling_canonical_side_assets() {
    let terminal = TradingTerminal::boot().0;
    for (key, asset) in [("#650", 100_000_650), ("#651", 100_000_651)] {
        let cancel = terminal
            .prepare_cancel_order(CancelIntent {
                surface: OrderSurface::Cancel,
                symbol_key: key.into(),
                oid: 42,
            })
            .expect("cancel without expired metadata");
        assert_eq!(cancel.asset, asset);
        assert_eq!(cancel.market_type, MarketType::Outcome);
        assert!(
            terminal
                .prepare_place_order(ticket_limit_intent(key))
                .is_err()
        );
        assert!(
            terminal
                .prepare_modify_order(move_modify_intent(key))
                .is_err()
        );
    }
    for key in [
        "#",
        "#0650",
        "#652",
        "#-650",
        "#+650",
        "#650 ",
        "+650",
        "#4294967290",
        "#42949672950",
    ] {
        assert!(
            terminal
                .prepare_cancel_order(CancelIntent {
                    surface: OrderSurface::Cancel,
                    symbol_key: key.into(),
                    oid: 42,
                })
                .is_err(),
            "{key}"
        );
    }
}

#[test]
fn unsafe_outcome_terms_block_ticket_preset_and_modify_but_preserve_cancellation() {
    for state in [
        "expired",
        "unverified",
        "unsupported",
        "settled",
        "unknown quote",
    ] {
        let mut terminal = TradingTerminal::boot().0;
        let mut symbol = outcome_symbol("#650");
        symbol.asset_index = 100_000_650;
        let info = symbol.outcome.as_mut().expect("terms");
        let expected_reason = match state {
            "expired" => {
                info.contract.deadline_ms = Some(1);
                "Expired"
            }
            "unverified" => {
                info.contract.verified = false;
                "unverified"
            }
            "unsupported" => {
                info.contract.blocked_reason = Some("Contract template is unavailable".into());
                "template is unavailable"
            }
            "settled" => {
                info.question_settled_named_outcomes = vec![65];
                "Settled"
            }
            "unknown quote" => {
                info.quote_token_index = None;
                "Unsupported outcome quote token"
            }
            _ => unreachable!(),
        };
        terminal.exchange_symbols = vec![symbol];
        // Keep the frame clock behind expiry to prove submission rechecks
        // the actual time even before the UI has rendered its next frame.
        terminal.status_bar_now_ms = 0;
        for surface in [OrderSurface::Ticket, OrderSurface::Preset] {
            let mut intent = ticket_limit_intent("#650");
            intent.surface = surface;
            let error = terminal
                .prepare_place_order(intent)
                .expect_err("unsafe placement rejected");
            assert!(error.contains(expected_reason), "{state}: {error}");
        }
        let result = terminal.prepare_modify_order(move_modify_intent("#650"));
        assert!(
            matches!(result, Err(error) if error.contains(expected_reason)),
            "{state}"
        );
        let cancel = terminal
            .prepare_cancel_order(CancelIntent {
                surface: OrderSurface::Cancel,
                symbol_key: "#650".into(),
                oid: 42,
            })
            .expect("cancel remains available");
        assert_eq!(cancel.asset, 100_000_650);
        assert_eq!(cancel.oid, 42);
    }
}
