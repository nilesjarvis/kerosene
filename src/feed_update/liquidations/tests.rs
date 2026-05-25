use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::LiquidationEvent;

#[test]
fn clear_liquidations_resets_rows_summary_and_chart_buckets() {
    let mut terminal = TradingTerminal::boot().0;
    let liquidation = LiquidationEvent {
        coin: "HYPE".to_string(),
        price: 25.0,
        size: 4.0,
        is_buy: false,
        time_ms: TradingTerminal::now_ms(),
        method: "market".to_string(),
        liquidated_user: "0x0000000000000000000000000000000000000001".to_string(),
        tx_index: 1,
    };

    let _ = terminal.update_liquidation_feed(Message::WsHydromancerLiquidation(
        crate::ws::HydromancerWsMessage::Event(liquidation),
    ));
    assert!(!terminal.liquidations.is_empty());
    assert!(!terminal.liquidation_summary_buckets.is_empty());
    assert!(!terminal.liquidation_chart_buckets.is_empty());

    let _ = terminal.update_liquidation_feed(Message::ClearLiquidations);

    assert!(terminal.liquidations.is_empty());
    assert!(terminal.liquidation_summary_buckets.is_empty());
    assert!(terminal.liquidation_chart_buckets.is_empty());
}
