use crate::app_state::TradingTerminal;
use crate::helpers::format_price;
use crate::message::Message;
use crate::order_execution::PendingOrderAction;
use iced::widget::container as container_style;
use iced::widget::{Column, button, container, row, text};
use iced::{Color, Element, Fill, Theme, color};

// ---------------------------------------------------------------------------
// Chase Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_chase_controls<'a>(
        &'a self,
        form: Column<'a, Message>,
        can_trade: bool,
    ) -> Column<'a, Message> {
        let theme = self.theme();
        let pending_chase_buy = self.pending_order_action == Some(PendingOrderAction::ChaseBuy);
        let pending_chase_sell = self.pending_order_action == Some(PendingOrderAction::ChaseSell);

        if pending_chase_buy || pending_chase_sell {
            let chase_buy = self.pending_chase_control(true, pending_chase_buy);
            let chase_sell = self.pending_chase_control(false, pending_chase_sell);
            return form.push(row![chase_buy, chase_sell].spacing(8));
        }

        let mut form = form;
        if let Some(chase) = self.selected_chase() {
            let side_str = if chase.is_buy { "BUY" } else { "SELL" };
            let price = if chase.current_price.is_finite() && chase.current_price > 0.0 {
                format_price(chase.current_price)
            } else {
                "loading".to_string()
            };
            let chase_info = text(format!(
                "Chasing {side_str} {} {:.4} @ {} ({} active)",
                chase.coin.as_str(),
                chase.remaining_size,
                price,
                self.chase_orders.len()
            ))
            .size(11)
            .color(theme.palette().primary);
            let stop_btn = button(
                text("Stop Chase")
                    .size(10)
                    .center()
                    .color(theme.palette().danger)
                    .width(Fill),
            )
            .on_press(Message::StopChase)
            .padding([4, 12])
            .width(Fill)
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => color!(0x5a2020),
                    _ => color!(0x3a2020),
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().danger,
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            });
            form = form.push(chase_info).push(stop_btn);
        }

        if can_trade && self.chase_orders.len() < Self::MAX_ACTIVE_CHASE_ORDERS {
            let chase_buy = chase_start_button(
                format!("CHASE BUY {}", self.active_symbol_display.to_uppercase()),
                true,
                theme.palette().success,
            );
            let chase_sell = chase_start_button(
                format!("CHASE SELL {}", self.active_symbol_display.to_uppercase()),
                false,
                theme.palette().danger,
            );
            form.push(row![chase_buy, chase_sell].spacing(8))
        } else if can_trade {
            form.push(
                text(format!(
                    "Maximum of {} active chase orders reached",
                    Self::MAX_ACTIVE_CHASE_ORDERS
                ))
                .size(10)
                .color(theme.palette().danger),
            )
        } else {
            form
        }
    }

    fn pending_chase_control(&self, is_buy: bool, is_pending: bool) -> Element<'_, Message> {
        let theme = self.theme();
        let accent = if is_buy {
            theme.palette().success
        } else {
            theme.palette().danger
        };
        let dim_text = if is_buy {
            color!(0x507a5e)
        } else {
            color!(0x8a5757)
        };
        let bg = if is_buy {
            color!(0x1b2320)
        } else {
            color!(0x231b1b)
        };
        let label = if is_buy {
            format!("CHASE BUY {}", self.active_symbol_display.to_uppercase())
        } else {
            format!("CHASE SELL {}", self.active_symbol_display.to_uppercase())
        };

        if is_pending {
            container(self.view_spinner(14))
                .padding([8, 0])
                .center(Fill)
                .style(move |_theme: &Theme| muted_action_style(accent, 0.3))
                .into()
        } else if self.pending_order_action.is_some() {
            container(text("Chase").size(14).color(color!(0xffffff)))
                .padding([8, 0])
                .center(Fill)
                .style(move |_theme: &Theme| muted_action_style(accent, 0.3))
                .into()
        } else {
            container(text(label).size(10).center().color(dim_text).width(Fill))
                .padding([4, 8])
                .width(Fill)
                .style(move |_theme: &Theme| container_style::Style {
                    background: Some(bg.into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        width: 1.0,
                        color: Color { a: 0.15, ..accent },
                    },
                    ..Default::default()
                })
                .into()
        }
    }
}

fn chase_start_button(label: String, is_buy: bool, accent: Color) -> Element<'static, Message> {
    let (message, bg_hover, bg_default) = if is_buy {
        (
            Message::StartChase(true),
            color!(0x162c1d),
            color!(0x122017),
        )
    } else {
        (
            Message::StartChase(false),
            color!(0x2c1616),
            color!(0x201212),
        )
    };

    button(
        text(label)
            .size(10)
            .center()
            .color(Color { a: 0.8, ..accent })
            .width(Fill),
    )
    .on_press(message)
    .padding([4, 8])
    .width(Fill)
    .style(move |_theme: &Theme, status| {
        let bg = match status {
            button::Status::Hovered => bg_hover,
            _ => bg_default,
        };
        button::Style {
            background: Some(bg.into()),
            text_color: accent,
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color { a: 0.15, ..accent },
            },
            ..Default::default()
        }
    })
    .into()
}

fn muted_action_style(accent: Color, border_alpha: f32) -> container_style::Style {
    container_style::Style {
        background: Some(Color { a: 0.15, ..accent }.into()),
        border: iced::Border {
            radius: 3.0.into(),
            width: 1.0,
            color: Color {
                a: border_alpha,
                ..accent
            },
        },
        ..Default::default()
    }
}
