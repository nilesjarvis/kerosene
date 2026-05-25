use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::signing::OrderKind;
use calculations::{denomination_label, order_notional_text, parse_positive_finite};
use components::denomination_button;
use presets::{SIZE_SLIDER_HEIGHT, SizePresetDots};

use iced::widget::{Column, Space, canvas, checkbox, row, slider, stack, text, text_input};
use iced::{Color, Fill, Length, Theme};

mod calculations;
mod components;
mod presets;

impl TradingTerminal {
    pub(super) fn push_size_input_controls<'a>(
        &'a self,
        mut form: Column<'a, Message>,
        active_is_spot: bool,
        active_is_outcome: bool,
    ) -> (Column<'a, Message>, Option<f64>) {
        let theme = self.theme();
        let qty_placeholder = if active_is_outcome {
            "Contracts"
        } else {
            "Quantity"
        };
        let qty_input = text_input(qty_placeholder, &self.order_quantity)
            .style(helpers::text_input_style)
            .on_input(Message::OrderQuantityChanged)
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
            &self.active_symbol,
            parsed_qty,
            parsed_price,
        );
        let size_header = row![
            text("Size")
                .size(12)
                .color(theme.extended_palette().background.weak.text),
            Space::new().width(6.0),
            denomination_button(denomination_label(
                self.order_quantity_is_usd,
                active_is_outcome
            )),
            Space::new().width(Fill),
            text(notional_text)
                .size(11)
                .color(theme.extended_palette().background.weak.text),
        ]
        .align_y(iced::Alignment::Center);

        let percent_slider = slider(
            0.0..=100.0,
            self.order_percentage,
            Message::OrderPercentageChanged,
        )
        .width(Fill)
        .step(1.0)
        .style(|theme: &Theme, status| {
            let palette = theme.palette();
            let mut style = slider::default(theme, status);
            style.handle.background = palette.primary.into();
            style.handle.border_color = palette.primary;
            style.rail.backgrounds.0 = palette.primary.into();
            style.rail.backgrounds.1 = Color {
                a: 0.2,
                ..palette.text
            }
            .into();
            style
        });
        let preset_markers = canvas(SizePresetDots {
            current_pct: self.order_percentage,
        })
        .width(Fill)
        .height(Length::Fixed(SIZE_SLIDER_HEIGHT));
        let size_slider = stack![percent_slider, preset_markers].width(Fill);

        let slider_label = text(format!("{:.0}%", self.order_percentage))
            .size(10)
            .color(theme.extended_palette().background.weak.text);
        let slider_row = row![size_slider, Space::new().width(6.0), slider_label]
            .spacing(4)
            .align_y(iced::Alignment::Center);

        form = form.push(size_header).push(qty_input).push(slider_row);

        let limit_selected = matches!(self.order_kind, OrderKind::Limit | OrderKind::LimitIoc);
        let mut options_row = row![].spacing(14).align_y(iced::Alignment::Center);
        let mut has_options = false;

        if !active_is_spot && !active_is_outcome {
            has_options = true;
            options_row = options_row.push(
                checkbox(self.order_reduce_only)
                    .label("Reduce Only")
                    .on_toggle(|_| Message::ToggleReduceOnly)
                    .size(14)
                    .text_size(12)
                    .text_shaping(iced::widget::text::Shaping::Advanced),
            );
        }
        if limit_selected {
            has_options = true;
            options_row = options_row.push(
                checkbox(self.order_kind == OrderKind::LimitIoc)
                    .label("IOC")
                    .on_toggle(|enabled| {
                        Message::SetOrderKind(if enabled {
                            OrderKind::LimitIoc
                        } else {
                            OrderKind::Limit
                        })
                    })
                    .size(14)
                    .text_size(12)
                    .text_shaping(iced::widget::text::Shaping::Advanced),
            );
        }

        if has_options {
            form = form.push(options_row);
        }

        (form, notional_val)
    }
}

#[cfg(test)]
mod tests;
