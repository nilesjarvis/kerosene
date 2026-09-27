use crate::app_state::TradingTerminal;
use crate::helpers::format_price;
use crate::message::Message;
use crate::order_execution::{AdvancedOrderStartSnapshot, PendingOrderAction};
use crate::twap_state::MAX_ACTIVE_ADVANCED_ORDERS;
use iced::widget::container as container_style;
use iced::widget::{Column, button, container, row, text};
use iced::{Color, Element, Fill, Theme};

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

        if can_trade && self.active_advanced_order_count() < MAX_ACTIVE_ADVANCED_ORDERS {
            let snapshot = self.advanced_order_start_snapshot();
            let chase_buy = chase_start_button(
                format!("CHASE BUY {}", self.active_symbol_display.to_uppercase()),
                true,
                theme.palette().success,
                snapshot.clone(),
            );
            let chase_sell = chase_start_button(
                format!("CHASE SELL {}", self.active_symbol_display.to_uppercase()),
                false,
                theme.palette().danger,
                snapshot,
            );
            form.push(row![chase_buy, chase_sell].spacing(8))
        } else if can_trade {
            form.push(
                text(format!(
                    "Maximum of {} active advanced orders reached",
                    MAX_ACTIVE_ADVANCED_ORDERS
                ))
                .size(10)
                .color(theme.palette().danger),
            )
        } else {
            form
        }
    }

    pub(in crate::order_views) fn push_chase_status<'a>(
        &'a self,
        form: Column<'a, Message>,
    ) -> Column<'a, Message> {
        if matches!(
            self.pending_order_action,
            Some(PendingOrderAction::ChaseBuy | PendingOrderAction::ChaseSell)
        ) {
            return form;
        }
        let theme = self.theme();
        let mut form = form;
        if let Some(chase) = self.selected_chase() {
            let side_str = if chase.is_buy { "BUY" } else { "SELL" };
            let price = if chase.current_price.is_finite() && chase.current_price > 0.0 {
                format_price(chase.current_price)
            } else {
                "loading".to_string()
            };
            let order_id = chase
                .current_oid
                .map(|oid| format!(" order #{oid}"))
                .unwrap_or_default();
            let chase_info = text(format!(
                "Chasing {side_str} {}{} {}/{} rem {} @ {} ({} active)",
                self.display_name_for_symbol(&chase.coin),
                order_id,
                self.display_size_for_symbol(&chase.coin, chase.filled_size),
                self.display_size_for_symbol(&chase.coin, chase.target_size),
                self.display_size_for_symbol(&chase.coin, chase.remaining_size),
                price,
                self.active_advanced_order_count()
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
                let bg = Color {
                    a: if matches!(status, button::Status::Hovered) {
                        0.16
                    } else {
                        0.1
                    },
                    ..theme.palette().danger
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

        form
    }

    fn pending_chase_control(&self, is_buy: bool, is_pending: bool) -> Element<'_, Message> {
        let theme = self.theme();
        let accent = if is_buy {
            theme.palette().success
        } else {
            theme.palette().danger
        };
        let label = format!(
            "CHASE {} {}",
            if is_buy { "BUY" } else { "SELL" },
            self.active_symbol_display.to_uppercase()
        );
        let content = if is_pending {
            row![self.view_spinner(14), text(label).size(10)]
                .spacing(6)
                .align_y(iced::Alignment::Center)
        } else {
            row![text(label).size(10)]
        };
        container(content)
            .padding([6, 8])
            .center_x(Fill)
            .style(move |_theme: &Theme| muted_action_style(accent, 0.15))
            .into()
    }
}

fn chase_start_button(
    label: String,
    is_buy: bool,
    accent: Color,
    snapshot: AdvancedOrderStartSnapshot,
) -> Element<'static, Message> {
    let message = Message::StartChase { is_buy, snapshot };

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
        let bg = Color {
            a: if matches!(status, button::Status::Hovered) {
                0.16
            } else {
                0.1
            },
            ..accent
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
        background: Some(Color { a: 0.1, ..accent }.into()),
        text_color: Some(accent),
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
