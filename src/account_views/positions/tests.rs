use super::*;

#[test]
fn position_columns_hide_one_group_at_a_time_as_width_shrinks() {
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_TOTAL_PNL_BELOW),
        PositionColumnVisibility {
            entry: true,
            liquidation: true,
            mark: true,
            funding: true,
            total_pnl: true,
            leverage: true,
        }
    );
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_TOTAL_PNL_BELOW - 1.0),
        PositionColumnVisibility {
            entry: true,
            liquidation: true,
            mark: true,
            funding: true,
            total_pnl: false,
            leverage: true,
        }
    );
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_LEVERAGE_BELOW - 1.0),
        PositionColumnVisibility {
            entry: true,
            liquidation: true,
            mark: true,
            funding: true,
            total_pnl: false,
            leverage: false,
        }
    );
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_FUNDING_BELOW - 1.0),
        PositionColumnVisibility {
            entry: true,
            liquidation: true,
            mark: true,
            funding: false,
            total_pnl: false,
            leverage: false,
        }
    );
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_LIQUIDATION_BELOW - 1.0),
        PositionColumnVisibility {
            entry: true,
            liquidation: false,
            mark: true,
            funding: false,
            total_pnl: false,
            leverage: false,
        }
    );
    // Narrow panes drop the fixed Entry then Mark columns so the essentials and
    // the close/NUKE action cell keep fitting on-screen.
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_ENTRY_BELOW - 1.0),
        PositionColumnVisibility {
            entry: false,
            liquidation: false,
            mark: true,
            funding: false,
            total_pnl: false,
            leverage: false,
        }
    );
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_MARK_BELOW - 1.0),
        PositionColumnVisibility {
            entry: false,
            liquidation: false,
            mark: false,
            funding: false,
            total_pnl: false,
            leverage: false,
        }
    );
}

/// Width consumed by the fixed-width columns, the close/NUKE action cell, the
/// inter-column spacing and the row's horizontal padding for a given pane
/// width. Mirrors the layout built in `header.rs` / `position_row.rs`: a row
/// with `.spacing(4)` inside a container with `.padding([_, 8])`, where the
/// remaining width is shared by the `Fill` columns (Symbol, Size, Value, uPnL
/// and — when visible — Total PnL).
fn fixed_layout_budget(width: f32) -> f32 {
    let columns = PositionColumnVisibility::for_width(width);
    // Always present: Side + action cell are fixed; Symbol, Size, Value and uPnL
    // are Fill but still count as children for spacing purposes.
    let mut fixed = POSITION_SIDE_WIDTH + POSITION_ACTION_WIDTH;
    let mut children = 6u32; // Symbol, Side, Size, Value, uPnL, action
    if columns.entry {
        fixed += POSITION_ENTRY_WIDTH;
        children += 1;
    }
    if columns.liquidation {
        fixed += POSITION_LIQ_WIDTH;
        children += 1;
    }
    if columns.mark {
        fixed += POSITION_MARK_WIDTH;
        children += 1;
    }
    if columns.funding {
        fixed += POSITION_FUNDING_WIDTH;
        children += 1;
    }
    if columns.total_pnl {
        children += 1; // Total PnL is a Fill column
    }
    if columns.leverage {
        fixed += POSITION_LEVERAGE_WIDTH;
        children += 1;
    }
    fixed + ROW_SPACING * (children.saturating_sub(1) as f32) + ROW_HORIZONTAL_PADDING
}

/// Number of `Fill` columns (Symbol, Size, Value, uPnL, and Total PnL when
/// visible) that share the leftover width at a given pane width.
fn fill_column_count(width: f32) -> u32 {
    if PositionColumnVisibility::for_width(width).total_pnl {
        5
    } else {
        4
    }
}

/// Width each `Fill` column receives after the fixed columns, action cell,
/// spacing and padding are subtracted and the remainder is split equally.
fn fill_share(width: f32) -> f32 {
    (width - fixed_layout_budget(width)) / fill_column_count(width) as f32
}

#[test]
fn fill_columns_stay_at_least_min_width_in_full_mode() {
    // In Full (non-abbreviated) mode every `Fill` column must keep at least
    // MIN_FILL_WIDTH of the shared slack, so revealing an optional column never
    // squeezes ordinary numbers below their natural width and clips them. Below
    // COMPACT_NUMBERS_BELOW numbers are abbreviated and fit a narrower share.
    let mut width = COMPACT_NUMBERS_BELOW;
    while width <= 1_600.0 {
        let share = fill_share(width);
        assert!(
            share >= MIN_FILL_WIDTH,
            "fill share {share} < MIN_FILL_WIDTH {MIN_FILL_WIDTH} at width {width}",
        );
        width += 1.0;
    }
}

#[test]
fn fixed_columns_never_overflow_at_realistic_pane_widths() {
    // Regression guard: across every realistic pane width the fixed columns plus
    // the close/NUKE action cell must fit, leaving the Fill columns a width >= 0.
    // If they didn't, the action cell would overflow past the right edge and be
    // clipped (the table only scrolls vertically), making close/NUKE unreachable.
    // Below ~260px a position row can't fit even the essentials, same as before
    // the fixed-width work, so the sweep starts there.
    let mut width = 260.0_f32;
    while width <= 1_600.0 {
        let budget = fixed_layout_budget(width);
        assert!(
            budget <= width,
            "fixed-column budget {budget} exceeds available width {width}",
        );
        width += 1.0;
    }
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
