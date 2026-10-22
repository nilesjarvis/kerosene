use crate::app_state::TradingTerminal;
use crate::helpers::design::{UiColors, soft_action_pair};
use crate::helpers::format_price;
use crate::helpers::{TradeAction, TradeSide, trade_action_button};
use crate::message::Message;
use crate::order_execution::PendingOrderAction;
use crate::twap_state::MAX_ACTIVE_ADVANCED_ORDERS;
use iced::widget::{Column, button, row, text};
use iced::{Fill, Theme};

// ---------------------------------------------------------------------------
// Chase Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_chase_controls<'a>(
        &'a self,
        form: Column<'a, Message>,
        can_trade: bool,
    ) -> Column<'a, Message> {
        let pending_chase_buy = self.pending_order_action == Some(PendingOrderAction::ChaseBuy);
        let pending_chase_sell = self.pending_order_action == Some(PendingOrderAction::ChaseSell);

        let pane_corner_radius = self.pane_corner_radius;
        let symbol = self.active_symbol_display.to_uppercase();

        if pending_chase_buy || pending_chase_sell {
            // Same action family while starting: the pending side keeps its
            // directional label with the spinner; the other side is quietly
            // unavailable instead of a differently sized container.
            let chase_buy = trade_action_button(
                TradeSide::Buy,
                format!("CHASE BUY {symbol}"),
                if pending_chase_buy {
                    TradeAction::Busy
                } else {
                    TradeAction::Unavailable
                },
                pending_chase_buy.then(|| self.view_spinner(14)),
                pane_corner_radius,
            );
            let chase_sell = trade_action_button(
                TradeSide::Sell,
                format!("CHASE SELL {symbol}"),
                if pending_chase_sell {
                    TradeAction::Busy
                } else {
                    TradeAction::Unavailable
                },
                pending_chase_sell.then(|| self.view_spinner(14)),
                pane_corner_radius,
            );
            return form.push(row![chase_buy, chase_sell].spacing(8));
        }

        if can_trade && self.active_advanced_order_count() < MAX_ACTIVE_ADVANCED_ORDERS {
            let snapshot = self.advanced_order_start_snapshot();
            let chase_buy = trade_action_button(
                TradeSide::Buy,
                format!("CHASE BUY {symbol}"),
                TradeAction::Place(Box::new(Message::StartChase {
                    is_buy: true,
                    snapshot: snapshot.clone(),
                })),
                None,
                pane_corner_radius,
            );
            let chase_sell = trade_action_button(
                TradeSide::Sell,
                format!("CHASE SELL {symbol}"),
                TradeAction::Place(Box::new(Message::StartChase {
                    is_buy: false,
                    snapshot,
                })),
                None,
                pane_corner_radius,
            );
            form.push(row![chase_buy, chase_sell].spacing(8))
        } else if can_trade {
            form.push(
                text(format!(
                    "Maximum of {} active advanced orders reached",
                    MAX_ACTIVE_ADVANCED_ORDERS
                ))
                .size(11)
                .style(move |theme: &Theme| text::Style {
                    color: Some(UiColors::new(theme).negative),
                }),
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
            .style(move |theme: &Theme| text::Style {
                color: Some(UiColors::new(theme).muted),
            });
            // Stopping an algorithm is visually distinct from starting one:
            // a quiet destructive control, not another filled trade action.
            let pane_corner_radius = self.pane_corner_radius;
            let stop_btn = button(text("Stop Chase").size(12).center().width(Fill))
                .on_press(Message::StopChase)
                .height(24)
                .padding([0, 8])
                .width(Fill)
                .style(move |theme: &Theme, _status| {
                    let (fill, foreground) =
                        soft_action_pair(theme.palette().danger, UiColors::new(theme).panel);
                    button::Style {
                        background: Some(fill.into()),
                        text_color: foreground,
                        border: iced::Border {
                            radius: crate::config::effective_radius(pane_corner_radius, 4.0).into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                });
            form = form.push(chase_info).push(stop_btn);
        }

        form
    }
}
