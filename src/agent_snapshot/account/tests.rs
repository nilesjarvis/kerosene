use super::super::MAX_TOOL_ACTIVITY_ROWS;
use super::*;
use std::collections::HashMap;

#[test]
fn public_and_private_activity_share_fields_but_keep_distinct_limits() {
    let (mut terminal, _) = TradingTerminal::boot();
    for count in [0, MAX_RECENT_ROWS + 1, MAX_TOOL_ACTIVITY_ROWS + 1] {
        terminal.account_data = Some(crate::account::AccountData {
            fetch_scope: Default::default(),
            request_weight_estimate: 0,
            account_abstraction: Default::default(),
            clearinghouse: crate::account::ClearinghouseState {
                margin_summary: crate::account::MarginSummary {
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
            spot: crate::account::SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders: Vec::new(),
            fills: (0..count)
                .map(|index| crate::account::UserFill {
                    coin: "BTC".to_string(),
                    px: "100".to_string(),
                    sz: "2".to_string(),
                    side: "B".to_string(),
                    start_position: Some("99".to_string()),
                    time: index as u64,
                    hash: Some("excluded-fill-hash".to_string()),
                    tid: Some(900_001),
                    oid: Some(900_002),
                    dir: "Open Long".to_string(),
                    closed_pnl: "3".to_string(),
                    fee: "0.1".to_string(),
                    fee_token: None,
                })
                .collect(),
            funding_history: (0..count)
                .map(|index| crate::account::FundingEntry {
                    delta: crate::account::FundingDelta {
                        coin: "BTC".to_string(),
                        funding_rate: "0.01".to_string(),
                        szi: "2".to_string(),
                        usdc: "-0.2".to_string(),
                    },
                    time: index as u64,
                })
                .collect(),
            fee_rates: Default::default(),
            completeness: Default::default(),
            fetched_at_ms: 123,
        });
        let value: Value =
            serde_json::from_slice(&terminal.build_agent_snapshot().expect("snapshot"))
                .expect("snapshot json");
        let account = &value["account"];
        let activity = &value["_tool_data"]["activity"];
        for (public_key, private_key) in [("recent_fills", "fills"), ("recent_funding", "funding")]
        {
            let public = account[public_key].as_array().expect("public activity");
            let private = activity[private_key].as_array().expect("private activity");
            assert_eq!(public.len(), count.min(MAX_RECENT_ROWS));
            assert_eq!(private.len(), count.min(MAX_TOOL_ACTIVITY_ROWS));
            assert_eq!(public, &private[..public.len()]);
            if let Some(last) = private.last() {
                assert_eq!(last["time_ms"], private.len() - 1);
            }
            assert_eq!(account["coverage"][public_key]["total_count"], count);
            assert_eq!(
                account["coverage"][public_key]["truncated"],
                count > MAX_RECENT_ROWS
            );
            assert_eq!(activity["coverage"][private_key]["total_count"], count);
            assert_eq!(
                activity["coverage"][private_key]["truncated"],
                count > MAX_TOOL_ACTIVITY_ROWS
            );
        }
        if count > 0 {
            assert_eq!(
                activity["fills"][0],
                json!({
                    "coin": "BTC", "price": "100", "size": "2", "side": "B",
                    "direction": "Open Long", "time_ms": 0, "closed_pnl": "3",
                    "fee": "0.1", "fee_token": null,
                })
            );
            assert_eq!(
                activity["funding"][0],
                json!({
                    "coin": "BTC", "funding_rate": "0.01", "position_size": "2",
                    "usdc": "-0.2", "time_ms": 0,
                })
            );
        }
    }
}
