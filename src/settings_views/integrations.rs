use crate::app_state::TradingTerminal;
use crate::config::ReadDataProvider;
use crate::helpers;
use crate::message::Message;
use iced::widget::{button, checkbox, column, pick_list, row, rule, text, text_input};
use iced::{Alignment, Element, Fill, Length};

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
        let openrouter_status = if self.openrouter_api_key.trim().is_empty() {
            "Not configured"
        } else {
            "Configured"
        };
        let openrouter_status_color = if self.openrouter_api_key.trim().is_empty() {
            current_theme.palette().danger
        } else {
            current_theme.palette().success
        };
        let schwab_status = if self.schwab.has_access_token() {
            "Connected"
        } else if self.schwab.has_refresh_credentials() {
            "Refresh ready"
        } else {
            "Not configured"
        };
        let schwab_status_color =
            if self.schwab.has_access_token() || self.schwab.has_refresh_credentials() {
                current_theme.palette().success
            } else {
                current_theme.palette().danger
            };
        let read_provider_status = match self.read_data_provider {
            ReadDataProvider::Hyperliquid => "Native Hyperliquid",
            ReadDataProvider::Hydromancer if self.hydromancer_api_key.trim().is_empty() => {
                "Fallback: key missing"
            }
            ReadDataProvider::Hydromancer => "Hydromancer",
        };
        let read_provider_status_color = match self.read_data_provider {
            ReadDataProvider::Hyperliquid => current_theme.extended_palette().background.weak.text,
            ReadDataProvider::Hydromancer if self.hydromancer_api_key.trim().is_empty() => {
                current_theme.palette().danger
            }
            ReadDataProvider::Hydromancer => current_theme.palette().success,
        };
        let hydromancer_key_configured = !self.hydromancer_api_key.trim().is_empty();
        let realtime_position_pnl_status = if hydromancer_key_configured {
            "Uses Hydromancer l2Book ticks, matching Tick chart book-mid prices for open-position mark, value, uPnL, and total PnL."
        } else {
            "Save a Hydromancer API key to enable real-time open-position PnL ticks."
        };
        let realtime_position_pnl_toggle = hydromancer_key_configured
            .then_some(Message::ToggleHydromancerRealtimePositionPnl as fn(bool) -> Message);

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
                        .on_input(|value| Message::HydromancerKeyInputChanged(value.into()))
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
                text("Enables Liquidations and Wallet Tracker")
                    .size(11)
                    .color(current_theme.extended_palette().background.weak.text),
                row![
                    text("Read data provider")
                        .size(12)
                        .color(current_theme.palette().text)
                        .width(Fill),
                    pick_list(
                        ReadDataProvider::ALL.to_vec(),
                        Some(self.read_data_provider),
                        Message::ReadDataProviderChanged,
                    )
                    .padding([4, 8])
                    .text_size(12)
                    .width(Length::Fixed(160.0)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                text(read_provider_status)
                    .size(11)
                    .color(read_provider_status_color),
                checkbox(self.hydromancer_realtime_position_pnl_enabled)
                    .label("Real-time position PnL from Hydromancer ticks")
                    .on_toggle_maybe(realtime_position_pnl_toggle)
                    .size(12)
                    .spacing(8)
                    .text_size(12),
                text(realtime_position_pnl_status)
                    .size(11)
                    .color(if hydromancer_key_configured {
                        current_theme.extended_palette().background.weak.text
                    } else {
                        current_theme.palette().warning
                    }),
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
                        .on_input(|value| Message::HyperdashKeyInputChanged(value.into()))
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
            rule::horizontal(1),
            column![
                row![
                    text("OpenRouter")
                        .size(14)
                        .color(current_theme.palette().text)
                        .width(Fill),
                    text(openrouter_status)
                        .size(12)
                        .color(openrouter_status_color),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    text_input("OpenRouter API key", &self.openrouter_key_input)
                        .style(helpers::text_input_style)
                        .on_input(|value| Message::OpenRouterKeyInputChanged(value.into()))
                        .on_submit(Message::SaveOpenRouterKey)
                        .secure(true)
                        .size(12)
                        .padding(6)
                        .width(Fill),
                    button(text("Save").size(12))
                        .padding([6, 12])
                        .on_press(Message::SaveOpenRouterKey),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
                row![
                    text("Default model")
                        .size(12)
                        .color(current_theme.palette().text)
                        .width(Fill),
                    text_input(
                        crate::openrouter_api::DEFAULT_OPENROUTER_MODEL,
                        &self.openrouter_model
                    )
                    .style(helpers::text_input_style)
                    .on_input(Message::OpenRouterModelChanged)
                    .size(12)
                    .padding(6)
                    .width(Length::Fixed(240.0)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                text(
                    self.openrouter_key_status
                        .as_ref()
                        .map(|(message, _)| message.as_str())
                        .unwrap_or("Enables AI summaries for news and TradFi filings")
                )
                .size(11)
                .color(
                    self.openrouter_key_status
                        .as_ref()
                        .map(|(_, is_error)| if *is_error {
                            current_theme.palette().danger
                        } else {
                            current_theme.extended_palette().background.weak.text
                        })
                        .unwrap_or(current_theme.extended_palette().background.weak.text)
                ),
            ]
            .spacing(8),
            rule::horizontal(1),
            column![
                row![
                    text("Schwab")
                        .size(14)
                        .color(current_theme.palette().text)
                        .width(Fill),
                    text(schwab_status).size(12).color(schwab_status_color),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    text_input("Schwab app key", &self.schwab.client_id_input)
                        .style(helpers::text_input_style)
                        .on_input(|value| Message::SchwabClientIdChanged(value.into()))
                        .on_submit(Message::SchwabConnect)
                        .secure(true)
                        .size(12)
                        .padding(6)
                        .width(Length::FillPortion(1)),
                    text_input("Schwab app secret", &self.schwab.client_secret_input)
                        .style(helpers::text_input_style)
                        .on_input(|value| Message::SchwabClientSecretChanged(value.into()))
                        .on_submit(Message::SchwabConnect)
                        .secure(true)
                        .size(12)
                        .padding(6)
                        .width(Length::FillPortion(1)),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
                row![
                    text_input("Schwab refresh token", &self.schwab.refresh_token_input)
                        .style(helpers::text_input_style)
                        .on_input(|value| Message::SchwabRefreshTokenChanged(value.into()))
                        .on_submit(Message::SchwabConnect)
                        .secure(true)
                        .size(12)
                        .padding(6)
                        .width(Length::FillPortion(2)),
                    text_input("Schwab access token", &self.schwab.access_token_input)
                        .style(helpers::text_input_style)
                        .on_input(|value| Message::SchwabAccessTokenChanged(value.into()))
                        .on_submit(Message::SchwabConnect)
                        .secure(true)
                        .size(12)
                        .padding(6)
                        .width(Length::FillPortion(2)),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
                row![
                    button(text(if self.schwab.loading() { "Connecting" } else { "Connect" }).size(12))
                        .padding([6, 12])
                        .on_press(Message::SchwabConnect),
                    button(text("Refresh Accounts").size(12))
                        .padding([6, 12])
                        .on_press(Message::SchwabAccountsRefresh),
                    button(text("Clear").size(12))
                        .padding([6, 12])
                        .on_press(Message::SchwabClearCredentials),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
                text(
                    self.schwab
                        .status
                        .as_ref()
                        .map(|(message, _)| message.as_str())
                        .unwrap_or("Use your own Schwab developer app credentials. Schwab trading is disabled in this build.")
                )
                .size(11)
                .color(
                    self.schwab
                        .status
                        .as_ref()
                        .map(|(_, is_error)| if *is_error {
                            current_theme.palette().danger
                        } else {
                            current_theme.extended_palette().background.weak.text
                        })
                        .unwrap_or(current_theme.extended_palette().background.weak.text)
                ),
            ]
            .spacing(8),
        ]
        .spacing(12)
        .into()
    }
}
