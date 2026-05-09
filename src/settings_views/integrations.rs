use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::{button, column, row, rule, text, text_input};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_settings_integrations_section(&self) -> Element<'_, Message> {
        let current_theme = self.theme();
        let hydromancer_status = if self.hydromancer_api_key.trim().is_empty() {
            "Not configured"
        } else {
            "Configured"
        };
        let hydromancer_status_color = if self.hydromancer_api_key.trim().is_empty() {
            current_theme.palette().danger
        } else {
            current_theme.palette().success
        };
        let hyperdash_status = if self.hyperdash_api_key.trim().is_empty() {
            "Not configured"
        } else {
            "Configured"
        };
        let hyperdash_status_color = if self.hyperdash_api_key.trim().is_empty() {
            current_theme.palette().danger
        } else {
            current_theme.palette().success
        };

        column![
            text("Integrations")
                .size(16)
                .color(current_theme.palette().text),
            rule::horizontal(1),
            column![
                row![
                    text("Hydromancer")
                        .size(14)
                        .color(current_theme.palette().text)
                        .width(Fill),
                    text(hydromancer_status)
                        .size(12)
                        .color(hydromancer_status_color),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    text_input("Hydromancer API key", &self.hydromancer_key_input)
                        .style(helpers::text_input_style)
                        .on_input(Message::HydromancerKeyInputChanged)
                        .on_submit(Message::SaveHydromancerKey)
                        .secure(true)
                        .size(12)
                        .padding(6)
                        .width(Fill),
                    button(text("Save").size(12))
                        .padding([6, 12])
                        .on_press(Message::SaveHydromancerKey),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
                text("Enables Liquidations and Tracked Trades")
                    .size(11)
                    .color(current_theme.extended_palette().background.weak.text),
            ]
            .spacing(8),
            rule::horizontal(1),
            column![
                row![
                    text("HyperDash")
                        .size(14)
                        .color(current_theme.palette().text)
                        .width(Fill),
                    text(hyperdash_status)
                        .size(12)
                        .color(hyperdash_status_color),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    text_input("HyperDash API key", &self.hyperdash_key_input)
                        .style(helpers::text_input_style)
                        .on_input(Message::HyperdashKeyInputChanged)
                        .on_submit(Message::SaveHyperdashKey)
                        .secure(true)
                        .size(12)
                        .padding(6)
                        .width(Fill),
                    button(text("Save").size(12))
                        .padding([6, 12])
                        .on_press(Message::SaveHyperdashKey),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
                text("Enables LIQ and HEAT on perp charts")
                    .size(11)
                    .color(current_theme.extended_palette().background.weak.text),
            ]
            .spacing(8),
        ]
        .spacing(12)
        .into()
    }
}
