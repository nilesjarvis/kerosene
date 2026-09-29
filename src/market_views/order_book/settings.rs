#[path = "settings/symbol_mode.rs"]
mod symbol_mode;

use crate::app_state::TradingTerminal;
use crate::market_state::{OrderBookDisplayMode, OrderBookId, OrderBookInstance};
use crate::message::Message;
use iced::widget::{Space, button, column, container, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_order_book_settings<'a>(
        &'a self,
        id: OrderBookId,
        inst: &'a OrderBookInstance,
    ) -> Element<'a, Message> {
        let search_col = self.view_order_book_symbol_mode_controls(id, inst);
        let show_chart_btn = settings_toggle(
            if inst.show_spread_chart {
                "Hide Spread Chart"
            } else {
                "Show Spread Chart"
            },
            inst.show_spread_chart,
            Message::ToggleOrderBookSpreadChart(id),
        );

        let mut settings = column![search_col, Space::new().height(10.0)].spacing(4);
        // The depth chart has a fixed bids-left/asks-right layout, so the
        // orientation toggle would be inert there.
        if inst.display_mode != OrderBookDisplayMode::DepthChart {
            settings = settings.push(settings_toggle(
                if inst.reverse_side {
                    "Reversed Side"
                } else {
                    "Standard Side"
                },
                inst.reverse_side,
                Message::ToggleOrderBookReverseSide(id),
            ));
        }
        let settings = settings.push(show_chart_btn);

        container(settings)
            .padding(8)
            .style(move |theme: &Theme| container::Style {
                background: Some(theme.extended_palette().background.weak.color.into()),
                border: iced::border::rounded(4),
                ..Default::default()
            })
            .into()
    }
}

fn settings_toggle(
    label: &'static str,
    active: bool,
    message: Message,
) -> Element<'static, Message> {
    button(text(label).size(12).center().width(Fill))
        .on_press(message)
        .style(move |theme: &Theme, status| {
            let bg = if active {
                theme.extended_palette().background.strong.color
            } else {
                match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ => theme.extended_palette().background.base.color,
                }
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 2.0.into(),
                    width: if active { 1.0 } else { 0.0 },
                    color: if active {
                        Color {
                            a: 0.5,
                            ..theme.palette().primary
                        }
                    } else {
                        Color::TRANSPARENT
                    },
                },
                ..Default::default()
            }
        })
        .into()
}
