use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::chart_state::{ChartId, ChartInstance};
use crate::order_execution::QuickOrderForm;
use crate::timeframe::Timeframe;

mod restoration;

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 7,
        collateral_token: None,
        sz_decimals: 4,
        max_leverage: 50,
        only_isolated: false,
        market_type,
        outcome: None,
    }
}

fn quick_order_form() -> QuickOrderForm {
    QuickOrderForm {
        price: 100.0,
        quantity: "1.25".to_string(),
        quantity_is_usd: false,
        percentage: 0.0,
        is_limit: true,
        click_x: 10.0,
        click_y: 20.0,
        chart_w: 400.0,
        chart_h: 300.0,
    }
}

fn terminal_with_quick_order(chart_id: ChartId, chart_symbol: &str) -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.wallet_key_input = sensitive_string("agent-key");
    terminal.exchange_symbols.clear();

    let mut instance = ChartInstance::new(chart_id, chart_symbol.to_string(), Timeframe::H1);
    instance.set_quick_order(quick_order_form());
    terminal.charts.insert(chart_id, instance);
    terminal
}

fn order_status_or_panic(terminal: &TradingTerminal) -> (&str, bool) {
    match terminal.order_status.as_ref() {
        Some((message, is_error)) => (message.as_str(), *is_error),
        None => panic!("missing order status"),
    }
}

fn chart_instance_or_panic(terminal: &TradingTerminal, chart_id: ChartId) -> &ChartInstance {
    match terminal.charts.get(&chart_id) {
        Some(instance) => instance,
        None => panic!("missing chart instance {chart_id}"),
    }
}

fn quick_order_or_panic(instance: &ChartInstance) -> &QuickOrderForm {
    match instance.quick_order.as_ref() {
        Some(form) => form,
        None => panic!("missing restored quick order"),
    }
}
