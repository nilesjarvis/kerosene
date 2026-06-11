use crate::account::WalletOpenOrderDetail;
use crate::app_state::TradingTerminal;
use crate::denomination::DisplayDenominationContext;
use crate::helpers::{self, optional_value_color};
use crate::message::Message;
use crate::wallet_views::numbers::{
    format_wallet_display_usd, format_wallet_price, invalid_wallet_data, parse_wallet_number,
};
use crate::wallet_views::wallet_dex_label;

use iced::widget::{Row, button, row, text};
use iced::{Color, Element, Length, Theme};

#[cfg(test)]
mod tests;

pub(super) fn wallet_orders_header() -> Row<'static, Message> {
    row![
        text("Coin").size(10).width(110),
        text("Dex").size(10).width(60),
        text("Side").size(10).width(50),
        text("Size").size(10).width(86),
        text("Price").size(10).width(86),
        text("Notional").size(10).width(90),
        text("Age").size(10).width(76),
        text("OID").size(10).width(86),
    ]
    .spacing(8)
}

pub(super) fn wallet_order_row(
    detail: &WalletOpenOrderDetail,
    symbol_label: String,
    is_outcome: bool,
    now_ms: u64,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    let order = &detail.order;
    let symbol = TradingTerminal::wallet_detail_symbol(&detail.dex, &order.coin);
    let dex_label = wallet_dex_label(&detail.dex);
    let (side, side_color) = wallet_order_side(&order.side, theme);
    let size = parse_wallet_number(&order.sz);
    let price = parse_wallet_number(&order.limit_px);
    let notional = wallet_order_notional(size, price);
    let weak_color = theme.extended_palette().background.weak.text;
    let invalid_color = theme.palette().warning;
    let symbol_button = wallet_order_symbol_button(symbol_label, symbol, theme);

    row![
        symbol_button,
        text(dex_label)
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(theme.extended_palette().background.weak.text)
            .width(60),
        text(side)
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(side_color)
            .width(50),
        text(format_wallet_order_size(size, is_outcome))
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(optional_value_color(size, weak_color, invalid_color))
            .width(86),
        text(format_wallet_price(price))
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(optional_value_color(price, weak_color, invalid_color))
            .width(86),
        text(format_wallet_display_usd(denomination, notional, 0))
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(optional_value_color(notional, weak_color, invalid_color))
            .width(90),
        text(helpers::format_relative_time(order.timestamp, now_ms))
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(theme.extended_palette().background.weak.text)
            .width(76),
        text(order.oid.to_string())
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(theme.extended_palette().background.weak.text)
            .width(86),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

/// Like `wallet_symbol_button`, but the rendered label may differ from the
/// symbol key carried by the press message.
fn wallet_order_symbol_button(
    label: String,
    symbol: String,
    theme: &Theme,
) -> Element<'static, Message> {
    button(
        text(label)
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(theme.palette().primary),
    )
    .on_press(Message::SymbolSelected(symbol))
    .padding(0)
    .width(Length::Fixed(110.0))
    .style(|_theme: &Theme, _status| button::Style {
        background: None,
        ..Default::default()
    })
    .into()
}

fn wallet_order_side(side: &str, theme: &Theme) -> (&'static str, Color) {
    match side {
        "B" => ("Buy", theme.palette().success),
        "A" => ("Sell", theme.palette().danger),
        _ => ("Invalid", theme.palette().warning),
    }
}

fn wallet_order_notional(size: Option<f64>, price: Option<f64>) -> Option<f64> {
    size.zip(price).map(|(size, price)| size * price)
}

fn format_wallet_order_size(size: Option<f64>, is_outcome: bool) -> String {
    size.map(|size| {
        if is_outcome {
            format!("{size:.0}")
        } else {
            format!("{size:.4}")
        }
    })
    .unwrap_or_else(invalid_wallet_data)
}
