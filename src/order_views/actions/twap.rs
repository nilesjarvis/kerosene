use crate::app_state::TradingTerminal;
use crate::helpers::design::{UiColors, compact_input_style};
use crate::helpers::{TradeAction, TradeSide, trade_action_button};
use crate::message::{Message, RedactedOrderInput};
use crate::twap_state::MAX_ACTIVE_ADVANCED_ORDERS;
use iced::widget::{Column, checkbox, column, container, row, text, text_input};
use iced::{Alignment, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// TWAP Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_twap_controls<'a>(
        &'a self,
        form: Column<'a, Message>,
        can_trade: bool,
    ) -> Column<'a, Message> {
        let can_start =
            can_trade && self.active_advanced_order_count() < MAX_ACTIVE_ADVANCED_ORDERS;

        let pane_corner_radius = self.pane_corner_radius;
        let symbol = self.active_symbol_display.to_uppercase();
        let snapshot = self.twap_order_start_snapshot();

        // Starting a TWAP is the same prominent trade-action family as the
        // ticket's buy/sell and chase starts; unavailable stays quiet.
        let buy = trade_action_button(
            TradeSide::Buy,
            format!("TWAP BUY {symbol}"),
            if can_start {
                TradeAction::Place(Box::new(Message::StartTwap {
                    is_buy: true,
                    snapshot: snapshot.clone(),
                }))
            } else {
                TradeAction::Unavailable
            },
            None,
            pane_corner_radius,
        );
        let sell = trade_action_button(
            TradeSide::Sell,
            format!("TWAP SELL {symbol}"),
            if can_start {
                TradeAction::Place(Box::new(Message::StartTwap {
                    is_buy: false,
                    snapshot,
                }))
            } else {
                TradeAction::Unavailable
            },
            None,
            pane_corner_radius,
        );

        let mut form = form;

        if can_trade && !can_start {
            form = form.push(
                text(format!(
                    "Maximum of {MAX_ACTIVE_ADVANCED_ORDERS} active advanced orders reached"
                ))
                .size(11)
                .style(move |theme: &Theme| text::Style {
                    color: Some(UiColors::new(theme).negative),
                }),
            );
        }

        form.push(row![buy, sell].spacing(8))
    }

    pub(in crate::order_views) fn push_twap_settings<'a>(
        &'a self,
        form: Column<'a, Message>,
    ) -> Column<'a, Message> {
        let pane_corner_radius = self.pane_corner_radius;

        let duration = compact_input(
            "Min",
            &self.twap_form.duration_minutes,
            Message::TwapDurationChanged,
            pane_corner_radius,
        );
        let slices = compact_input(
            "Slices",
            &self.twap_form.slices,
            Message::TwapSlicesChanged,
            pane_corner_radius,
        );
        let min_price = compact_input(
            "Min px",
            &self.twap_form.min_price,
            Message::TwapMinPriceChanged,
            pane_corner_radius,
        );
        let max_price = compact_input(
            "Max px",
            &self.twap_form.max_price,
            Message::TwapMaxPriceChanged,
            pane_corner_radius,
        );

        let settings = column![
            row![
                text("TWAP")
                    .size(11)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(UiColors::new(theme).muted),
                    }),
                checkbox(self.twap_form.randomize)
                    .label("Randomize")
                    .on_toggle(Message::TwapRandomizeToggled)
                    .size(12)
                    .text_size(12)
                    .text_shaping(iced::widget::text::Shaping::Advanced)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![duration, slices].spacing(6),
            row![min_price, max_price].spacing(6),
        ]
        .spacing(5);

        form.push(
            container(settings)
                .padding(8)
                .width(Fill)
                .style(move |theme: &Theme| iced::widget::container::Style {
                    background: Some(UiColors::new(theme).inset.into()),
                    border: iced::Border {
                        radius: crate::config::effective_radius(pane_corner_radius, 6.0).into(),
                        width: 1.0,
                        color: UiColors::new(theme).divider,
                    },
                    ..Default::default()
                }),
        )
    }
}

fn compact_input<'a>(
    placeholder: &'static str,
    value: &'a str,
    message: fn(RedactedOrderInput) -> Message,
    pane_corner_radius: f32,
) -> Element<'a, Message> {
    // Sensitive order input: the shared field frame keeps the redacted input
    // wrapper; only the visual shell changes.
    text_input(placeholder, value)
        .on_input(move |value| message(value.into()))
        .size(12)
        .line_height(iced::widget::text::LineHeight::Absolute(iced::Pixels(16.0)))
        .padding([6, 8])
        .style(move |theme, status| compact_input_style(theme, status, false, pane_corner_radius))
        .into()
}
