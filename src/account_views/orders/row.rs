use crate::account::OpenOrder;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;

use iced::widget::{Space, button, container, row, text};
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
            button(
                text("Cancel")
                    .size(10)
                    .center()
                    .color(theme.palette().danger),
            )
            .on_press(Message::CancelOrder {
                coin: order.coin.clone(),
                oid: order.oid,
            })
            .padding([1, 6])
            .style(|theme: &Theme, _status| button::Style {
                background: Some(
                    Color {
                        a: 0.15,
                        ..theme.palette().danger
                    }
                    .into(),
                ),
                text_color: theme.palette().danger,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
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
                button(
                    text("Chase")
                        .size(10)
                        .center()
                        .color(theme.palette().primary),
                )
                .on_press(Message::ChaseRestingOrder {
                    coin: order.coin.clone(),
                    oid: order.oid,
                    is_buy,
                    sz,
                    limit_px,
                    reduce_only: order.reduce_only,
                })
                .padding([1, 6])
                .style(|theme: &Theme, _status| button::Style {
                    background: Some(
                        Color {
                            a: 0.15,
                            ..theme.palette().primary
                        }
                        .into(),
                    ),
                    text_color: theme.palette().primary,
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into()
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
            .font(iced::Font::MONOSPACE)
            .color(open_order_value_color(limit_px, weak_color, invalid_color))
            .width(Fill),
            text(format_open_order_size(sz, &order.sz))
                .size(12)
                .font(iced::Font::MONOSPACE)
                .color(open_order_value_color(sz, weak_color, invalid_color))
                .width(Fill),
            container(row![chase_cell, cancel_cell].spacing(4)).width(120),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .into()
    }
}

fn parse_open_order_positive(value: &str) -> Option<f64> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
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
        .unwrap_or_else(|| "Invalid data".to_string())
}

fn format_open_order_size(sz: Option<f64>, raw_sz: &str) -> String {
    sz.map(|_| raw_sz.to_string())
        .unwrap_or_else(|| "Invalid data".to_string())
}

fn open_order_value_color(value: Option<f64>, default_color: Color, invalid_color: Color) -> Color {
    if value.is_some() {
        default_color
    } else {
        invalid_color
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_order_positive_parser_rejects_invalid_zero_or_nonfinite_values() {
        assert_eq!(parse_open_order_positive(" 1.25 "), Some(1.25));
        assert_eq!(parse_open_order_positive("0"), None);
        assert_eq!(parse_open_order_positive("-1"), None);
        assert_eq!(parse_open_order_positive("bad"), None);
        assert_eq!(parse_open_order_positive("NaN"), None);
        assert_eq!(parse_open_order_positive("inf"), None);
    }

    #[test]
    fn chase_inputs_require_known_side_size_and_price() {
        assert_eq!(
            open_order_chase_inputs("B", Some(2.0), Some(100.0)),
            Some((true, 2.0, 100.0))
        );
        assert_eq!(
            open_order_chase_inputs("A", Some(2.0), Some(100.0)),
            Some((false, 2.0, 100.0))
        );
        assert_eq!(open_order_chase_inputs("bad", Some(2.0), Some(100.0)), None);
        assert_eq!(open_order_chase_inputs("B", None, Some(100.0)), None);
        assert_eq!(open_order_chase_inputs("B", Some(2.0), None), None);
    }

    #[test]
    fn open_order_formatters_mark_invalid_values() {
        let denomination = crate::denomination::DisplayDenominationContext::default();
        assert_eq!(
            format_open_order_price(Some(100.0), false, &denomination),
            "$100.00"
        );
        assert_eq!(
            format_open_order_price(Some(0.42), true, &denomination),
            "0.4200"
        );
        assert_eq!(
            format_open_order_price(None, false, &denomination),
            "Invalid data"
        );
        assert_eq!(format_open_order_size(Some(2.0), "2.0000"), "2.0000");
        assert_eq!(format_open_order_size(None, "bad"), "Invalid data");
    }
}
