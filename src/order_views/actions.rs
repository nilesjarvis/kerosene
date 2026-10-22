mod chase;
mod twap;

use crate::app_state::TradingTerminal;
use crate::helpers::{TradeAction, TradeSide, trade_action_button};
use crate::message::Message;
use crate::order_execution::PendingOrderAction;
use crate::signing::OrderKind;
use iced::widget::{Column, row};

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

        let pending_buy = self.pending_order_action == Some(PendingOrderAction::Buy);
        let pending_sell = self.pending_order_action == Some(PendingOrderAction::Sell);

        let buy_label = format!("BUY {}", self.active_symbol_display.to_uppercase());
        let sell_label = format!("SELL {}", self.active_symbol_display.to_uppercase());
        let snapshot = self.ticket_order_submission_snapshot();
        let pane_corner_radius = self.pane_corner_radius;

        // One action family across idle, busy, and unavailable: identical
        // 36px frames, directional labels retained while busy, and quiet
        // directional identity while trading is unavailable.
        let buy_btn = trade_action_button(
            TradeSide::Buy,
            buy_label,
            if pending_buy {
                TradeAction::Busy
            } else if can_trade {
                TradeAction::Place(Box::new(Message::PlaceOrder {
                    is_buy: true,
                    snapshot: snapshot.clone(),
                }))
            } else {
                TradeAction::Unavailable
            },
            pending_buy.then(|| self.view_spinner(14)),
            pane_corner_radius,
        );
        let sell_btn = trade_action_button(
            TradeSide::Sell,
            sell_label,
            if pending_sell {
                TradeAction::Busy
            } else if can_trade {
                TradeAction::Place(Box::new(Message::PlaceOrder {
                    is_buy: false,
                    snapshot,
                }))
            } else {
                TradeAction::Unavailable
            },
            pending_sell.then(|| self.view_spinner(14)),
            pane_corner_radius,
        );

        form.push(row![buy_btn, sell_btn].spacing(8))
    }
}
