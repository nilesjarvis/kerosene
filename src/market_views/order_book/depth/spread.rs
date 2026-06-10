use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::{OrderBookId, OrderBookInstance};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{button, container, row, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::market_views::order_book) fn view_order_book_spread_widget(
        id: OrderBookId,
        inst: &OrderBookInstance,
        theme: &Theme,
    ) -> Element<'static, Message> {
        // Use the same top-of-book the rows render so the spread row never
        // disagrees with the levels directly above and below it.
        let (best_bid, best_ask) = inst.visible_best_bid_ask();
        let (mid_readout, spread_text) =
            if let (Some(best_bid), Some(best_ask)) = (best_bid, best_ask) {
                let spread = best_ask - best_bid;
                let mid = (best_ask + best_bid) / 2.0;
                let spread_pct = if mid > 0.0 { spread / mid * 100.0 } else { 0.0 };
                let spread_decimals = helpers::tick_decimals(helpers::default_tick_for_price(mid));

                (
                    mid_price_readout(mid, inst.short_term_price_move(), spread_decimals, theme),
                    format!(
                        "{:.prec$} ({:.3}%)",
                        spread,
                        spread_pct,
                        prec = spread_decimals
                    ),
                )
            } else {
                // Identical chrome with placeholders, so a transiently
                // one-sided book never changes the row height or hides the
                // Center toggle.
                (
                    text("--")
                        .size(11)
                        .color(theme.extended_palette().background.weak.text)
                        .into(),
                    "-- (--%)".to_string(),
                )
            };

        container(
            row![
                container(mid_readout).width(Fill),
                container(
                    text(spread_text)
                        .size(11)
                        .color(theme.extended_palette().background.weak.text),
                )
                .width(Fill)
                .align_x(iced::alignment::Horizontal::Center),
                container(center_order_book_button(id, inst.center_on_mid, theme))
                    .width(Fill)
                    .align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(6)
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
    }
}

fn center_order_book_button(
    id: OrderBookId,
    is_active: bool,
    theme: &Theme,
) -> button::Button<'static, Message> {
    let text_color = if is_active {
        theme.palette().primary
    } else {
        theme.extended_palette().background.weak.text
    };

    button(text("Center").size(10).color(text_color))
        .padding([2, 4])
        .style(move |theme: &Theme, status| {
            let mut background = if is_active {
                theme.palette().primary
            } else {
                Color::TRANSPARENT
            };
            background.a = match (is_active, status) {
                (true, button::Status::Hovered) => 0.18,
                (true, _) => 0.12,
                (false, button::Status::Hovered) => 0.08,
                (false, _) => 0.0,
            };

            let mut border_color = theme.palette().primary;
            border_color.a = if is_active { 0.45 } else { 0.0 };

            button::Style {
                background: Some(background.into()),
                border: iced::Border {
                    width: 1.0,
                    color: border_color,
                    radius: 2.0.into(),
                },
                ..Default::default()
            }
        })
        .on_press(Message::ToggleOrderBookCenterOnMid(id))
}

/// Current mid price colored by the short-term direction of travel, with an
/// up/down arrow — the at-a-glance price readout between asks and bids.
fn mid_price_readout(
    mid: f64,
    price_move: Option<f64>,
    decimals: usize,
    theme: &Theme,
) -> Element<'static, Message> {
    let weak_text = theme.extended_palette().background.weak.text;
    let decimals = decimals.min(8);

    let (arrow, color) = match price_move {
        Some(price_move) if price_move > 0.0 => ("\u{2191} ", theme.palette().success),
        Some(price_move) if price_move < 0.0 => ("\u{2193} ", theme.palette().danger),
        Some(_) => ("\u{2192} ", weak_text),
        None => ("", weak_text),
    };

    text(format!(
        "{arrow}{}",
        helpers::format_decimal_with_commas(mid, decimals)
    ))
    .size(11)
    .font(crate::app_fonts::monospace_font())
    .color(color)
    .into()
}
