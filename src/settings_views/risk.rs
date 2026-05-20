mod muted;

use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::Element;
use iced::widget::{Column, button, column, pick_list, row, rule, text, text_input};

impl TradingTerminal {
    pub(crate) fn view_settings_risk_section(&self) -> Element<'_, Message> {
        let current_theme = self.theme();
        let mut risk_section = Column::new()
            .spacing(12)
            .push(text("Risk").size(16).color(current_theme.palette().text))
            .push(rule::horizontal(1))
            .push(self.view_market_universe_picker())
            .push(rule::horizontal(1))
            .push(self.view_display_denomination_picker())
            .push(rule::horizontal(1))
            .push(self.view_market_slippage_input())
            .push(rule::horizontal(1))
            .push(self.view_muted_ticker_input())
            .push(rule::horizontal(1))
            .push(self.view_muted_ticker_list());

        if let Some((status, is_error)) = &self.muted_ticker_status {
            risk_section = risk_section.push(text(status).size(11).color(if *is_error {
                current_theme.palette().danger
            } else {
                current_theme.extended_palette().background.weak.text
            }));
        }

        risk_section.into()
    }

    fn view_market_universe_picker(&self) -> Element<'_, Message> {
        let current_theme = self.theme();
        let mut options = self.market_universe_options();
        if !options.contains(&self.market_universe) {
            options.push(self.market_universe.clone());
        }

        column![
            text("Market Universe")
                .size(14)
                .color(current_theme.palette().text),
            row![
                pick_list(
                    options,
                    Some(self.market_universe.clone()),
                    Message::MarketUniverseChanged,
                )
                .padding([4, 8])
                .text_size(12)
                .width(iced::Length::Fixed(220.0)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            text("Restricts visible symbols, account rows, orders, charts, and watchlists.")
                .size(11)
                .color(current_theme.extended_palette().background.weak.text),
        ]
        .spacing(8)
        .into()
    }

    fn view_display_denomination_picker(&self) -> Element<'_, Message> {
        let current_theme = self.theme();
        let mut options = self.display_denomination_options();
        if !options.contains(&self.display_denomination) {
            options.push(self.display_denomination.clone());
        }
        let status = self.display_denomination_status();

        let mut content = column![
            text("Display Denomination")
                .size(14)
                .color(current_theme.palette().text),
            row![
                pick_list(
                    options,
                    Some(self.display_denomination.clone()),
                    Message::DisplayDenominationChanged,
                )
                .padding([4, 8])
                .text_size(12)
                .width(iced::Length::Fixed(120.0)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            text(
                "Converts read-only USD values. Order entry and API payloads stay native USD/USDC."
            )
            .size(11)
            .color(current_theme.extended_palette().background.weak.text),
        ]
        .spacing(8);

        if let Some(status) = status {
            content = content.push(text(status).size(11).color(current_theme.palette().warning));
        }

        content.into()
    }

    fn view_market_slippage_input(&self) -> Element<'_, Message> {
        let current_theme = self.theme();

        column![
            text("Market Order Slippage")
                .size(14)
                .color(current_theme.palette().text),
            row![
                text_input("0.0 - 20.0", &self.market_slippage_input)
                    .style(helpers::text_input_style)
                    .on_input(Message::MarketSlippageInputChanged)
                    .on_submit(Message::SaveMarketSlippage)
                    .size(12)
                    .padding(6)
                    .width(iced::Length::Fixed(120.0)),
                text("%")
                    .size(12)
                    .color(current_theme.extended_palette().background.weak.text),
                button(text("Save").size(12))
                    .padding([6, 12])
                    .on_press(Message::SaveMarketSlippage),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            text(format!(
                "Current limit price offset for market orders: {:.2}%",
                self.market_slippage_pct
            ))
            .size(11)
            .color(current_theme.extended_palette().background.weak.text),
        ]
        .spacing(8)
        .into()
    }
}
