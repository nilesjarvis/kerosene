use super::*;
use crate::api::{ExchangeSymbol, MarketType};
use crate::chart::OrderOverlay;
use crate::chart_state::ChartInstance;
use crate::signing::{ChaseOrder, OrderKind};
use crate::timeframe::Timeframe;

mod overlays;
mod start;

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 5,
        max_leverage: 50,
        only_isolated: false,
        market_type,
        outcome: None,
    }
}

fn chase_ready_terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.wallet_key_input = "agent-key".to_string().into();
    terminal.connected_address = Some("0xabc".to_string());
    terminal.order_kind = OrderKind::Chase;
    terminal.order_quantity = "2.5".to_string();
    terminal.order_quantity_is_usd = false;
    terminal.pending_order_action = None;
    terminal
}

fn selected_chase(terminal: &TradingTerminal) -> &ChaseOrder {
    match terminal.selected_chase() {
        Some(chase) => chase,
        None => panic!("chase order should be inserted"),
    }
}

fn selected_chase_id(terminal: &TradingTerminal) -> u64 {
    match terminal.selected_chase_id() {
        Some(chase_id) => chase_id,
        None => panic!("resting chase should be selected"),
    }
}

fn chart_instance(terminal: &TradingTerminal, chart_id: u64) -> &ChartInstance {
    match terminal.charts.get(&chart_id) {
        Some(instance) => instance,
        None => panic!("chart instance {chart_id}"),
    }
}

fn chart_instance_mut(terminal: &mut TradingTerminal, chart_id: u64) -> &mut ChartInstance {
    match terminal.charts.get_mut(&chart_id) {
        Some(instance) => instance,
        None => panic!("chart instance {chart_id}"),
    }
}

fn order_status_error_contains(terminal: &TradingTerminal, needle: &str) -> bool {
    terminal
        .order_status
        .as_ref()
        .is_some_and(|(message, is_error)| *is_error && message.contains(needle))
}
