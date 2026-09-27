use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::{Message, RedactedOrderInput};
use crate::order_execution::TwapOrderStartSnapshot;
use crate::twap_state::MAX_ACTIVE_ADVANCED_ORDERS;
use iced::widget::{Column, button, checkbox, column, container, row, text, text_input};
use iced::{Alignment, Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// TWAP Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_twap_controls<'a>(
        &'a self,
        form: Column<'a, Message>,
        can_trade: bool,
    ) -> Column<'a, Message> {
        let theme = self.theme();
        let can_start =
            can_trade && self.active_advanced_order_count() < MAX_ACTIVE_ADVANCED_ORDERS;

        let snapshot = self.twap_order_start_snapshot();
        let buy = twap_start_button(
            format!("TWAP BUY {}", self.active_symbol_display.to_uppercase()),
            true,
            theme.palette().success,
            can_start,
            snapshot.clone(),
        );
        let sell = twap_start_button(
            format!("TWAP SELL {}", self.active_symbol_display.to_uppercase()),
            false,
            theme.palette().danger,
            can_start,
            snapshot,
        );

        let mut form = form;

        if can_trade && !can_start {
            form = form.push(
                text(format!(
                    "Maximum of {MAX_ACTIVE_ADVANCED_ORDERS} active advanced orders reached"
                ))
                .size(10)
                .color(theme.palette().danger),
            );
        }

        form.push(row![buy, sell].spacing(8))
    }

    pub(in crate::order_views) fn push_twap_settings<'a>(
        &'a self,
        form: Column<'a, Message>,
    ) -> Column<'a, Message> {
        let theme = self.theme();
        let weak_text = theme.extended_palette().background.weak.text;

        let duration = compact_input(
            "Min",
            &self.twap_form.duration_minutes,
            Message::TwapDurationChanged,
        );
        let slices = compact_input("Slices", &self.twap_form.slices, Message::TwapSlicesChanged);
        let min_price = compact_input(
            "Min px",
            &self.twap_form.min_price,
            Message::TwapMinPriceChanged,
        );
        let max_price = compact_input(
            "Max px",
            &self.twap_form.max_price,
            Message::TwapMaxPriceChanged,
        );

        let settings = column![
            row![
                text("TWAP").size(11).color(weak_text),
                checkbox(self.twap_form.randomize)
                    .label("Randomize")
                    .on_toggle(Message::TwapRandomizeToggled)
                    .size(12)
                    .text_size(11)
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
                .padding([6, 7])
                .width(Fill)
                .style(|theme: &Theme| iced::widget::container::Style {
                    background: Some(theme.extended_palette().background.weak.color.into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        width: 1.0,
                        color: theme.extended_palette().background.strong.color,
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
) -> Element<'a, Message> {
    text_input(placeholder, value)
        .style(helpers::text_input_style)
        .on_input(move |value| message(value.into()))
        .size(12)
        .padding([4, 6])
        .into()
}

fn twap_start_button(
    label: String,
    is_buy: bool,
    accent: Color,
    enabled: bool,
    snapshot: TwapOrderStartSnapshot,
) -> Element<'static, Message> {
    let message = if is_buy {
        Message::StartTwap {
            is_buy: true,
            snapshot,
        }
    } else {
        Message::StartTwap {
            is_buy: false,
            snapshot,
        }
    };

    let mut button = button(text(label).size(10).center().width(Fill))
        .padding([4, 8])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            let bg = if enabled {
                Color {
                    a: if matches!(status, button::Status::Hovered) {
                        0.16
                    } else {
                        0.1
                    },
                    ..accent
                }
            } else {
                theme.extended_palette().background.weak.color
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if enabled {
                    accent
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: Color {
                        a: if enabled { 0.15 } else { 0.05 },
                        ..accent
                    },
                },
                ..Default::default()
            }
        });

    if enabled {
        button = button.on_press(message);
    }
    button.into()
}
