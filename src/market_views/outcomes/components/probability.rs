use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Space, container, row};
use iced::{Color, Element, Fill, Theme, color};

impl TradingTerminal {
    pub(in crate::market_views::outcomes) fn view_outcome_probability_bar(
        first_mid: Option<f64>,
        second_mid: Option<f64>,
        first_color: Color,
        second_color: Color,
    ) -> Element<'static, Message> {
        let (first_portion, second_portion) = outcome_probability_portions(first_mid, second_mid);
        let has_mid = first_mid.is_some_and(|value| value.is_finite())
            || second_mid.is_some_and(|value| value.is_finite());
        let first_bar = if has_mid {
            Color {
                a: 0.80,
                ..first_color
            }
        } else {
            color!(0x666666)
        };
        let second_bar = if has_mid {
            Color {
                a: 0.80,
                ..second_color
            }
        } else {
            color!(0x444444)
        };

        container(
            row![
                container(Space::new())
                    .width(iced::Length::FillPortion(first_portion))
                    .height(3.0)
                    .style(move |_theme: &Theme| container_style::Style {
                        background: Some(first_bar.into()),
                        ..Default::default()
                    }),
                container(Space::new())
                    .width(iced::Length::FillPortion(second_portion))
                    .height(3.0)
                    .style(move |_theme: &Theme| container_style::Style {
                        background: Some(second_bar.into()),
                        ..Default::default()
                    }),
            ]
            .width(Fill)
            .height(3.0),
        )
        .width(Fill)
        .height(3.0)
        .into()
    }
}

fn outcome_probability_portions(first_mid: Option<f64>, second_mid: Option<f64>) -> (u16, u16) {
    let first = first_mid.filter(|value| value.is_finite() && *value >= 0.0);
    let second = second_mid.filter(|value| value.is_finite() && *value >= 0.0);
    let first_ratio = match (first, second) {
        (Some(first), Some(second)) if first + second > 0.0 => first / (first + second),
        (Some(first), _) => first.clamp(0.0, 1.0),
        (_, Some(second)) => 1.0 - second.clamp(0.0, 1.0),
        _ => 0.5,
    };
    let first_portion = (first_ratio * 1000.0).round().clamp(1.0, 999.0) as u16;
    (first_portion, 1000 - first_portion)
}
