use super::fixtures::terminal_with_move_order;
use crate::api::MarketType;
use crate::order_execution::MoveOrderKey;
use crate::order_pending_indicators::PendingOrderIndicatorKind;

fn terminal_with_spot_alias() -> crate::app_state::TradingTerminal {
    let mut terminal = terminal_with_move_order("@0", "PURR/USDC", 100.0);
    let symbol = &mut terminal.exchange_symbols[0];
    symbol.key = "PURR/USDC".to_string();
    symbol.ticker = "PURR".to_string();
    symbol.display_name = Some("PURR/USDC".to_string());
    symbol.category = "spot".to_string();
    symbol.asset_index = 10_000;
    symbol.market_type = MarketType::Spot;
    terminal
}

#[test]
fn handle_move_order_preserves_raw_indicator_values_and_account_order() {
    for is_spot in [false, true] {
        for (side, is_buy) in [("B", true), ("A", false)] {
            let (mut terminal, coin) = if is_spot {
                (terminal_with_spot_alias(), "@0")
            } else {
                (terminal_with_move_order("BTC", "BTC", 100.0), "BTC")
            };
            let data = terminal.account_data.as_mut().expect("account fixture");
            data.completeness.spot_balances_complete = true;
            let order = &mut data.open_orders[0];
            order.side = side.to_string();
            order.sz = " 0.2500 ".to_string();
            order.limit_px = " 100.0000 ".to_string();
            order.reduce_only = if is_spot { None } else { Some(true) };
            order.is_trigger = Some(false);
            order.trigger_px = Some("0.0".to_string());
            order.order_type = Some("lImIt".to_string());
            order.tif = Some("gTc".to_string());
            let balances_revision = terminal.spot_balances_revision;

            let task = terminal.handle_move_order(coin.to_string(), 42, 101.0);

            assert_eq!(task.units(), 1);
            let label = if is_spot { "PURR/USDC" } else { "BTC" };
            assert_eq!(
                terminal.order_status,
                Some((format!("Moving {label} order to $101..."), false))
            );
            assert_eq!(terminal.pending_move_order_contexts.len(), 1);
            assert!(
                terminal
                    .pending_move_order_contexts
                    .contains_key(&MoveOrderKey::new(coin, 42))
            );
            assert_eq!(terminal.pending_order_indicators.len(), 1);
            let indicator = terminal
                .pending_order_indicators
                .values()
                .next()
                .expect("pending modification");
            assert_eq!(
                Some(indicator.account_address.as_str()),
                terminal.connected_address.as_deref()
            );
            assert_eq!(indicator.symbol, coin);
            assert_eq!(indicator.oid, Some(42));
            assert_eq!(indicator.is_buy, is_buy);
            assert_eq!(indicator.size, " 0.2500 ");
            assert_eq!(indicator.price, "101");
            assert_eq!(indicator.kind, PendingOrderIndicatorKind::Modifying);

            let data = terminal.account_data.as_ref().expect("retained account");
            assert_eq!(data.open_orders.len(), 1);
            let order = &data.open_orders[0];
            assert_eq!(order.coin, coin);
            assert_eq!(order.side, side);
            assert_eq!(order.sz, " 0.2500 ");
            assert_eq!(order.limit_px, " 100.0000 ");
            assert_eq!(order.oid, 42);
            assert_eq!(order.timestamp, 1);
            assert_eq!(order.reduce_only, if is_spot { None } else { Some(true) });
            assert_eq!(order.is_trigger, Some(false));
            assert_eq!(order.trigger_px.as_deref(), Some("0.0"));
            assert_eq!(order.order_type.as_deref(), Some("lImIt"));
            assert_eq!(order.tif.as_deref(), Some("gTc"));
            assert_eq!(data.completeness.spot_balances_complete, !is_spot);
            assert_eq!(
                terminal.spot_balances_revision,
                balances_revision.wrapping_add(u64::from(is_spot))
            );
        }
    }
}

#[test]
fn handle_move_order_rounded_noop_keeps_status_and_balances_without_mid() {
    for is_spot in [false, true] {
        let (mut terminal, coin, price) = if is_spot {
            (terminal_with_spot_alias(), "@0", 100.00001)
        } else {
            (
                terminal_with_move_order("BTC", "BTC", 100.0),
                "BTC",
                100.001,
            )
        };
        terminal.order_status = Some(("Previous status".to_string(), true));
        terminal.all_mids.clear();
        terminal.all_mids_updated_at_ms.clear();
        terminal
            .account_data
            .as_mut()
            .expect("account fixture")
            .completeness
            .spot_balances_complete = true;
        let balances_revision = terminal.spot_balances_revision;

        let task = terminal.handle_move_order(coin.to_string(), 42, price);

        assert_eq!(task.units(), 0);
        assert_eq!(
            terminal.order_status,
            Some(("Previous status".to_string(), true))
        );
        assert!(terminal.pending_order_indicators.is_empty());
        assert!(terminal.pending_move_order_contexts.is_empty());
        assert_eq!(terminal.spot_balances_revision, balances_revision);
        let data = terminal.account_data.as_ref().expect("retained account");
        assert!(data.completeness.spot_balances_complete);
        assert_eq!(data.open_orders[0].limit_px, "100");
    }
}
