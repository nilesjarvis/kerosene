mod chase;
mod twap;

#[cfg(test)]
mod tests;

use crate::app_state::TradingTerminal;
use crate::helpers::{buy_button, sell_button};
use crate::message::Message;
use crate::order_execution::{PendingOrderAction, TicketWait};
use crate::order_update::PlacementLookup;
use crate::signing::OrderKind;
use iced::widget::{Column, button, row, text};
use iced::{Element, Fill};

// ---------------------------------------------------------------------------
// Order Action Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_order_algorithm_settings<'a>(
        &'a self,
        form: Column<'a, Message>,
    ) -> Column<'a, Message> {
        match self.order_kind {
            OrderKind::Chase => self.push_chase_status(form),
            OrderKind::Twap => self.push_twap_settings(form),
            _ => form,
        }
    }

    pub(super) fn push_order_action_controls<'a>(
        &'a self,
        form: Column<'a, Message>,
        can_trade: bool,
    ) -> Column<'a, Message> {
        match self.order_kind {
            OrderKind::Chase => return self.push_chase_controls(form, can_trade),
            OrderKind::Twap => return self.push_twap_controls(form, can_trade),
            OrderKind::Market | OrderKind::Limit | OrderKind::LimitIoc => {}
        }

        let wait = self.standard_ticket_wait();
        let reselect_size = self.outdated_ticket_percentage_source().is_some();
        let enabled = can_trade && wait.is_none() && !reselect_size;
        let action = |is_buy: bool| {
            let label = format!(
                "{} {}",
                if is_buy { "BUY" } else { "SELL" },
                self.active_symbol_display.to_uppercase()
            );
            if !enabled {
                return disabled_order_button(label);
            }
            let message = Message::PlaceOrder {
                is_buy,
                snapshot: self.ticket_order_submission_snapshot(),
            };
            if is_buy {
                buy_button(label, message)
            } else {
                sell_button(label, message)
            }
        };
        let form = form.push(row![action(true), action(false)].spacing(8));
        match wait {
            Some(wait) if self.connected_address.is_some() => self.push_ticket_wait(form, wait),
            None if can_trade && reselect_size => form.push(
                text("Account updated. Reselect the percentage or enter a quantity.")
                    .size(11)
                    .color(self.theme().palette().warning),
            ),
            _ => form,
        }
    }

    fn push_ticket_wait<'a>(
        &'a self,
        form: Column<'a, Message>,
        wait: TicketWait<'_>,
    ) -> Column<'a, Message> {
        let mut control = None;
        let label = match wait {
            TicketWait::OrderStatus(check) => {
                let symbol = self.display_name_for_symbol(&check.context.symbol_key);
                if check.lookup == PlacementLookup::Checking {
                    format!("Checking {symbol} order status. Execution is unconfirmed.")
                } else {
                    control = Some((
                        "Check order status",
                        Message::RecheckTicketOrderStatus(check.request_id),
                    ));
                    format!("{symbol} order unconfirmed. Check open orders and fills.")
                }
            }
            TicketWait::TradingRequest => match self.pending_order_action {
                Some(PendingOrderAction::Buy) => "Awaiting buy confirmation…",
                Some(PendingOrderAction::Sell) => "Awaiting sell confirmation…",
                _ => "Waiting for the previous trading request",
            }
            .into(),
            TicketWait::Refreshing => "Refreshing account data…".into(),
            TicketWait::RateLimited => {
                "Account refresh rate limited. Retrying automatically.".into()
            }
            TicketWait::AccountDataUnavailable => {
                control = Some(("Refresh account", Message::RefreshAccountData));
                "Waiting for fresh account data".into()
            }
        };
        let color = if matches!(
            wait,
            TicketWait::OrderStatus(_)
                | TicketWait::AccountDataUnavailable
                | TicketWait::RateLimited
        ) {
            self.theme().palette().warning
        } else {
            self.theme().palette().text
        };
        let mut status = row![];
        if matches!(wait, TicketWait::Refreshing)
            || (matches!(wait, TicketWait::TradingRequest) && self.pending_order_action.is_some())
        {
            status = status.push(self.view_spinner(12));
        }
        let mut status = status
            .push(text(label).size(11).color(color).width(Fill))
            .align_y(iced::Alignment::Center)
            .spacing(4);
        if let Some((label, message)) = control {
            status = status.push(
                button(text(label).size(11))
                    .on_press(message)
                    .style(button::secondary)
                    .padding([2, 6]),
            );
        }
        form.push(status)
    }
}

fn disabled_order_button(label: String) -> Element<'static, Message> {
    // A different widget tree drops a press held across a disabled interval.
    iced::widget::column![
        button(text(label).size(14).center().width(Fill))
            .padding([8, 16])
            .width(Fill)
            .style(button::secondary)
    ]
    .into()
}
