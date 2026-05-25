use super::*;
use crate::api::{ExchangeSymbol, MarketType};
use crate::chart_state::ChartInstance;
use crate::timeframe::Timeframe;

fn symbol(key: &str) -> ExchangeSymbol {
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
        market_type: MarketType::Perp,
        outcome: None,
    }
}

#[test]
fn open_quick_order_reuses_last_type_and_size_for_same_chart_symbol() {
    let chart_id = 7;
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.wallet_key_input = "agent-key".to_string().into();
    terminal.exchange_symbols = vec![symbol("BTC")];
    terminal.primary_chart_id = Some(chart_id);

    let mut instance = ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1);
    instance.last_quick_order_symbol = "BTC".to_string();
    instance.last_quick_order_quantity = "2.5".to_string();
    instance.last_quick_order_quantity_is_usd = false;
    instance.last_quick_order_percentage = 25.0;
    instance.last_quick_order_is_limit = false;
    terminal.charts.insert(chart_id, instance);

    let _task = terminal.handle_open_quick_order(QuickOrderOpenRequest {
        chart_id,
        surface_id: ChartSurfaceId::Docked(chart_id),
        price: 101.0,
        click_x: 10.0,
        click_y: 20.0,
        chart_w: 300.0,
        chart_h: 200.0,
    });

    let instance = terminal.charts.get(&chart_id).expect("chart instance");
    let form = instance.quick_order.as_ref().expect("quick order form");
    assert!(!form.is_limit);
    assert_eq!(form.quantity, "2.5");
    assert!(!form.quantity_is_usd);
    assert_eq!(form.percentage, 25.0);
    assert_eq!(form.price, 101.0);
    assert_eq!(instance.chart.quick_order_limit_price, None);
}
