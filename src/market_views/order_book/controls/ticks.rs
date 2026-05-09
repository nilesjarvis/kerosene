use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::{OrderBookId, OrderBookInstance};
use crate::message::Message;

use iced::widget::{button, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::market_views::order_book) fn resolved_order_book_tick(
        inst: &OrderBookInstance,
        tick_options: &[f64],
    ) -> f64 {
        if tick_options
            .iter()
            .any(|&opt| (opt - inst.tick_size).abs() / opt.max(1e-12) < 0.01)
        {
            inst.tick_size
        } else {
            helpers::default_tick_for_price(inst.book.mid_price())
        }
    }

    pub(in crate::market_views::order_book) fn view_order_book_tick_buttons(
        id: OrderBookId,
        tick_options: &[f64],
        tick: f64,
    ) -> Element<'static, Message> {
        tick_options
            .iter()
            .fold(iced::widget::Row::new().spacing(4), |row_w, &opt| {
                let is_active = (opt - tick).abs() / opt.max(1e-12) < 0.01;
                let label = helpers::format_tick(opt);
                row_w.push(
                    button(text(label).size(10).center().width(Fill))
                        .on_press(Message::SetBookTickSize(id, opt))
                        .padding([2, 6])
                        .style(move |theme: &Theme, status| {
                            let bg = if is_active {
                                theme.extended_palette().background.strong.color
                            } else {
                                match status {
                                    button::Status::Hovered => {
                                        theme.extended_palette().background.weak.color
                                    }
                                    _ => theme.extended_palette().background.base.color,
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
                        }),
                )
            })
            .into()
    }
}
