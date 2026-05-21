use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::{
    OrderBookDisplayMode, OrderBookId, OrderBookInstance, OrderBookSymbolMode,
};
use crate::message::Message;

use iced::widget::{button, column, container, row, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::market_views::order_book) fn view_order_book_header(
        reverse_side: bool,
    ) -> Element<'static, Message> {
        let labels = if reverse_side {
            ["Total", "Size", "Price"]
        } else {
            ["Price", "Size", "Total"]
        };

        row![
            header_cell(labels[0]),
            header_cell(labels[1]),
            header_cell(labels[2]),
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

    pub(in crate::market_views::order_book) fn view_order_book_outcome_metadata(
        &self,
        symbol: &str,
        inst: &OrderBookInstance,
    ) -> Option<Element<'static, Message>> {
        let exchange_symbol = self.exchange_symbols.iter().find(|sym| sym.key == symbol)?;
        let info = exchange_symbol.outcome.as_ref()?;
        let theme = self.theme();
        let condition = info.side_condition_label_with_countdown(Self::now_ms());
        let probability = inst
            .current_mid_price()
            .or_else(|| self.resolve_mid_for_symbol(symbol))
            .map(|mid| format!("{:.1}% implied", mid * 100.0))
            .unwrap_or_else(|| "mid n/a".to_string());
        let token_name = format!("+{}", info.encoding);
        let detail = format!(
            "{} | token {} | asset {} | quote {} | whole contracts",
            symbol, token_name, exchange_symbol.asset_index, info.quote_symbol
        );

        Some(
            container(
                column![
                    row![
                        text(format!("Pays if {condition}"))
                            .size(11)
                            .color(theme.palette().text)
                            .width(Fill),
                        text(probability)
                            .size(11)
                            .font(crate::app_fonts::monospace_font())
                            .color(theme.palette().primary),
                    ]
                    .spacing(6)
                    .align_y(iced::Alignment::Center),
                    text(detail)
                        .size(10)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(2),
            )
            .width(Fill)
            .padding([4, 6])
            .style(move |theme: &Theme| container::Style {
                background: Some(theme.extended_palette().background.weak.color.into()),
                border: iced::Border {
                    radius: 3.0.into(),
                    width: 1.0,
                    color: Color {
                        a: 0.25,
                        ..theme.palette().primary
                    },
                },
                ..Default::default()
            })
            .into(),
        )
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

fn header_cell(label: &'static str) -> Element<'static, Message> {
    text(label)
        .size(12)
        .width(Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .into()
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
