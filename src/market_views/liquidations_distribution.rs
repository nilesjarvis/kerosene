use crate::app_state::TradingTerminal;
use crate::denomination::DisplayDenominationContext;
use crate::helpers;
use crate::liquidations_distribution_state::LiquidationDistributionData;
use crate::message::Message;
use iced::widget::{column, container, responsive, row, text};
use iced::{Alignment, Element, Fill, Length, Theme, color};

mod chart;
mod controls;

use chart::LiquidationsDistributionChart;

// ---------------------------------------------------------------------------
// Liquidations Distribution View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_liquidations_distribution(&self) -> Element<'_, Message> {
        responsive(move |size| self.view_liquidations_distribution_sized(size.width)).into()
    }

    fn view_liquidations_distribution_sized(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let now_ms = self.status_bar_now_ms;
        let denomination = self.display_denomination_context();
        let state = &self.liquidation_distribution;
        let refresh_btn = self.view_liquidations_distribution_refresh_button(state.loading);
        let mut header_actions = row![].spacing(6).align_y(Alignment::Center);
        if state.data.is_some() {
            header_actions = header_actions.push(
                self.view_liquidations_distribution_zoom_controls(state.zoom_factor(), &theme),
            );
        }
        header_actions = header_actions.push(refresh_btn);

        let header = row![
            container(self.view_liquidations_distribution_symbol_button(&theme)).width(Fill),
            header_actions,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let mut content = column![header].spacing(8);
        if state.symbol_picker_open {
            content = content.push(self.view_liquidations_distribution_symbol_dropdown(&theme));
        }

        if state.loading && state.data.is_none() {
            content = content.push(
                row![
                    self.view_spinner(18),
                    text("Loading HyperDash liquidation levels")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }

        if let Some(error) = &state.error {
            content = content.push(text(error.as_str()).size(11).color(color!(0xff5555)));
            if state.data.is_some() {
                content = content.push(
                    text("Showing last successful snapshot")
                        .size(11)
                        .color(theme.extended_palette().background.weak.text),
                );
            }
        }

        if let Some(data) = &state.data {
            content = content
                .push(self.view_liquidations_distribution_metrics(
                    data,
                    now_ms,
                    available_width,
                    &denomination,
                    &theme,
                ))
                .push(
                    iced::widget::canvas(LiquidationsDistributionChart {
                        data,
                        denomination,
                        zoom: state.zoom_factor(),
                        zoom_center_price: state.zoom_center_price,
                    })
                    .width(Fill)
                    .height(Fill),
                );
        } else if !state.loading {
            content = content.push(
                container(
                    text("No liquidation distribution loaded")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                )
                .width(Fill)
                .height(Fill)
                .center(Fill),
            );
        }

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }

    fn view_liquidations_distribution_metrics(
        &self,
        data: &LiquidationDistributionData,
        now_ms: u64,
        available_width: f32,
        denomination: &DisplayDenominationContext,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let total = data.total_long_usd + data.total_short_usd;
        let updated = helpers::format_relative_time(data.fetched_at_ms, now_ms);
        let first_row = row![
            metric_block("Mark", denomination.format_price(data.request.mark), theme),
            metric_block(
                "Longs",
                denomination.format_value(data.total_long_usd, 0),
                theme
            ),
            metric_block(
                "Shorts",
                denomination.format_value(data.total_short_usd, 0),
                theme
            ),
        ]
        .spacing(8)
        .width(Fill);
        let second_row = row![
            metric_block("Total", denomination.format_value(total, 0), theme),
            metric_block("Levels", data.raw_count.to_string(), theme),
            metric_block("Updated", updated, theme),
        ]
        .spacing(8)
        .width(Fill);

        if available_width < 520.0 {
            column![first_row, second_row].spacing(6).width(Fill).into()
        } else {
            row![first_row, second_row].spacing(8).width(Fill).into()
        }
    }
}

fn metric_block(label: &'static str, value: String, theme: &Theme) -> Element<'static, Message> {
    column![
        text(label)
            .size(10)
            .font(crate::app_fonts::monospace_font())
            .color(theme.extended_palette().background.weak.text),
        text(value)
            .size(12)
            .font(crate::app_fonts::monospace_font())
            .color(theme.palette().text),
    ]
    .spacing(2)
    .width(Length::FillPortion(1))
    .into()
}
