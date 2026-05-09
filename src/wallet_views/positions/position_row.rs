use crate::account::WalletPositionDetail;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use crate::wallet_views::numbers::{
    format_wallet_price, format_wallet_signed_usd, format_wallet_usd, invalid_wallet_data,
    parse_wallet_number,
};
use iced::widget::{button, row, text};
use iced::{Color, Element, Theme};

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(super) fn view_wallet_position_row<'a>(
        &'a self,
        detail: &'a WalletPositionDetail,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let pos = &detail.asset_position.position;
        let symbol = Self::wallet_detail_symbol(&detail.dex, &pos.coin);
        let dex_label = if detail.dex.is_empty() {
            "main".to_string()
        } else {
            detail.dex.clone()
        };
        let szi = parse_wallet_number(&pos.szi);
        let entry_px = parse_wallet_number(&pos.entry_px);
        let mark_px = self
            .resolve_mid_for_symbol(&symbol)
            .or_else(|| self.resolve_mid_for_symbol(&pos.coin));
        let position_value = wallet_position_value(szi, &pos.position_value, mark_px);
        let upnl = wallet_position_upnl(szi, entry_px, &pos.unrealized_pnl, mark_px);
        let funding = Self::position_funding_pnl(pos.cum_funding.as_ref());
        let (side, side_color) = wallet_position_side(szi, &theme);
        let upnl_color = wallet_signed_value_color(upnl, &theme);
        let invalid_color = theme.palette().warning;
        let weak_color = theme.extended_palette().background.weak.text;
        let liq_px = Self::parse_liquidation_px(&detail.asset_position);
        let symbol_for_message = symbol.clone();
        let symbol_button = button(
            text(symbol)
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(theme.palette().primary),
        )
        .on_press(Message::SymbolSelected(symbol_for_message))
        .padding(0)
        .width(95)
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            ..Default::default()
        });

        row![
            symbol_button,
            text(dex_label)
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(theme.extended_palette().background.weak.text)
                .width(60),
            text(side)
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(side_color)
                .width(44),
            text(format_wallet_position_size(szi))
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(wallet_value_color(szi, weak_color, invalid_color))
                .width(84),
            text(format_wallet_price(entry_px))
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(wallet_value_color(entry_px, weak_color, invalid_color))
                .width(78),
            text(
                mark_px
                    .map(|mark_px| format_wallet_price(Some(mark_px)))
                    .unwrap_or_else(|| "-".to_string())
            )
            .size(11)
            .font(iced::Font::MONOSPACE)
            .width(78),
            text(
                liq_px
                    .map(|liq_px| format_wallet_price(Some(liq_px)))
                    .unwrap_or_else(|| "-".to_string())
            )
            .size(11)
            .font(iced::Font::MONOSPACE)
            .width(78),
            text(format_wallet_usd(position_value, 0))
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(wallet_value_color(
                    position_value,
                    weak_color,
                    invalid_color
                ))
                .width(84),
            text(format_wallet_signed_usd(upnl))
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(upnl_color)
                .width(84),
            text(
                funding
                    .map(|funding| format_wallet_signed_usd(Some(funding)))
                    .unwrap_or_else(|| "-".to_string())
            )
            .size(11)
            .font(iced::Font::MONOSPACE)
            .color(theme.extended_palette().background.weak.text)
            .width(84),
            text(format!("{}x", pos.leverage.value))
                .size(11)
                .font(iced::Font::MONOSPACE)
                .width(44),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }
}

fn wallet_position_value(
    szi: Option<f64>,
    wire_position_value: &str,
    mark_px: Option<f64>,
) -> Option<f64> {
    match (szi, mark_px) {
        (Some(szi), Some(mark_px)) => Some(szi.abs() * mark_px),
        _ => parse_wallet_number(wire_position_value).map(f64::abs),
    }
}

fn wallet_position_upnl(
    szi: Option<f64>,
    entry_px: Option<f64>,
    wire_upnl: &str,
    mark_px: Option<f64>,
) -> Option<f64> {
    match (szi, entry_px, mark_px) {
        (Some(szi), Some(entry_px), Some(mark_px)) => Some(szi * (mark_px - entry_px)),
        _ => parse_wallet_number(wire_upnl),
    }
}

fn wallet_position_side(szi: Option<f64>, theme: &Theme) -> (&'static str, Color) {
    match szi {
        Some(szi) if szi >= 0.0 => ("Long", theme.palette().success),
        Some(_) => ("Short", theme.palette().danger),
        None => ("Invalid", theme.palette().warning),
    }
}

fn wallet_signed_value_color(value: Option<f64>, theme: &Theme) -> Color {
    match value {
        Some(value) if value > 0.0 => theme.palette().success,
        Some(value) if value < 0.0 => theme.palette().danger,
        Some(_) => theme.extended_palette().background.weak.text,
        None => theme.palette().warning,
    }
}

fn wallet_value_color(value: Option<f64>, default_color: Color, invalid_color: Color) -> Color {
    if value.is_some() {
        default_color
    } else {
        invalid_color
    }
}

fn format_wallet_position_size(szi: Option<f64>) -> String {
    szi.map(|szi| format!("{:.4}", szi.abs()))
        .unwrap_or_else(invalid_wallet_data)
}
