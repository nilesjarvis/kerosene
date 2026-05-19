mod actions;
mod advanced;
mod advanced_history_details;
mod header;
mod inputs;
mod presets;
mod quick_order;
mod status;
mod twap_details;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{column, container, scrollable, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_order_entry(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let active_is_spot = self.is_spot_coin(&self.active_symbol);
        let active_is_outcome = self.is_outcome_coin(&self.active_symbol);
        let active_symbol_is_orderable = self
            .resolve_exchange_symbol_by_key_or_ticker(&self.active_symbol)
            .is_some_and(|symbol| self.exchange_symbol_is_orderable(symbol));
        let can_trade = self.connected_address.is_some()
            && !self.wallet_key_input.trim().is_empty()
            && active_symbol_is_orderable;

        let (symbol_row, margin_used) = self.view_order_entry_symbol_row(&theme);
        let context_row = self.view_order_entry_context_row(margin_used, &theme);
        let type_row = self.view_order_entry_type_row();
        let mut form = column![symbol_row, context_row, type_row].spacing(8);

        if active_is_outcome {
            form = form.push(
                text("USDH outcome contract. Prices are probabilities; size is whole contracts.")
                    .size(10)
                    .color(theme.palette().primary),
            );
        }

        form = self.push_order_input_controls(form, active_is_spot, active_is_outcome);

        form = self.push_order_action_controls(form, can_trade);

        form = self.push_order_presets_menu(form, active_is_outcome);

        form = self.push_order_status_feedback(form, &theme);
        form = self.push_order_entry_hint(form, active_is_outcome, can_trade);

        container(
            scrollable(form).direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4)
                    .margin(0)
                    .scroller_width(4),
            )),
        )
        .width(Fill)
        .height(Fill)
        .padding(iced::Padding {
            top: 10.0,
            right: 14.0,
            bottom: 10.0,
            left: 10.0,
        }) // Add right padding to prevent scrollbar overlap
        .into()
    }
}
