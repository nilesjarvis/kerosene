use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::{OrderBookId, OrderBookInstance};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, button, container, row, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::market_views::order_book) fn view_order_book_spread_widget(
        id: OrderBookId,
        inst: &OrderBookInstance,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let (true_best_bid, true_best_ask) = true_best_prices(inst);
        if let (Some(best_bid), Some(best_ask)) = (true_best_bid, true_best_ask) {
            let spread = best_ask - best_bid;
            let mid = (best_ask + best_bid) / 2.0;
            let spread_pct = if mid > 0.0 { spread / mid * 100.0 } else { 0.0 };
            let spread_decimals = helpers::tick_decimals(helpers::default_tick_for_price(mid));

            container(
                row![
                    text(format!(
                        "Spread: {:.prec$} ({:.3}%)",
                        spread,
                        spread_pct,
                        prec = spread_decimals
                    ))
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
                    Space::new().width(Fill),
                    center_order_book_button(id, theme)
                ]
                .align_y(iced::Alignment::Center),
            )
            .width(Fill)
            .padding([3, 0])
            .style(move |theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.weak.color.into()),
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
        } else {
            container(text("").size(11))
                .width(Fill)
                .padding([3, 0])
                .into()
        }
    }
}

fn center_order_book_button(id: OrderBookId, theme: &Theme) -> button::Button<'static, Message> {
    button(
        text("Center")
            .size(10)
            .color(theme.extended_palette().background.weak.text),
    )
    .padding([2, 4])
    .style(move |_theme: &Theme, _status| button::Style {
        background: Some(Color::TRANSPARENT.into()),
        ..Default::default()
    })
    .on_press(Message::CenterOrderBook(id))
}

fn true_best_prices(inst: &OrderBookInstance) -> (Option<f64>, Option<f64>) {
    let mut true_best_bid = inst.book.bids.first().map(|level| level.px);
    let mut true_best_ask = inst.book.asks.first().map(|level| level.px);

    if let Some(ctx) = &inst.asset_ctx
        && let Some(impact) = &ctx.impact_pxs
        && impact.len() >= 2
        && let (Ok(best_bid), Ok(best_ask)) = (impact[0].parse::<f64>(), impact[1].parse::<f64>())
    {
        true_best_bid = Some(best_bid);
        true_best_ask = Some(best_ask);
    }

    (true_best_bid, true_best_ask)
}
