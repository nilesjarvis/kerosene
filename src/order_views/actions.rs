mod chase;
mod twap;

use crate::app_state::TradingTerminal;
use crate::helpers::{buy_button, sell_button};
use crate::message::Message;
use crate::order_execution::PendingOrderAction;
use crate::signing::OrderKind;
use iced::widget::container as container_style;
use iced::widget::{Column, container, row, text};
use iced::{Color, Element, Fill, Theme, color};

// ---------------------------------------------------------------------------
// Order Action Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
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
        let pending_standard = pending_buy || pending_sell;

        let buy_label = format!("BUY {}", self.active_symbol_display.to_uppercase());
        let sell_label = format!("SELL {}", self.active_symbol_display.to_uppercase());
        let snapshot = self.ticket_order_submission_snapshot();
        let mut buy_btn: Element<'_, Message> = if pending_buy {
            pending_order_button(self.view_spinner(18), color!(0x30a050))
        } else {
            buy_button(
                buy_label,
                Message::PlaceOrder {
                    is_buy: true,
                    snapshot: snapshot.clone(),
                },
            )
        };
        let mut sell_btn: Element<'_, Message> = if pending_sell {
            pending_order_button(self.view_spinner(18), color!(0xdd3333))
        } else {
            sell_button(
                sell_label,
                Message::PlaceOrder {
                    is_buy: false,
                    snapshot,
                },
            )
        };

        if !can_trade || pending_standard {
            buy_btn = if pending_buy {
                pending_order_button(self.view_spinner(18), color!(0x30a050))
            } else {
                disabled_order_button("BUY")
            };
            sell_btn = if pending_sell {
                pending_order_button(self.view_spinner(18), color!(0xdd3333))
            } else {
                disabled_order_button("SELL")
            };
        }
        form.push(row![buy_btn, sell_btn].spacing(8))
    }
}

fn pending_order_button<'a>(
    spinner: Element<'a, Message>,
    background: Color,
) -> Element<'a, Message> {
    container(spinner)
        .padding([6, 0])
        .center_x(Fill)
        .width(Fill)
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(background.into()),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn disabled_order_button(label: &'static str) -> Element<'static, Message> {
    container(
        text(label)
            .size(12)
            .center()
            .width(Fill)
            .color(color!(0x666666)),
    )
    .padding([6, 0])
    .width(Fill)
    .style(|_theme: &Theme| container_style::Style {
        background: Some(color!(0x2a2a2a).into()),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}
