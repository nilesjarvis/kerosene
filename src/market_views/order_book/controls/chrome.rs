use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::{
    OrderBookDisplayMode, OrderBookId, OrderBookInstance, OrderBookSymbolMode,
};
use crate::message::Message;

use iced::widget::{button, row, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::market_views::order_book) fn view_order_book_header() -> Element<'static, Message>
    {
        row![
            text("Price")
                .size(12)
                .width(Fill)
                .align_x(iced::alignment::Horizontal::Right),
            text("Size")
                .size(12)
                .width(Fill)
                .align_x(iced::alignment::Horizontal::Right),
            text("Total")
                .size(12)
                .width(Fill)
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(4)
        .into()
    }

    pub(in crate::market_views::order_book) fn view_order_book_title(
        &self,
        id: OrderBookId,
        inst: &OrderBookInstance,
    ) -> Element<'static, Message> {
        let tracking_text = match &inst.mode {
            OrderBookSymbolMode::Active => format!("Active: {}", self.active_symbol_display),
            OrderBookSymbolMode::Fixed(symbol) => self
                .exchange_symbols
                .iter()
                .find(|exchange_symbol| &exchange_symbol.key == symbol)
                .map(|exchange_symbol| {
                    exchange_symbol
                        .display_name
                        .as_deref()
                        .unwrap_or(exchange_symbol.key.as_str())
                })
                .unwrap_or(symbol.as_str())
                .to_string(),
        };

        row![
            text(format!("Order Book ({tracking_text})"))
                .size(13)
                .style(move |theme: &Theme| text::Style {
                    color: Some(theme.palette().text)
                })
                .width(Fill),
            display_mode_button(
                id,
                inst.display_mode,
                OrderBookDisplayMode::DepthList,
                "Book"
            ),
            display_mode_button(
                id,
                inst.display_mode,
                OrderBookDisplayMode::DomLadder,
                "DOM"
            ),
            button(text("\u{2699}").size(12).style(move |theme: &Theme| {
                text::Style {
                    color: Some(theme.extended_palette().background.weak.text),
                }
            }))
            .style(button::text)
            .on_press(Message::ToggleOrderBookSettings(id))
            .padding(2)
        ]
        .align_y(iced::Alignment::Center)
        .into()
    }

    pub(in crate::market_views::order_book) fn view_order_book_spread_chart<'a>(
        id: OrderBookId,
        inst: &'a OrderBookInstance,
    ) -> Element<'a, Message> {
        let mid = inst.book.mid_price();
        let spread_decimals = helpers::tick_decimals(helpers::default_tick_for_price(mid));

        iced::widget::canvas(crate::spread_chart::SpreadChart {
            id,
            data: &inst.spread_history,
            spread_decimals,
        })
        .width(Fill)
        .height(iced::Length::Fixed(inst.spread_chart_height))
        .into()
    }
}

fn display_mode_button(
    id: OrderBookId,
    active: OrderBookDisplayMode,
    mode: OrderBookDisplayMode,
    label: &'static str,
) -> button::Button<'static, Message> {
    let is_active = active == mode;
    button(text(label).size(10).center())
        .on_press(Message::SetOrderBookDisplayMode(id, mode))
        .padding([2, 6])
        .style(move |theme: &Theme, status| {
            let bg = if is_active {
                theme.extended_palette().background.strong.color
            } else {
                match status {
                    button::Status::Hovered => theme.extended_palette().background.weak.color,
                    _ => Color::TRANSPARENT,
                }
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if is_active {
                    theme.palette().text
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 2.0.into(),
                    width: if is_active { 1.0 } else { 0.0 },
                    color: if is_active {
                        Color {
                            a: 0.4,
                            ..theme.palette().primary
                        }
                    } else {
                        Color::TRANSPARENT
                    },
                },
                ..Default::default()
            }
        })
}
