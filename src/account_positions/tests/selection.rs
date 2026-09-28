use super::*;

#[test]
fn spot_pair_selection_preserves_sorted_precedence_and_tie_rules() {
    for (case, expected_coin, expected_mark) in [
        ("reconciled", "@10", Some(10.0)),
        ("latest tie", "@30", Some(30.0)),
        ("live mark", "@20", Some(20.0)),
        ("fallback", "@10", None),
        ("reconciled duplicate", "@30", Some(30.0)),
        ("latest", "@20", Some(20.0)),
    ] {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![
            spot_symbol("@20", "ubtc", 10_020),
            spot_symbol("@30", "UBTC", 10_020),
            spot_symbol("@10", "UBTC", 10_010),
            spot_symbol_with_display("@1", "UBTC", 10_001, "UBTC/UETH"),
            ExchangeSymbol {
                market_type: MarketType::Perp,
                ..spot_symbol("UBTC", "UBTC", 0)
            },
            spot_symbol("@0", "OTHER", 10_000),
        ];
        if case != "fallback" {
            for (coin, mark) in [("@10", 10.0), ("@20", 20.0), ("@30", 30.0)] {
                if case != "live mark" || coin != "@10" {
                    set_mid(&mut terminal, coin, mark);
                }
            }
        }
        set_mid(&mut terminal, "@1", 999.0);
        let mut fills = match case {
            "reconciled" => vec![
                spot_fill("@20", "100", "2", "0", "USDC", 2),
                spot_fill("@10", "90", "2", "0", "USDC", 1),
            ],
            "latest tie" => ["@30", "@20", "@10"]
                .into_iter()
                .map(|coin| spot_fill(coin, "100", "1", "0", "USDC", 50))
                .collect(),
            "reconciled duplicate" => vec![spot_fill("@30", "100", "2", "0", "USDC", 1)],
            "latest" => vec![
                spot_fill("@20", "100", "1", "0", "USDC", 60),
                spot_fill("@10", "100", "1", "0", "USDC", 50),
            ],
            _ => Vec::new(),
        };
        // A reconciled, recent non-USD pair must never enter USD valuation.
        fills.push(spot_fill("@1", "999", "2", "0", "USDC", 100));
        let balance = spot_balance("UBTC", "2", "100");

        let position = terminal
            .spot_asset_position_for_balance(&balance, &fills)
            .expect("projected spot position");

        assert_eq!(position.position.coin, expected_coin, "{case}");
        assert_eq!(
            terminal.spot_balance_mark_price(&balance, &fills),
            expected_mark,
            "{case}"
        );
        assert_wire_close(&position.position.entry_px, 50.0);
        if let Some(mark) = expected_mark {
            assert_wire_close(&position.position.position_value, 2.0 * mark);
            assert_wire_close(&position.position.unrealized_pnl, 2.0 * mark - 100.0);
        } else {
            assert_wire_close(&position.position.position_value, 100.0);
            assert_eq!(position.position.unrealized_pnl, "");
        }
    }
}
