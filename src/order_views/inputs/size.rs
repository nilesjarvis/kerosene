use crate::app_state::TradingTerminal;
use crate::helpers::{self, format_usd};
use crate::message::Message;
use crate::signing::OrderKind;

use iced::widget::{Column, Space, button, row, text, text_input};
use iced::{Color, Fill, Theme};

impl TradingTerminal {
    pub(super) fn push_size_input_controls<'a>(
        &'a self,
        mut form: Column<'a, Message>,
        active_is_spot: bool,
        active_is_outcome: bool,
    ) -> (Column<'a, Message>, Option<f64>) {
        let theme = self.theme();
        let qty_placeholder = if active_is_outcome {
            "Contracts"
        } else {
            "Quantity"
        };
        let qty_input = text_input(qty_placeholder, &self.order_quantity)
            .style(helpers::text_input_style)
            .on_input(Message::OrderQuantityChanged)
            .size(13)
            .padding(6);

        let parsed_qty = parse_positive_finite(&self.order_quantity);
        let parsed_price =
            if matches!(
                self.order_kind,
                OrderKind::Limit | OrderKind::Chase | OrderKind::LimitIoc
            ) {
                parse_positive_finite(&self.order_price)
            } else {
                self.resolve_mid_for_symbol(&self.active_symbol)
                    .filter(|price| price.is_finite() && *price > 0.0)
            };

        let (notional_val, notional_text) = order_notional_text(
            self.order_quantity_is_usd,
            &self.active_symbol,
            parsed_qty,
            parsed_price,
        );
        let size_header = row![
            text("Size")
                .size(12)
                .color(theme.extended_palette().background.weak.text),
            Space::new().width(6.0),
            denomination_button(denomination_label(
                self.order_quantity_is_usd,
                active_is_outcome
            )),
            Space::new().width(Fill),
            text(notional_text)
                .size(11)
                .color(theme.extended_palette().background.weak.text),
        ]
        .align_y(iced::Alignment::Center);

        let percent_slider = iced::widget::slider(
            0.0..=100.0,
            self.order_percentage,
            Message::OrderPercentageChanged,
        )
        .step(1.0)
        .style(|theme: &Theme, status| {
            let palette = theme.palette();
            let mut style = iced::widget::slider::default(theme, status);
            style.handle.background = palette.primary.into();
            style.handle.border_color = palette.primary;
            style.rail.backgrounds.0 = palette.primary.into();
            style.rail.backgrounds.1 = Color {
                a: 0.2,
                ..palette.text
            }
            .into();
            style
        });

        let slider_label = text(format!("{:.0}%", self.order_percentage))
            .size(10)
            .color(theme.extended_palette().background.weak.text);
        let slider_row = row![percent_slider, Space::new().width(6.0), slider_label]
            .spacing(4)
            .align_y(iced::Alignment::Center);

        form = form.push(size_header).push(qty_input).push(slider_row);
        if !active_is_spot && !active_is_outcome {
            form = form.push(
                iced::widget::checkbox(self.order_reduce_only)
                    .label("Reduce Only")
                    .on_toggle(|_| Message::ToggleReduceOnly)
                    .size(14)
                    .text_size(12)
                    .text_shaping(iced::widget::text::Shaping::Advanced),
            );
        }

        (form, notional_val)
    }
}

fn parse_positive_finite(value: &str) -> Option<f64> {
    let parsed = value.trim().parse::<f64>().ok()?;
    (parsed.is_finite() && parsed > 0.0).then_some(parsed)
}

fn order_notional_text(
    quantity_is_usd: bool,
    active_symbol: &str,
    parsed_qty: Option<f64>,
    parsed_price: Option<f64>,
) -> (Option<f64>, String) {
    let Some(parsed_qty) = parsed_qty else {
        return (None, String::new());
    };

    if quantity_is_usd {
        let coin_text = parsed_price
            .and_then(|price| {
                let coin_val = parsed_qty / price;
                (coin_val.is_finite() && coin_val > 0.0).then_some(coin_val)
            })
            .map(|coin_val| {
                let mut search_coin = active_symbol;
                if let Some((_, suffix)) = search_coin.split_once(':') {
                    search_coin = suffix;
                }
                format!("\u{2248} {coin_val:.4} {search_coin}")
            })
            .unwrap_or_default();
        (Some(parsed_qty), coin_text)
    } else {
        let Some(parsed_price) = parsed_price else {
            return (None, String::new());
        };
        let notional = parsed_qty * parsed_price;
        if !notional.is_finite() || notional <= 0.0 {
            return (None, String::new());
        }
        (
            Some(notional),
            format!("\u{2248} {}", format_usd(&format!("{notional:.2}"))),
        )
    }
}

fn denomination_label(order_quantity_is_usd: bool, active_is_outcome: bool) -> &'static str {
    if active_is_outcome {
        if order_quantity_is_usd {
            "USDH"
        } else {
            "CONTRACTS"
        }
    } else if order_quantity_is_usd {
        "USD"
    } else {
        "COIN"
    }
}

fn denomination_button<'a>(label: &'static str) -> button::Button<'a, Message> {
    button(text(label).size(10).center())
        .on_press(Message::ToggleOrderDenomination)
        .padding([2, 6])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::{order_notional_text, parse_positive_finite};

    #[test]
    fn size_input_parser_rejects_invalid_nonpositive_or_nonfinite_values() {
        assert_eq!(parse_positive_finite("12.5"), Some(12.5));
        assert_eq!(parse_positive_finite("0"), None);
        assert_eq!(parse_positive_finite("-1"), None);
        assert_eq!(parse_positive_finite("NaN"), None);
        assert_eq!(parse_positive_finite("bad"), None);
    }

    #[test]
    fn usd_quantity_keeps_known_notional_when_price_is_missing() {
        assert_eq!(
            order_notional_text(true, "BTC", Some(100.0), None),
            (Some(100.0), String::new())
        );
    }

    #[test]
    fn coin_quantity_requires_valid_reference_price_for_notional() {
        assert_eq!(
            order_notional_text(false, "BTC", Some(2.0), None),
            (None, String::new())
        );
        assert_eq!(
            order_notional_text(false, "BTC", Some(2.0), Some(125.0)),
            (Some(250.0), "\u{2248} $250.00".to_string())
        );
    }
}
