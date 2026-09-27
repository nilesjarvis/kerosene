use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::signing::OrderKind;

use iced::Fill;
use iced::Theme;
use iced::widget::{Column, Space, button, column, row, text, text_input};

impl TradingTerminal {
    pub(super) fn push_price_input_controls<'a>(
        &'a self,
        form: Column<'a, Message>,
    ) -> Column<'a, Message> {
        let theme = self.theme();
        match self.order_kind {
            OrderKind::Market => {
                return form.push(market_price_label(
                    self.resolve_mid_for_symbol(&self.active_symbol),
                    self.market_slippage_pct,
                    &theme,
                ));
            }
            OrderKind::Chase => {
                return form.push(chase_price_label(
                    self.resolve_mid_for_symbol(&self.active_symbol),
                    &theme,
                ));
            }
            OrderKind::Twap => return form,
            OrderKind::Limit | OrderKind::LimitIoc => {}
        }

        let price_input = text_input("Price", &self.order_price)
            .style(helpers::text_input_style)
            .on_input(|value| Message::OrderPriceChanged(value.into()))
            .size(13)
            .padding(6);
        let mid_btn = button(text("Mid").size(10).center())
            .on_press(Message::SetMidPrice)
            .padding([3, 8])
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
            });
        let price_row = row![price_input, mid_btn]
            .spacing(4)
            .align_y(iced::Alignment::Center);

        let quote = if self.is_outcome_coin(&self.active_symbol) {
            self.outcome_quote_symbol_for_coin(&self.active_symbol)
        } else if self.is_spot_coin(&self.active_symbol) {
            self.active_symbol_display
                .rsplit_once('/')
                .map(|(_, quote)| quote.to_string())
                .unwrap_or_else(|| "\u{2014}".to_string())
        } else {
            "USD".to_string()
        };
        let label = row![
            text("Price")
                .size(12)
                .color(theme.extended_palette().background.weak.text),
            Space::new().width(Fill),
            text(quote)
                .size(11)
                .color(theme.extended_palette().background.weak.text),
        ];
        form.push(column![label, price_row].spacing(4))
    }
}

fn market_price_label<'a>(
    mid: Option<f64>,
    slippage_pct: f64,
    theme: &Theme,
) -> iced::widget::Text<'a> {
    let market_info = if let Some(mid) = mid {
        format!("Market (~${mid:.2} mid, {slippage_pct:.2}% slippage)")
    } else {
        "Market (no mid data)".to_string()
    };

    text(market_info)
        .size(11)
        .color(theme.extended_palette().background.weak.text)
}

fn chase_price_label<'a>(mid: Option<f64>, theme: &Theme) -> iced::widget::Text<'a> {
    let chase_info = if let Some(mid) = mid {
        format!("Chase (~${mid:.2} mid)")
    } else {
        "Chase (no mid data)".to_string()
    };

    text(chase_info)
        .size(11)
        .color(theme.extended_palette().background.weak.text)
}
