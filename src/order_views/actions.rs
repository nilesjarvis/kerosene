mod chase;
mod twap;

use crate::app_state::TradingTerminal;
use crate::helpers::{buy_button, sell_button};
use crate::message::Message;
use crate::order_execution::PendingOrderAction;
use iced::widget::container as container_style;
use iced::widget::{Column, container, row, text};
use iced::{Element, Fill, Theme, color};

// ---------------------------------------------------------------------------
// Order Action Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_order_action_controls<'a>(
        &'a self,
        form: Column<'a, Message>,
        can_trade: bool,
    ) -> Column<'a, Message> {
        let pending_buy = self.pending_order_action == Some(PendingOrderAction::Buy);
        let pending_sell = self.pending_order_action == Some(PendingOrderAction::Sell);
        let pending_standard = pending_buy || pending_sell;

        let buy_label = format!("BUY {}", self.active_symbol_display.to_uppercase());
        let sell_label = format!("SELL {}", self.active_symbol_display.to_uppercase());
        let mut buy_btn: Element<'_, Message> = buy_button(buy_label, Message::PlaceBuy);
        let mut sell_btn: Element<'_, Message> = sell_button(sell_label, Message::PlaceSell);
        if pending_buy {
            buy_btn = container(self.view_spinner(18))
                .padding([6, 0])
                .center_x(Fill)
                .width(Fill)
                .style(move |_theme: &Theme| container_style::Style {
                    background: Some(color!(0x30a050).into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into();
        }
        if pending_sell {
            sell_btn = container(self.view_spinner(18))
                .padding([6, 0])
                .center_x(Fill)
                .width(Fill)
                .style(move |_theme: &Theme| container_style::Style {
                    background: Some(color!(0xdd3333).into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into();
        }

        if !can_trade || pending_standard {
            buy_btn = disabled_order_button("BUY");
            if pending_buy {
                buy_btn = container(self.view_spinner(18))
                    .padding([6, 0])
                    .center_x(Fill)
                    .width(Fill)
                    .style(move |_theme: &Theme| container_style::Style {
                        background: Some(color!(0x30a050).into()),
                        border: iced::Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .into();
            }
            sell_btn = disabled_order_button("SELL");
            if pending_sell {
                sell_btn = container(self.view_spinner(18))
                    .padding([6, 0])
                    .center_x(Fill)
                    .width(Fill)
                    .style(move |_theme: &Theme| container_style::Style {
                        background: Some(color!(0xdd3333).into()),
                        border: iced::Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .into();
            }
        }
        let form = form.push(row![buy_btn, sell_btn].spacing(8));
        let form = self.push_chase_controls(form, can_trade);
        self.push_twap_controls(form, can_trade)
    }
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
