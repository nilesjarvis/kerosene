use super::*;
use crate::account::AccountAbstractionMode;
use crate::config::MarketUniverseConfig;

#[test]
fn projected_positions_preserve_native_rows_synthesis_order_and_account_scope() {
    for hip3 in [false, true] {
        for portfolio_margin in [false, true] {
            let mut terminal = TradingTerminal::boot().0;
            if hip3 {
                terminal.market_universe = MarketUniverseConfig::hip3_dex("xyz");
            }
            terminal.exchange_symbols = vec![spot_symbol("@107", "HYPE", 10_107)];
            set_mid(&mut terminal, "@107", 3.0);
            set_mid(&mut terminal, "#950", 0.6);
            let mut data = account_data(vec![
                spot_balance("HYPE", "2", "4"),
                spot_balance("+950", "30", "12"),
                spot_balance("HYPE", "0", "0"),
                spot_balance("+951", "NaN", "0"),
                spot_balance("USDC", "10", "0"),
            ]);
            data.clearinghouse.asset_positions = [
                ("BTC", "-1", "100"),
                ("@107", "7", "2"),
                ("BAD", "NaN", "invalid"),
            ]
            .into_iter()
            .map(|(coin, size, price)| {
                serde_json::from_value(serde_json::json!({"position": {
                    "coin": coin, "szi": size, "entryPx": price, "positionValue": "retained",
                    "unrealizedPnl": "-2", "leverage": {"type": "cross", "value": 5}
                }}))
                .expect("native position fixture")
            })
            .collect();
            if portfolio_margin {
                data.account_abstraction = AccountAbstractionMode::PortfolioMargin;
                data.spot.portfolio_margin_enabled = true;
            }
            set_connected_account_data(&mut terminal, data);
            terminal.connected_address = Some(format!(" {} ", TEST_ACCOUNT.to_ascii_uppercase()));

            let positions = terminal.account_positions_with_outcomes();

            let include_spot = !hip3 || portfolio_margin;
            let expected_coins = if include_spot {
                vec!["BTC", "@107", "BAD", "#950", "@107"]
            } else {
                vec!["BTC", "@107", "BAD", "#950"]
            };
            assert_eq!(
                positions
                    .iter()
                    .map(|ap| ap.position.coin.as_str())
                    .collect::<Vec<_>>(),
                expected_coins
            );
            for (position, (size, price)) in
                positions
                    .iter()
                    .zip([("-1", "100"), ("7", "2"), ("NaN", "invalid")])
            {
                assert_eq!(position.position.szi, size);
                assert_eq!(position.position.entry_px, price);
                assert_eq!(position.position.position_value, "retained");
                assert_eq!(position.position.leverage.value, 5);
            }
            assert_eq!(positions[3].position.leverage.leverage_type, "outcome");
            assert_wire_close(&positions[3].position.unrealized_pnl, 6.0);
            if include_spot {
                assert_eq!(positions[4].position.leverage.leverage_type, "spot");
                assert_wire_close(&positions[4].position.unrealized_pnl, 2.0);
            }

            terminal.connected_address = Some("another-account".to_string());
            assert!(terminal.account_positions_with_outcomes().is_empty());
            terminal.connected_address = None;
            assert!(terminal.account_positions_with_outcomes().is_empty());
        }
    }
}
