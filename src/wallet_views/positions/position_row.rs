use crate::account::WalletPositionDetail;
use crate::app_state::TradingTerminal;
use crate::helpers::optional_value_color;
use crate::message::Message;
use crate::wallet_views::position_metrics::{wallet_position_upnl, wallet_position_value};
use crate::wallet_views::style::{wallet_signed_value_color, wallet_symbol_button};
use crate::wallet_views::wallet_dex_label;

use crate::wallet_views::numbers::{
    format_wallet_display_signed_usd, format_wallet_display_usd, invalid_wallet_data,
    parse_wallet_number,
};
use iced::widget::{row, text};
use iced::{Color, Element, Theme};

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(super) fn view_wallet_position_row<'a>(
        &'a self,
        detail: &'a WalletPositionDetail,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let denomination = self.display_denomination_context();
        let pos = &detail.asset_position.position;
        let symbol = Self::wallet_detail_symbol(&detail.dex, &pos.coin);
        let dex_label = wallet_dex_label(&detail.dex);
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
        let symbol_button = wallet_symbol_button(symbol, 95.0, &theme);

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
                .width(44),
            text(format_wallet_position_size(szi))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(optional_value_color(szi, weak_color, invalid_color))
                .width(84),
            text(format_wallet_display_price(&denomination, entry_px))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(optional_value_color(entry_px, weak_color, invalid_color))
                .width(78),
            text(
                mark_px
                    .map(|mark_px| denomination.format_price(mark_px))
                    .unwrap_or_else(|| "-".to_string())
            )
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .width(78),
            text(
                liq_px
                    .map(|liq_px| denomination.format_price(liq_px))
                    .unwrap_or_else(|| "-".to_string())
            )
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .width(78),
            text(format_wallet_display_usd(&denomination, position_value, 0))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(optional_value_color(
                    position_value,
                    weak_color,
                    invalid_color
                ))
                .width(84),
            text(format_wallet_display_signed_usd(&denomination, upnl))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(upnl_color)
                .width(84),
            text(
                funding
                    .map(|funding| {
                        format_wallet_display_signed_usd(&denomination, Some(funding))
                    })
                    .unwrap_or_else(|| "-".to_string())
            )
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(theme.extended_palette().background.weak.text)
            .width(84),
            text(format!("{}x", pos.leverage.value))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .width(44),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }
}

fn format_wallet_display_price(
    denomination: &crate::denomination::DisplayDenominationContext,
    value: Option<f64>,
) -> String {
    value
        .map(|value| denomination.format_price(value))
        .unwrap_or_else(invalid_wallet_data)
}

fn wallet_position_side(szi: Option<f64>, theme: &Theme) -> (&'static str, Color) {
    match szi {
        Some(szi) if szi >= 0.0 => ("Long", theme.palette().success),
        Some(_) => ("Short", theme.palette().danger),
        None => ("Invalid", theme.palette().warning),
    }
}

fn format_wallet_position_size(szi: Option<f64>) -> String {
    szi.map(|szi| format!("{:.4}", szi.abs()))
        .unwrap_or_else(invalid_wallet_data)
}
