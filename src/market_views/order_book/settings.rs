#[path = "settings/symbol_mode.rs"]
mod symbol_mode;

use crate::app_state::TradingTerminal;
use crate::config::effective_radius;
use crate::helpers::design::{UiColors, compact_button_style};
use crate::market_state::{OrderBookDisplayMode, OrderBookId, OrderBookInstance};
use crate::message::Message;
use iced::widget::{Space, button, column, container, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_order_book_settings<'a>(
        &'a self,
        id: OrderBookId,
        inst: &'a OrderBookInstance,
    ) -> Element<'a, Message> {
        let pane_corner_radius = self.pane_corner_radius;
        let search_col = self.view_order_book_symbol_mode_controls(id, inst);
        let show_chart_btn = settings_toggle(
            if inst.show_spread_chart {
                "Hide Spread Chart"
            } else {
                "Show Spread Chart"
            },
            inst.show_spread_chart,
            Message::ToggleOrderBookSpreadChart(id),
            pane_corner_radius,
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
                pane_corner_radius,
            ));
        }
        let settings = settings.push(show_chart_btn);

        container(settings)
            .padding(8)
            .style(move |theme: &Theme| container::Style {
                background: Some(UiColors::new(theme).inset.into()),
                border: iced::Border {
                    radius: effective_radius(pane_corner_radius, 6.0).into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }
}

fn settings_toggle(
    label: &'static str,
    active: bool,
    message: Message,
    pane_corner_radius: f32,
) -> Element<'static, Message> {
    // Choice toggles use the shared compact selected-toggle recipe so the
    // active option keeps its soft fill and outline in every state.
    button(text(label).size(12).center().width(Fill))
        .on_press(message)
        .width(Fill)
        .height(24)
        .style(move |theme: &Theme, status| {
            compact_button_style(theme, status, active, pane_corner_radius)
        })
        .into()
}
