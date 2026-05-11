use crate::app_state::TradingTerminal;
use crate::helpers::{self, format_price, format_usd};
use crate::message::Message;

use super::sort::PositionRowData;
use iced::widget::{Space, button, container, row, text};
use iced::{Element, Fill, Theme, color};

impl TradingTerminal {
    pub(super) fn view_position_row<'a>(
        &'a self,
        data: PositionRowData<'a>,
        can_close: bool,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let ap = data.ap;
        let pos = &ap.position;
        let (side, side_color) = match data.is_long {
            Some(true) => ("\u{2191} Long", theme.palette().success),
            Some(false) => ("\u{2193} Short", theme.palette().danger),
            None => ("Invalid", theme.palette().warning),
        };

        let mark_str = data
            .mark_px
            .map(|mid| format!("{mid:.4}"))
            .unwrap_or_else(|| "\u{2014}".to_string());
        let entry_str = data
            .entry_px
            .map(|_| pos.entry_px.clone())
            .unwrap_or_else(|| "Invalid".to_string());
        let size_str = data
            .szi
            .map(|szi| self.display_size_for_symbol(&pos.coin, szi.abs()))
            .unwrap_or_else(|| "Invalid".to_string());

        let pnl_color = data
            .upnl
            .map(|upnl| self.direction_color(theme, upnl))
            .unwrap_or_else(|| theme.palette().warning);
        let lev_str = format!("{}x {}", pos.leverage.value, pos.leverage.leverage_type);
        let liq_element: Element<'a, Message> = text(
            data.liq_px
                .map(format_price)
                .unwrap_or_else(|| "\u{2014}".to_string()),
        )
        .size(12)
        .font(iced::Font::MONOSPACE)
        .color(color!(0xff9d66))
        .into();

        let funding_color = match data.funding_since_open {
            Some(funding) if funding > 0.0 => self.direction_color(theme, funding),
            Some(funding) if funding < 0.0 => self.direction_color(theme, funding),
            Some(_) => theme.extended_palette().background.weak.text,
            None => theme.extended_palette().background.weak.text,
        };
        let total_pnl_color = data
            .total_pnl
            .map(|total_pnl| self.direction_color(theme, total_pnl))
            .unwrap_or_else(|| theme.palette().warning);

        let coin_key = pos.coin.clone();
        let mut coin_content = row![];
        if let Some(icon) = helpers::symbol_icon(&pos.coin, 14, theme.palette().text) {
            coin_content = coin_content.push(icon).push(Space::new().width(4.0));
        }
        coin_content = coin_content
            .push(text(&pos.coin).size(12))
            .align_y(iced::Alignment::Center);

        let symbol_btn = button(coin_content)
            .on_press(Message::SymbolSelected(coin_key))
            .padding(0)
            .style(|theme: &Theme, status| {
                let text_color = match status {
                    button::Status::Hovered => theme.palette().success,
                    _ => theme.palette().text,
                };
                button::Style {
                    background: None,
                    text_color,
                    ..Default::default()
                }
            });

        let row_can_close = can_close && data.szi.is_some_and(|szi| szi.abs() > 1e-12);
        let close_cell = self.view_position_close_cell(&pos.coin, row_can_close, theme);
        let (val_display, upnl_display, fund_display, total_display) = if self.hide_pnl {
            (
                data.position_value
                    .map(|_| "$***".to_string())
                    .unwrap_or_else(|| "Invalid".to_string()),
                data.upnl
                    .map(|_| "$***".to_string())
                    .unwrap_or_else(|| "Invalid".to_string()),
                "***".to_string(),
                data.total_pnl
                    .map(|_| "$***".to_string())
                    .unwrap_or_else(|| "Invalid".to_string()),
            )
        } else {
            (
                data.position_value
                    .map(|value| format_usd(&format!("{value:.2}")))
                    .unwrap_or_else(|| "Invalid".to_string()),
                data.upnl
                    .map(|upnl| format_usd(&format!("{upnl:.2}")))
                    .unwrap_or_else(|| "Invalid".to_string()),
                data.funding_since_open
                    .map(Self::format_signed_amount)
                    .unwrap_or_else(|| "-".to_string()),
                data.total_pnl
                    .map(|total_pnl| format_usd(&format!("{total_pnl:.2}")))
                    .unwrap_or_else(|| "Invalid".to_string()),
            )
        };

        let row_content = row![
            container(symbol_btn).width(Fill),
            text(side).size(12).color(side_color).width(Fill),
            text(size_str)
                .size(12)
                .font(iced::Font::MONOSPACE)
                .width(Fill),
            text(entry_str)
                .size(12)
                .font(iced::Font::MONOSPACE)
                .width(Fill),
            container(liq_element).width(Fill),
            text(mark_str)
                .size(12)
                .font(iced::Font::MONOSPACE)
                .width(Fill),
            text(val_display)
                .size(12)
                .font(iced::Font::MONOSPACE)
                .width(Fill),
            text(upnl_display)
                .size(12)
                .font(iced::Font::MONOSPACE)
                .color(pnl_color)
                .width(Fill),
            text(fund_display)
                .size(12)
                .font(iced::Font::MONOSPACE)
                .color(funding_color)
                .width(Fill),
            text(total_display)
                .size(13)
                .font(iced::Font::MONOSPACE)
                .color(total_pnl_color)
                .width(Fill),
            text(lev_str)
                .size(12)
                .font(iced::Font::MONOSPACE)
                .color(theme.extended_palette().background.weak.text)
                .width(Fill),
            container(close_cell).width(120),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center);

        container(row_content)
            .width(Fill)
            .padding([6, 8])
            .style(move |_theme: &Theme| {
                use iced::gradient;
                let mut base_color = side_color;
                base_color.a = 0.15;
                iced::widget::container::Style {
                    background: Some(
                        gradient::Linear::new(iced::Degrees(90.0))
                            .add_stop(0.0, base_color)
                            .add_stop(0.20, iced::Color::TRANSPARENT)
                            .add_stop(1.0, iced::Color::TRANSPARENT)
                            .into(),
                    ),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }
}
