use super::price_range::parse_positive_price;
use super::validation::{
    TwapStartSchedule, parse_twap_start_schedule, validate_twap_schedule_capacity,
};
use super::*;
use crate::api::{ExchangeSymbol, MarketType};
use crate::signing::OrderKind;
use crate::twap_state::{TwapOrder, TwapStatus};

mod prices;
mod schedule;
mod submission;

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

fn twap_ready_terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.wallet_key_input = "agent-key".to_string().into();
    terminal.connected_address = Some("0xabc".to_string());
    terminal.order_kind = OrderKind::Twap;
    terminal.order_quantity = "2.5".to_string();
    terminal.order_quantity_is_usd = false;
    terminal.twap_form.duration_minutes = "5".to_string();
    terminal.twap_form.slices = "2".to_string();
    terminal.twap_form.min_price = "90".to_string();
    terminal.twap_form.max_price = "110".to_string();
    terminal.twap_form.randomize = false;
    terminal.pending_order_action = None;
    terminal
}

fn started_twap_or_panic(terminal: &TradingTerminal) -> &TwapOrder {
    let Some(twap_id) = terminal.selected_twap_id else {
        panic!("started twap should be selected");
    };
    match terminal.twap_orders.get(&twap_id) {
        Some(twap) => twap,
        None => panic!("twap order should be inserted"),
    }
}

fn parse_schedule_or_panic(duration_minutes: &str, slices: &str) -> TwapStartSchedule {
    match parse_twap_start_schedule(duration_minutes, slices) {
        Ok(schedule) => schedule,
        Err(error) => panic!("valid TWAP schedule: {error}"),
    }
}

fn schedule_error_or_panic(duration_minutes: &str, slices: &str) -> String {
    match parse_twap_start_schedule(duration_minutes, slices) {
        Ok(_) => panic!("invalid TWAP schedule should fail"),
        Err(error) => error,
    }
}

fn schedule_capacity_error_or_panic(active_slice_rate: f64, schedule: TwapStartSchedule) -> String {
    match validate_twap_schedule_capacity(
        active_slice_rate,
        schedule.duration,
        schedule.slice_count,
    ) {
        Ok(()) => panic!("dense TWAP schedule should fail"),
        Err(error) => error,
    }
}
