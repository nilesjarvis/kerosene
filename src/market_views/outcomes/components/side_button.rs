use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::market_views::outcomes) fn view_outcome_side_button<'a>(
        &'a self,
        theme: &Theme,
        sym: &'a ExchangeSymbol,
        accent: Color,
        is_active: bool,
        mid: Option<f64>,
    ) -> Element<'a, Message> {
        let Some(side_info) = &sym.outcome else {
            return container(Space::new()).into();
        };

        let key = sym.key.clone();
        let probability = outcome_probability_text(mid);
        let condition = side_info.side_condition_short_label();
        let side_label = column![
            text(&side_info.side_name)
                .size(12)
                .color(theme.palette().text),
            text(condition)
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(Fill),
        ]
        .spacing(1)
        .width(Fill);
        let mut side_content = row![
            side_label,
            text(probability)
                .size(16)
                .font(iced::Font::MONOSPACE)
                .color(accent),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);

        if is_active {
            side_content = side_content.push(outcome_chip(
                "ACTIVE",
                theme.palette().primary,
                Color {
                    a: 0.12,
                    ..theme.palette().primary
                },
                Color {
                    a: 0.50,
                    ..theme.palette().primary
                },
            ));
        }

        button(side_content)
            .on_press(Message::SymbolSelected(key))
            .padding([6, 8])
            .width(Fill)
            .style(move |theme: &Theme, status| {
                let background = match status {
                    button::Status::Hovered => Color {
                        a: if is_active { 0.20 } else { 0.14 },
                        ..accent
                    },
                    _ if is_active => Color { a: 0.14, ..accent },
                    _ => theme.extended_palette().background.strong.color,
                };
                let border_color = if is_active {
                    theme.palette().primary
                } else {
                    Color { a: 0.35, ..accent }
                };

                button::Style {
                    background: Some(background.into()),
                    text_color: theme.palette().text,
                    border: iced::Border {
                        radius: 4.0.into(),
                        width: 1.0,
                        color: border_color,
                    },
                    ..Default::default()
                }
            })
            .into()
    }
}

fn outcome_probability_text(mid: Option<f64>) -> String {
    match mid.filter(|value| value.is_finite()) {
        Some(value) => format!("{:.1}%", value * 100.0),
        None => "n/a".to_string(),
    }
}

fn outcome_chip(
    label: impl ToString,
    text_color: Color,
    background: Color,
    border_color: Color,
) -> Element<'static, Message> {
    container(text(label.to_string()).size(9).color(text_color))
        .padding([1, 5])
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(background.into()),
            border: iced::Border {
                radius: 3.0.into(),
                width: 1.0,
                color: border_color,
            },
            ..Default::default()
        })
        .into()
}
