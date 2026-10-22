use crate::app_state::TradingTerminal;
use crate::config::effective_radius;
use crate::helpers;
use crate::helpers::design::{UiColors, compact_button_style};
use crate::market_state::{OrderBookId, OrderBookInstance};
use crate::message::Message;

use iced::widget::{button, container, text};
use iced::{Element, Fill, Theme};

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
            // The stored tick fell out of the option set (price regime
            // moved): snap to the nearest option instead of silently
            // resetting to the default, so the selector always has an
            // active button and the user's coarseness choice is preserved.
            helpers::nearest_tick_option(tick_options, inst.tick_size)
        }
    }

    pub(in crate::market_views::order_book) fn view_order_book_tick_buttons(
        &self,
        id: OrderBookId,
        tick_options: &[f64],
        tick: f64,
    ) -> Element<'static, Message> {
        let pane_corner_radius = self.pane_corner_radius;
        // Tick-size choices are the same segmented control as the display
        // modes: equal widths on a shared inset track, with the selected
        // coarseness marked by the shared soft-accent recipe.
        let segments =
            tick_options
                .iter()
                .fold(iced::widget::Row::new().spacing(2), |row_w, &opt| {
                    let is_active = (opt - tick).abs() / opt.max(1e-12) < 0.01;
                    let label = helpers::format_tick(opt);
                    row_w.push(
                        button(text(label).size(12).center())
                            .on_press(Message::SetBookTickSize(id, opt))
                            .height(24)
                            .padding([0, 6])
                            .width(Fill)
                            .style(move |theme: &Theme, status| {
                                compact_button_style(theme, status, is_active, pane_corner_radius)
                            }),
                    )
                });

        container(segments)
            .width(Fill)
            .padding(2)
            .style(move |theme: &Theme| container::Style {
                background: Some(UiColors::new(theme).inset.into()),
                border: iced::Border {
                    radius: effective_radius(pane_corner_radius, 4.0).into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }
}
