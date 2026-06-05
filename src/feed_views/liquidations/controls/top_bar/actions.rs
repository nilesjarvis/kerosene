use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::{button, row, text, text_input};
use iced::{Element, Theme};

impl TradingTerminal {
    pub(in crate::feed_views::liquidations::controls) fn view_liquidation_threshold_controls(
        &self,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let threshold_input = text_input("Min $", &self.liquidation_alert_input)
            .style(helpers::text_input_style)
            .on_input(Message::LiquidationAlertThresholdChanged)
            .on_submit(Message::SaveLiquidationAlertThreshold)
            .size(10)
            .width(60)
            .padding([2, 4]);

        row![
            text(">$")
                .size(10)
                .color(theme.extended_palette().background.weak.text),
            threshold_input,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }

    pub(in crate::feed_views::liquidations::controls) fn view_clear_liquidations_button(
        &self,
    ) -> Element<'static, Message> {
        let corner_radius = self.pane_corner_radius;
        button(text("Clear").size(10).center())
            .on_press(Message::ClearLiquidations)
            .padding([2, 6])
            .style(move |theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ => theme.extended_palette().background.weak.color,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().text,
                    border: iced::Border {
                        radius: crate::config::effective_radius(corner_radius, 3.0).into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }
}
