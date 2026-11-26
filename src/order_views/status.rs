use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, button, row, text};
use iced::{Fill, Theme};

impl TradingTerminal {
    pub(super) fn push_order_status_feedback<'a>(
        &'a self,
        form: Column<'a, Message>,
        theme: &Theme,
    ) -> Column<'a, Message> {
        let mut form = form;
        if let Some((receipt, is_error)) = &self.ticket_receipt_during_refresh
            && self.account_reconciliation_required
            && self.ticket_receipt_during_refresh != self.order_status
        {
            let color = if *is_error {
                theme.palette().danger
            } else {
                theme.palette().success
            };
            form = form.push(text(receipt).size(11).color(color));
        }
        let Some((msg, is_err)) = &self.order_status else {
            return form;
        };

        let status_color = if *is_err {
            theme.palette().danger
        } else {
            theme.palette().success
        };
        let status_row = row![
            text(msg).size(11).color(status_color).width(Fill),
            button(text("X").size(10))
                .on_press(Message::DismissOrderStatus)
                .padding([1, 4])
                .style(button::text),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center);

        form.push(status_row)
    }

    pub(super) fn order_entry_unavailable_hint(&self) -> Option<&'static str> {
        let Some(account) = self.connected_address.as_deref() else {
            return Some("Connect wallet to trade");
        };
        if self.active_account_is_ghost() {
            return Some("Watch-only account");
        }
        if !self.has_active_committed_agent_key() {
            return Some("Add agent key to trade");
        }
        if !self.active_wallet_context_matches_connected_account(account) {
            return Some("Reconnect the active account to trade");
        }
        let orderable = self
            .resolve_exchange_symbol_by_key_or_ticker(&self.active_symbol)
            .is_some_and(|symbol| self.exchange_symbol_is_orderable(symbol));
        (!orderable).then_some("Market unavailable for trading")
    }

    pub(super) fn push_order_entry_hint<'a>(
        &self,
        form: Column<'a, Message>,
        active_is_outcome: bool,
        unavailable_hint: Option<&'static str>,
    ) -> Column<'a, Message> {
        if let Some(hint) = unavailable_hint {
            form.push(
                text(hint)
                    .size(10)
                    .color(self.theme().extended_palette().background.weak.text),
            )
        } else if active_is_outcome {
            let quote_symbol = self.outcome_quote_symbol_for_coin(&self.active_symbol);
            form.push(
                text(format!(
                    "Outcome orders use {quote_symbol} prices and whole-contract sizes"
                ))
                .size(10)
                .color(self.theme().extended_palette().background.weak.text),
            )
        } else {
            form
        }
    }
}
