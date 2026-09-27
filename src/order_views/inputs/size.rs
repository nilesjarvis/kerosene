use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::signing::OrderKind;
use calculations::{denomination_label, order_notional_text, parse_positive_finite};
use components::denomination_button;
use presets::{SIZE_PERCENT_LABEL_WIDTH, SIZE_SLIDER_HEIGHT, size_slider_style};

use iced::widget::{
    Column, Space, button, checkbox, column, pick_list, row, slider, text, text_input,
};
use iced::{Fill, Length};

mod calculations;
mod components;
mod presets;

impl TradingTerminal {
    pub(super) fn push_size_input_controls<'a>(
        &'a self,
        mut form: Column<'a, Message>,
        active_is_outcome: bool,
        wide: bool,
    ) -> (Column<'a, Message>, Option<f64>) {
        let theme = self.theme();
        let qty_placeholder = if active_is_outcome {
            "Contracts"
        } else {
            "Quantity"
        };
        let qty_input = text_input(qty_placeholder, &self.order_quantity)
            .style(helpers::text_input_style)
            .on_input(|value| Message::OrderQuantityChanged(value.into()))
            .size(13)
            .padding(6);

        let parsed_qty = parse_positive_finite(&self.order_quantity);
        let parsed_price = if matches!(self.order_kind, OrderKind::Limit | OrderKind::LimitIoc) {
            parse_positive_finite(&self.order_price)
        } else {
            self.resolve_mid_for_symbol(&self.active_symbol)
                .and_then(helpers::positive_finite_value)
        };

        let (notional_val, notional_text) = order_notional_text(
            self.order_quantity_is_usd,
            &self.active_symbol_display,
            active_is_outcome,
            parsed_qty,
            parsed_price,
        );
        let size_header = row![
            text("Size")
                .size(12)
                .color(theme.extended_palette().background.weak.text),
            Space::new().width(Fill),
            denomination_button(denomination_label(
                self.order_quantity_is_usd,
                active_is_outcome,
                &self.active_symbol_display,
                &self.outcome_quote_symbol_for_coin(&self.active_symbol),
            )),
        ]
        .align_y(iced::Alignment::Center);

        let percent_slider = slider(
            0.0..=100.0,
            self.order_percentage,
            Message::OrderPercentageChanged,
        )
        .width(Fill)
        .height(SIZE_SLIDER_HEIGHT)
        .step(1.0)
        .style(size_slider_style);
        let slider_label = text(format!("{:.0}%", self.order_percentage))
            .size(11)
            .center()
            .width(Length::Fixed(SIZE_PERCENT_LABEL_WIDTH));
        let slider_row = row![percent_slider, slider_label]
            .spacing(8)
            .align_y(iced::Alignment::Center);

        let mut size = column![size_header, qty_input].spacing(4);
        if !notional_text.is_empty() {
            size = size.push(
                text(notional_text)
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .width(Fill)
                    .align_x(iced::alignment::Horizontal::Right),
            );
        }
        let mut percentages = row![].spacing(4);
        for pct in [25, 50, 75, 100] {
            percentages = percentages.push(
                button(text(format!("{pct}%")).size(11).center())
                    .on_press(Message::OrderPercentageChanged(pct as f32))
                    .padding([2, 4])
                    .width(Fill)
                    .style(button::text),
            );
        }
        form = if wide && self.order_kind != OrderKind::Twap {
            column![row![form.width(Fill), size.width(Fill)].spacing(8)].spacing(8)
        } else {
            form.push(size)
        };
        form = form.push(column![slider_row, percentages].spacing(0));

        (form, notional_val)
    }

    pub(in crate::order_views::inputs) fn push_order_execution_options<'a>(
        &'a self,
        mut form: Column<'a, Message>,
        active_is_spot: bool,
        active_is_outcome: bool,
    ) -> Column<'a, Message> {
        let limit_selected = matches!(self.order_kind, OrderKind::Limit | OrderKind::LimitIoc);
        let mut options_row = row![].spacing(14).align_y(iced::Alignment::Center);
        let mut has_options = false;

        if !active_is_spot && !active_is_outcome {
            has_options = true;
            options_row = options_row.push(
                checkbox(self.order_reduce_only)
                    .label("Reduce only")
                    .on_toggle(|_| Message::ToggleReduceOnly)
                    .size(14)
                    .text_size(12)
                    .text_shaping(iced::widget::text::Shaping::Advanced),
            );
        }
        if limit_selected {
            has_options = true;
            let selected = if self.order_kind == OrderKind::LimitIoc {
                "IOC"
            } else {
                "GTC"
            };
            options_row = options_row.push(Space::new().width(Fill)).push(
                row![
                    text("TIF")
                        .size(11)
                        .color(self.theme().extended_palette().background.weak.text),
                    pick_list(["GTC", "IOC"], Some(selected), |value| {
                        Message::SetOrderKind(if value == "IOC" {
                            OrderKind::LimitIoc
                        } else {
                            OrderKind::Limit
                        })
                    })
                    .text_size(11)
                    .padding([4, 6]),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            );
        }

        if has_options {
            form = form.push(options_row);
        }

        form
    }
}

#[cfg(test)]
mod tests;
