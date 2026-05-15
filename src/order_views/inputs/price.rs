use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::signing::OrderKind;

use iced::Theme;
use iced::widget::{Column, button, row, text, text_input};

impl TradingTerminal {
    pub(super) fn push_price_input_controls<'a>(
        &'a self,
        form: Column<'a, Message>,
    ) -> Column<'a, Message> {
        let theme = self.theme();
        if !matches!(self.order_kind, OrderKind::Limit | OrderKind::LimitIoc) {
            return form.push(market_price_label(
                self.resolve_mid_for_symbol(&self.active_symbol),
                self.market_slippage_pct,
                &theme,
            ));
        }

        let price_input = text_input("Price", &self.order_price)
            .style(helpers::text_input_style)
            .on_input(Message::OrderPriceChanged)
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

        form.push(
            text("Price")
                .size(12)
                .color(theme.extended_palette().background.weak.text),
        )
        .push(price_row)
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
