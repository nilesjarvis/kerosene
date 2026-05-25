use crate::account::OpenOrder;
use crate::account_views::invalid_account_data;
use crate::account_views::style::compact_action_button;
use crate::app_state::TradingTerminal;
use crate::helpers::{self, optional_value_color, parse_positive_finite_number};
use crate::message::Message;

use iced::widget::{Space, container, row, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_open_order_row<'a>(
        &'a self,
        order: &'a OpenOrder,
        can_cancel: bool,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let (side_str, side_color) = open_order_side_display(&order.side, theme);
        let is_outcome_order = self.is_outcome_coin(&order.coin);

        let cancel_cell: Element<'_, Message> = if can_cancel {
            compact_action_button(
                "Cancel",
                theme.palette().danger,
                Message::CancelOrder {
                    coin: order.coin.clone(),
                    oid: order.oid,
                },
            )
        } else {
            text("").size(12).into()
        };

        let sz = parse_open_order_positive(&order.sz);
        let limit_px = parse_open_order_positive(&order.limit_px);
        let chase_inputs = open_order_chase_inputs(&order.side, sz, limit_px);
        let weak_color = theme.extended_palette().background.weak.text;
        let invalid_color = theme.palette().warning;

        let chase_cell: Element<'_, Message> = if can_cancel && !is_outcome_order {
            if let Some((is_buy, sz, limit_px)) = chase_inputs {
                compact_action_button(
                    "Chase",
                    theme.palette().primary,
                    Message::ChaseRestingOrder {
                        coin: order.coin.clone(),
                        oid: order.oid,
                        is_buy,
                        sz,
                        limit_px,
                        reduce_only: order.reduce_only,
                    },
                )
            } else {
                text("").size(12).into()
            }
        } else {
            text("").size(12).into()
        };

        let mut coin_content = row![];
        if let Some(icon) = helpers::symbol_icon(&order.coin, 14, theme.palette().text) {
            coin_content = coin_content.push(icon).push(Space::new().width(4.0));
        }
        let coin_label = if is_outcome_order {
            self.display_name_for_symbol(&order.coin)
        } else {
            order.coin.clone()
        };
        coin_content = coin_content
            .push(text(coin_label).size(12))
            .align_y(iced::Alignment::Center);

        row![
            coin_content.width(Fill),
            text(side_str).size(12).color(side_color).width(Fill),
            text(format_open_order_price(
                limit_px,
                is_outcome_order,
                &self.display_denomination_context(),
            ))
            .size(12)
            .font(crate::app_fonts::monospace_font())
            .color(optional_value_color(limit_px, weak_color, invalid_color))
            .width(Fill),
            text(format_open_order_size(sz, &order.sz))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(optional_value_color(sz, weak_color, invalid_color))
                .width(Fill),
            container(row![chase_cell, cancel_cell].spacing(4)).width(120),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .into()
    }
}

fn parse_open_order_positive(value: &str) -> Option<f64> {
    parse_positive_finite_number(value)
}

fn open_order_chase_inputs(
    side: &str,
    sz: Option<f64>,
    limit_px: Option<f64>,
) -> Option<(bool, f64, f64)> {
    let is_buy = match side {
        "B" => true,
        "A" => false,
        _ => return None,
    };
    Some((is_buy, sz?, limit_px?))
}

fn open_order_side_display(side: &str, theme: &Theme) -> (&'static str, Color) {
    match side {
        "B" => ("\u{2191} Buy", theme.palette().success),
        "A" => ("\u{2193} Sell", theme.palette().danger),
        _ => ("Invalid", theme.palette().warning),
    }
}

fn format_open_order_price(
    limit_px: Option<f64>,
    is_outcome: bool,
    denomination: &crate::denomination::DisplayDenominationContext,
) -> String {
    limit_px
        .map(|limit_px| {
            if is_outcome {
                format!("{limit_px:.4}")
            } else {
                denomination.format_value(limit_px, 2)
            }
        })
        .unwrap_or_else(invalid_account_data)
}

fn format_open_order_size(sz: Option<f64>, raw_sz: &str) -> String {
    sz.map(|_| raw_sz.to_string())
        .unwrap_or_else(invalid_account_data)
}

#[cfg(test)]
mod tests;
