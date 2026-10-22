use crate::app_state::TradingTerminal;
use crate::config::effective_radius;
use crate::helpers;
use crate::helpers::design::{UiColors, compact_button_style};
use crate::market_state::{OrderBookId, OrderBookInstance};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{button, container, row, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::market_views::order_book) fn view_order_book_spread_widget(
        id: OrderBookId,
        inst: &OrderBookInstance,
        theme: &Theme,
        pane_corner_radius: f32,
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
                        .size(16)
                        .style(move |t: &Theme| text::Style {
                            color: Some(UiColors::new(t).muted),
                        })
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
                        .style(move |t: &Theme| text::Style {
                            color: Some(UiColors::new(t).muted),
                        }),
                )
                .width(Fill)
                .align_x(iced::alignment::Horizontal::Center),
                container(center_order_book_button(
                    id,
                    inst.center_on_mid,
                    theme,
                    pane_corner_radius,
                ))
                .width(Fill)
                .align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .width(Fill)
        .padding([3, 0])
        // The mid/spread separator is a neutral inset band anchored by a
        // stronger current price, not a large button (design language).
        .style(move |t: &Theme| container_style::Style {
            background: Some(UiColors::new(t).inset.into()),
            border: iced::Border {
                radius: effective_radius(pane_corner_radius, 3.0).into(),
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
    _theme: &Theme,
    pane_corner_radius: f32,
) -> button::Button<'static, Message> {
    // Independent on/off toggle: the shared compact selected-toggle recipe,
    // so the active state keeps a soft fill plus outline instead of a
    // bespoke accent tint.
    button(text("Center").size(12))
        .on_press(Message::ToggleOrderBookCenterOnMid(id))
        .height(24)
        .padding([2, 8])
        .style(move |t: &Theme, status| {
            compact_button_style(t, status, is_active, pane_corner_radius)
        })
}

/// Current mid price colored by the short-term direction of travel, with an
/// up/down arrow — the at-a-glance price readout between asks and bids.
/// Featured-size (16px) so the mid stays the strongest number in the book.
fn mid_price_readout(
    mid: f64,
    price_move: Option<f64>,
    decimals: usize,
    theme: &Theme,
) -> Element<'static, Message> {
    let colors = UiColors::new(theme);
    let muted = colors.muted;
    let decimals = decimals.min(8);

    let (arrow, direction) = match price_move {
        Some(price_move) if price_move > 0.0 => ("\u{2191} ", colors.positive),
        Some(price_move) if price_move < 0.0 => ("\u{2193} ", colors.negative),
        Some(_) => ("\u{2192} ", muted),
        None => ("", muted),
    };

    text(format!(
        "{arrow}{}",
        helpers::format_decimal_with_commas(mid, decimals)
    ))
    .size(16)
    .font(crate::app_fonts::monospace_font())
    .color(direction)
    .into()
}
