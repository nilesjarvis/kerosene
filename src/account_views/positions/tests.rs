use super::*;

#[test]
fn position_columns_hide_one_group_at_a_time_as_width_shrinks() {
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_TOTAL_PNL_BELOW),
        PositionColumnVisibility {
            liquidation: true,
            funding: true,
            total_pnl: true,
            leverage: true,
        }
    );
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_TOTAL_PNL_BELOW - 1.0),
        PositionColumnVisibility {
            liquidation: true,
            funding: true,
            total_pnl: false,
            leverage: true,
        }
    );
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_LEVERAGE_BELOW - 1.0),
        PositionColumnVisibility {
            liquidation: true,
            funding: true,
            total_pnl: false,
            leverage: false,
        }
    );
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_FUNDING_BELOW - 1.0),
        PositionColumnVisibility {
            liquidation: true,
            funding: false,
            total_pnl: false,
            leverage: false,
        }
    );
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_LIQUIDATION_BELOW - 1.0),
        PositionColumnVisibility {
            liquidation: false,
            funding: false,
            total_pnl: false,
            leverage: false,
        }
    );
}

#[test]
fn position_numbers_compact_after_optional_columns_are_hidden() {
    assert_eq!(
        PositionNumberMode::for_width(COMPACT_NUMBERS_BELOW),
        PositionNumberMode::Full
    );
    assert_eq!(
        PositionNumberMode::for_width(COMPACT_NUMBERS_BELOW - 1.0),
        PositionNumberMode::Compact
    );
}

#[test]
fn opening_position_label_uses_resolved_symbol_and_size_labels() {
    let delta = ProjectedPositionDelta {
        symbol: "#950".to_string(),
        signed_size: 5.0,
        estimated_price: Some(0.42),
    };
    assert_eq!(
        opening_position_label(&delta, "YES: Will BTC close green?", "5"),
        "\u{27f3} YES: Will BTC close green? market buy 5 @ ~0.4200 in flight\u{2026}"
    );

    let sell_delta = ProjectedPositionDelta {
        symbol: "BTC".to_string(),
        signed_size: -1.5,
        estimated_price: None,
    };
    assert_eq!(
        opening_position_label(&sell_delta, "BTC", "1.5"),
        "\u{27f3} BTC market sell 1.5 in flight\u{2026}"
    );
}

#[test]
fn opening_lines_are_suppressed_for_symbols_with_any_position_even_hidden() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.optimistic_account_updates = true;
    let pending_id = terminal.add_pending_market_order_placement_indicator(
        "0xabc0000000000000000000000000000000000000".to_string(),
        "ETH".to_string(),
        true,
        "1".to_string(),
        "100".to_string(),
    );
    assert!(pending_id.is_some());

    // A position exists for the symbol (visible or user-hidden): the order
    // adds to it rather than opening a new one, so no opening line.
    let with_position = terminal.optimistic_opening_position_deltas(&["ETH".to_string()]);
    assert!(with_position.is_empty());

    let without_position = terminal.optimistic_opening_position_deltas(&["BTC".to_string()]);
    assert_eq!(without_position.len(), 1);
    assert_eq!(without_position[0].symbol, "ETH");
}
