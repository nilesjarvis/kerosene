use super::*;
use crate::account::{position_notional_from_mark_or_wire, position_upnl_from_mark_or_wire};

#[test]
fn position_sort_columns_preserve_missing_values_ties_and_input_order() {
    let mut terminal =
        TradingTerminal::boot_from_config(crate::config::KeroseneConfig::default()).0;
    let positions: Vec<account::AssetPosition> = [
        ("B", "-2", "20", "40", "2", "10", "-2", 2, "b"),
        ("A", "1", "10", "10", "1", "5", "-1", 1, "first-a"),
        ("C", "NaN", "NaN", "NaN", "NaN", "NaN", "NaN", 0, "c"),
        ("A", "1", "10", "10", "1", "5", "-1", 1, "second-a"),
    ]
    .into_iter()
    .map(
        |(coin, size, entry, value, pnl, liq, funding, leverage, tag)| {
            serde_json::from_value(serde_json::json!({"position": {
                "coin": coin, "szi": size, "entryPx": entry, "positionValue": value,
                "unrealizedPnl": pnl, "liquidationPx": liq, "marginUsed": tag,
                "leverage": {"type": "cross", "value": leverage},
                "cumFunding": {"allTime": "0", "sinceOpen": funding, "sinceChange": "0"}
            }}))
            .expect("position fixture")
        },
    )
    .collect();
    let ascending = ["first-a", "second-a", "b", "c"];
    let descending = ["c", "b", "first-a", "second-a"];
    for (column, expected_ascending, expected_descending) in [
        (PositionsSortColumn::Symbol, ascending, descending),
        (PositionsSortColumn::Side, descending, ascending),
        (PositionsSortColumn::Size, ascending, descending),
        (PositionsSortColumn::Entry, ascending, descending),
        (PositionsSortColumn::Liquidation, ascending, descending),
        (PositionsSortColumn::Mark, ascending, ascending),
        (PositionsSortColumn::Value, ascending, descending),
        (PositionsSortColumn::UnrealizedPnl, ascending, descending),
        (PositionsSortColumn::Funding, ascending, descending),
        (PositionsSortColumn::SpentFees, ascending, ascending),
        (PositionsSortColumn::TotalPnl, ascending, descending),
        (
            PositionsSortColumn::Leverage,
            ["c", "first-a", "second-a", "b"],
            ["b", "first-a", "second-a", "c"],
        ),
    ] {
        terminal.positions_sort_column = column;
        for (direction, expected) in [
            (config::SortDirection::Ascending, expected_ascending),
            (config::SortDirection::Descending, expected_descending),
        ] {
            terminal.positions_sort_direction = direction;
            let rows = terminal.sorted_position_rows(&positions);
            assert_eq!(
                rows.iter()
                    .map(|row| row.ap.position.margin_used.as_str())
                    .collect::<Vec<_>>(),
                expected,
                "{column:?} {direction:?}"
            );
        }
    }
    assert_eq!(
        positions
            .iter()
            .map(|ap| ap.position.margin_used.as_str())
            .collect::<Vec<_>>(),
        ["b", "first-a", "c", "second-a"]
    );
}

#[test]
fn position_row_number_parser_rejects_invalid_or_nonfinite_values() {
    assert_eq!(parse_position_row_number(" 2.5 "), Some(2.5));
    assert_eq!(parse_position_row_number("-0.125"), Some(-0.125));

    assert_eq!(parse_position_row_number("not-a-number"), None);
    assert_eq!(parse_position_row_number("NaN"), None);
    assert_eq!(parse_position_row_number("inf"), None);
}

#[test]
fn position_row_value_prefers_live_mid_only_when_inputs_are_valid() {
    assert_eq!(
        position_notional_from_mark_or_wire(Some(-2.0), Some(999.0), Some(100.0)),
        Some(200.0)
    );
    assert_eq!(
        position_notional_from_mark_or_wire(None, Some(999.0), Some(100.0)),
        Some(999.0)
    );
    assert_eq!(
        position_notional_from_mark_or_wire(Some(-2.0), Some(-250.0), None),
        Some(250.0)
    );
    assert_eq!(
        position_notional_from_mark_or_wire(Some(-2.0), None, None),
        None
    );
}

#[test]
fn position_row_upnl_prefers_live_mid_only_when_inputs_are_valid() {
    assert_eq!(
        position_upnl_from_mark_or_wire(Some(2.0), Some(90.0), Some(1.0), Some(100.0)),
        Some(20.0)
    );
    assert_eq!(
        position_upnl_from_mark_or_wire(None, Some(90.0), Some(1.0), Some(100.0)),
        Some(1.0)
    );
    assert_eq!(
        position_upnl_from_mark_or_wire(Some(2.0), Some(90.0), None, None),
        None
    );
}

fn fee_account(terminal: &mut TradingTerminal) -> Vec<account::AssetPosition> {
    let positions: Vec<account::AssetPosition> = ["BTC", "ETH", "SOL"]
        .into_iter()
        .map(|coin| {
            serde_json::from_value(serde_json::json!({"position": {
                "coin": coin, "szi": "1", "entryPx": "100", "positionValue": "100",
                "unrealizedPnl": "0", "leverage": {"type": "cross", "value": 10}
            }}))
            .expect("position fixture")
        })
        .collect();
    let fills = [("BTC", "1"), ("ETH", "3")]
        .into_iter()
        .enumerate()
        .map(|(index, (coin, fee))| {
            serde_json::from_value(serde_json::json!({
                "coin": coin, "sz": "1", "px": "100", "side": "B", "time": index,
                "tid": index, "startPosition": "0", "dir": "Open Long", "closedPnl": "0",
                "fee": fee, "feeToken": "USDC"
            }))
            .expect("fill fixture")
        })
        .collect();
    let data = account::AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: account::ClearinghouseState {
            margin_summary: account::MarginSummary {
                account_value: "0".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "0".to_string(),
            asset_positions: positions.clone(),
        },
        clearinghouses_by_dex: Default::default(),
        spot: account::SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills,
        funding_history: Vec::new(),
        fee_rates: Default::default(),
        completeness: Default::default(),
        fetched_at_ms: 1000,
    };
    let address = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some(address.to_string());
    terminal.set_account_data_for_address_for_test(address, data);
    positions
}

#[test]
fn spent_fees_sort_numerically_with_missing_values_last_in_both_directions() {
    let (mut terminal, _) = TradingTerminal::boot();
    let positions = fee_account(&mut terminal);
    terminal.positions_sort_column = PositionsSortColumn::SpentFees;
    terminal.positions_sort_direction = PositionsSortColumn::SpentFees.default_direction();
    let rows = terminal.sorted_position_rows(&positions);
    assert_eq!(
        rows.iter().map(|row| row.coin).collect::<Vec<_>>(),
        ["ETH", "BTC", "SOL"]
    );
    assert_eq!(rows[0].spent_fees, Some(3.0));
    terminal.positions_sort_direction = config::SortDirection::Ascending;
    assert_eq!(
        terminal
            .sorted_position_rows(&positions)
            .iter()
            .map(|row| row.coin)
            .collect::<Vec<_>>(),
        ["BTC", "ETH", "SOL"]
    );
}

#[test]
fn spent_fees_use_only_complete_matching_account_data_and_follow_live_fills() {
    let (mut terminal, _) = TradingTerminal::boot();
    let mut positions = fee_account(&mut terminal);
    assert_eq!(
        terminal.position_row_data(&positions[0]).spent_fees,
        Some(1.0)
    );
    let data = terminal.account_data.as_mut().expect("account fixture");
    let mut increase = data.fills[0].clone();
    increase.tid = Some(100);
    increase.time = 100;
    increase.start_position = Some("1".to_string());
    data.fills.push(increase);
    // Fills can arrive before the corresponding position snapshot.
    assert_eq!(terminal.position_row_data(&positions[0]).spent_fees, None);
    positions[0].position.szi = "2".to_string();
    assert_eq!(
        terminal.position_row_data(&positions[0]).spent_fees,
        Some(2.0)
    );
    terminal
        .account_data
        .as_mut()
        .expect("account fixture")
        .completeness
        .fills_complete = false;
    assert_eq!(terminal.position_row_data(&positions[0]).spent_fees, None);
    terminal
        .account_data
        .as_mut()
        .expect("account fixture")
        .completeness
        .fills_complete = true;
    terminal
        .account_data
        .as_mut()
        .expect("account fixture")
        .completeness
        .positions_complete = false;
    assert_eq!(terminal.position_row_data(&positions[0]).spent_fees, None);
    terminal
        .account_data
        .as_mut()
        .expect("account fixture")
        .completeness
        .positions_complete = true;
    terminal.connected_address = Some("0xdef0000000000000000000000000000000000000".to_string());
    assert_eq!(terminal.position_row_data(&positions[0]).spent_fees, None);
    terminal.connected_address = None;
    assert_eq!(terminal.position_row_data(&positions[0]).spent_fees, None);
}
