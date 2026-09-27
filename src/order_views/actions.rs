mod chase;
mod twap;

use crate::app_state::TradingTerminal;
use crate::helpers::{buy_button, sell_button};
use crate::message::Message;
use crate::order_execution::PendingOrderAction;
use crate::signing::OrderKind;
use iced::widget::container as container_style;
use iced::widget::{Column, button, container, row, text};
use iced::{Color, Element, Fill, Theme};

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
        let pending_standard = pending_buy || pending_sell;

        let buy_label = format!("BUY {}", self.active_symbol_display.to_uppercase());
        let sell_label = format!("SELL {}", self.active_symbol_display.to_uppercase());
        let palette = self.theme().palette();
        let snapshot = self.ticket_order_submission_snapshot();
        let mut buy_btn: Element<'_, Message> = if pending_buy {
            pending_order_button(self.view_spinner(14), &buy_label, palette.success)
        } else {
            buy_button(
                buy_label.clone(),
                Message::PlaceOrder {
                    is_buy: true,
                    snapshot: snapshot.clone(),
                },
            )
        };
        let mut sell_btn: Element<'_, Message> = if pending_sell {
            pending_order_button(self.view_spinner(14), &sell_label, palette.danger)
        } else {
            sell_button(
                sell_label.clone(),
                Message::PlaceOrder {
                    is_buy: false,
                    snapshot,
                },
            )
        };

        if !can_trade || pending_standard {
            buy_btn = if pending_buy {
                pending_order_button(self.view_spinner(14), &buy_label, palette.success)
            } else {
                disabled_order_button(buy_label)
            };
            sell_btn = if pending_sell {
                pending_order_button(self.view_spinner(14), &sell_label, palette.danger)
            } else {
                disabled_order_button(sell_label)
            };
        }
        form.push(row![buy_btn, sell_btn].spacing(8))
    }
}

fn pending_order_button<'a>(
    spinner: Element<'a, Message>,
    label: &str,
    background: Color,
) -> Element<'a, Message> {
    container(
        row![spinner, text(label.to_string()).size(14)]
            .spacing(6)
            .align_y(iced::Alignment::Center),
    )
    .padding([8, 0])
    .center_x(Fill)
    .width(Fill)
    .style(move |_theme: &Theme| container_style::Style {
        background: Some(background.into()),
        text_color: Some(crate::helpers::text_color_for_bg(background)),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn disabled_order_button(label: String) -> Element<'static, Message> {
    button(text(label).size(14).center().width(Fill))
        .padding([8, 16])
        .width(Fill)
        .style(button::secondary)
        .into()
}
