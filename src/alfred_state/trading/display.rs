use super::AlfredTradeSide;
use crate::helpers::format_decimal_with_commas;
use crate::signing::OrderKind;

pub(super) fn trade_amount_label(value: f64, is_usd: bool) -> String {
    let amount = display_amount(value);
    if is_usd { format!("${amount}") } else { amount }
}

fn display_amount(value: f64) -> String {
    let formatted = format_decimal_with_commas(value, 4);
    trim_decimal_zeros(formatted)
}

pub(super) fn plain_amount(value: f64) -> String {
    trim_decimal_zeros(format!("{value:.8}"))
}

fn trim_decimal_zeros(mut value: String) -> String {
    if value.contains('.') {
        while value.ends_with('0') {
            value.pop();
        }
        if value.ends_with('.') {
            value.pop();
        }
    }
    value
}

pub(super) fn trade_title(
    side: Option<AlfredTradeSide>,
    quantity: &str,
    symbol: &str,
    order_kind: OrderKind,
    price: Option<&str>,
) -> String {
    let side = match order_kind {
        OrderKind::Chase => side
            .map(|side| format!("CHASE {}", side.label()))
            .unwrap_or_else(|| "CHASE".to_string()),
        OrderKind::Market | OrderKind::Limit | OrderKind::LimitIoc | OrderKind::Twap => side
            .map(|side| side.label().to_string())
            .unwrap_or_else(|| "ORDER".to_string()),
    };
    let mut title = format!("{side} {quantity} {}", symbol.to_ascii_uppercase());
    if order_kind == OrderKind::Limit
        && let Some(price) = price
    {
        title.push_str(" @ ");
        title.push_str(price);
    }
    title
}

pub(super) fn trade_detail(order_kind: OrderKind, quantity_is_usd: bool) -> String {
    let quantity = if quantity_is_usd {
        "USD notional"
    } else {
        "coin size"
    };
    match order_kind {
        OrderKind::Limit => format!("Limit order, {quantity}"),
        OrderKind::Market => format!("Market order, {quantity}"),
        OrderKind::Chase => format!("Chase order, {quantity}"),
        OrderKind::LimitIoc | OrderKind::Twap => "Trade draft".to_string(),
    }
}
