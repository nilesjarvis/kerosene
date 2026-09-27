mod actions;
mod advanced;
mod advanced_history_details;
mod details;
mod header;
mod inputs;
mod presets;
mod quick_order;
mod status;
mod twap_details;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{button, column, container, responsive, row, rule, scrollable, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_order_entry(&self) -> Element<'_, Message> {
        responsive(move |size| self.view_order_entry_for_width(size.width))
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn view_order_entry_for_width(&self, width: f32) -> Element<'_, Message> {
        let wide = width >= 560.0;
        let theme = self.theme();
        let active_is_spot = self.is_spot_coin(&self.active_symbol);
        let active_is_outcome = self.is_outcome_coin(&self.active_symbol);
        let active_symbol_is_orderable = self
            .resolve_exchange_symbol_by_key_or_ticker(&self.active_symbol)
            .is_some_and(|symbol| self.exchange_symbol_is_orderable(symbol));
        let can_trade = self.connected_address.is_some()
            && self.has_active_committed_agent_key()
            && active_symbol_is_orderable;

        let (symbol_row, margin_used) = self.view_order_entry_symbol_row(&theme);
        let context_row = self.view_order_entry_context_row(margin_used, &theme);
        let type_row = self.view_order_entry_type_row();
        let mut form = if wide {
            column![
                row![symbol_row, context_row]
                    .spacing(16)
                    .align_y(iced::Alignment::Center)
            ]
        } else {
            column![symbol_row, context_row]
        }
        .spacing(8);
        if self.order_leverage_dropdown_open
            && let Some(leverage_dropdown) = self.view_order_entry_leverage_dropdown(can_trade)
        {
            form = form.push(leverage_dropdown);
        }
        form = form.push(type_row);

        if active_is_outcome {
            let quote_symbol = self.outcome_quote_symbol_for_coin(&self.active_symbol);
            form = form.push(
                text(format!(
                    "{quote_symbol} outcome contract. Size is whole contracts."
                ))
                .size(10)
                .color(theme.palette().primary),
            );
            if let Some(info) = self
                .resolve_exchange_symbol_by_key_or_ticker(&self.active_symbol)
                .and_then(|symbol| symbol.outcome.as_ref())
            {
                if let Some(question) = &info.question_name {
                    form = form.push(text(question.clone()).size(11));
                }
                if let Some(reason) = info.trading_block_reason(self.status_bar_now_ms) {
                    form = form.push(
                        text(reason.to_string())
                            .size(11)
                            .color(theme.palette().danger),
                    );
                }
                if let Some(deadline) = info.contract_deadline_label() {
                    form = form.push(text(deadline).size(10));
                }
                if let Some(venue) = info.venue_label() {
                    form = form.push(text(format!("Venue: {venue}")).size(11));
                }
                if let Some(source) = info.settlement_source_label() {
                    form = form.push(
                        text(source)
                            .size(10)
                            .color(theme.extended_palette().background.weak.text),
                    );
                }
                if let Some(rules) = &info.contract.rules {
                    let expanded = self.outcome_expanded_rules.contains(&info.outcome_id);
                    form = form.push(
                        button(
                            text(if expanded {
                                "Hide contract rules"
                            } else {
                                "Contract rules"
                            })
                            .size(11),
                        )
                        .on_press(Message::OutcomeRulesToggled(info.outcome_id))
                        .style(button::text)
                        .padding([2, 0]),
                    );
                    if expanded {
                        form = form.push(text(rules.clone()).size(11).width(Fill));
                    }
                }
            }
        }

        form = self.push_order_input_controls(form, active_is_spot, active_is_outcome, wide);

        let actions = self.push_order_action_controls(column![].spacing(6), can_trade);
        let feedback = self.push_order_status_feedback(column![].spacing(4), &theme);
        let feedback = self.push_order_entry_hint(feedback, active_is_outcome, can_trade);
        let feedback = container(scrollable(feedback)).max_height(60).width(Fill);
        let footer_content: Element<'_, Message> = if wide {
            row![feedback, actions.width(Fill)]
                .spacing(16)
                .align_y(iced::Alignment::Center)
                .into()
        } else {
            column![actions, feedback].spacing(6).into()
        };
        let footer = column![rule::horizontal(1), footer_content].spacing(6);

        container(
            column![
                scrollable(container(form).padding(iced::Padding::default().right(8)))
                    .height(Fill)
                    .direction(iced::widget::scrollable::Direction::Vertical(
                        iced::widget::scrollable::Scrollbar::new()
                            .width(4)
                            .margin(0)
                            .scroller_width(4),
                    )),
                footer
            ]
            .spacing(8),
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
