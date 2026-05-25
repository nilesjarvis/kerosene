use crate::account;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pnl_card::{PnlCardTarget, pnl_card_icon_button};

use iced::widget::{container, row, text};
use iced::{Alignment, Color, Element, Fill, Theme};

use super::super::PositionNumberMode;

mod account_value;
mod formatting;
mod totals;

#[cfg(test)]
mod tests;

use formatting::*;
use totals::*;

impl TradingTerminal {
    pub(in crate::account_views::positions) fn view_position_summary_bar<'a>(
        &'a self,
        positions: &[&'a account::AssetPosition],
        theme: &Theme,
        number_mode: PositionNumberMode,
    ) -> Element<'a, Message> {
        let totals =
            PositionSummaryTotals::from_rows(positions.iter().map(|ap| self.position_row_data(ap)));
        let weak_text = theme.extended_palette().background.weak.text;
        let neutral_text = theme.palette().text;
        let long_color = theme.palette().success;
        let short_color = theme.palette().danger;
        let account_balance = self
            .account_data
            .as_ref()
            .and_then(|data| self.position_summary_account_value(data));
        let total_pnl_pct = position_total_pnl_percent(totals.total_pnl, account_balance);
        let denomination = self.display_denomination_context();

        let summary = row![
            summary_cell(
                "Funding",
                format_optional_unsigned_display(
                    &denomination,
                    totals.funding_gross,
                    self.hide_pnl,
                    number_mode,
                ),
                weak_text,
                neutral_text,
            ),
            summary_cell(
                "Long Ntl",
                format_unsigned_display(
                    &denomination,
                    totals.long_notional,
                    self.hide_pnl,
                    number_mode,
                ),
                weak_text,
                long_color,
            ),
            summary_cell(
                "Short Ntl",
                format_unsigned_display(
                    &denomination,
                    totals.short_notional,
                    self.hide_pnl,
                    number_mode,
                ),
                weak_text,
                short_color,
            ),
            summary_cell(
                "Net Fund",
                format_optional_signed_display(
                    &denomination,
                    totals.net_funding,
                    self.hide_pnl,
                    number_mode,
                ),
                weak_text,
                totals
                    .net_funding
                    .value()
                    .map(|value| self.direction_color(theme, value))
                    .unwrap_or(weak_text),
            ),
            summary_cell_with_action(
                "uPnL",
                format_optional_signed_display(
                    &denomination,
                    totals.upnl,
                    self.hide_pnl,
                    number_mode,
                ),
                weak_text,
                totals
                    .upnl
                    .value()
                    .map(|value| self.direction_color(theme, value))
                    .unwrap_or(weak_text),
                totals
                    .upnl
                    .value()
                    .map(|_| Message::OpenPnlCard(PnlCardTarget::Summary)),
            ),
            summary_cell(
                "Total PnL",
                format_optional_total_pnl_display(
                    &denomination,
                    totals.total_pnl,
                    total_pnl_pct,
                    self.hide_pnl,
                    number_mode,
                ),
                weak_text,
                totals
                    .total_pnl
                    .value()
                    .map(|value| self.direction_color(theme, value))
                    .unwrap_or(weak_text),
            ),
        ]
        .spacing(4)
        .align_y(Alignment::Center);

        container(summary)
            .width(Fill)
            .padding([4, 8])
            .style(|theme: &Theme| {
                let mut background = theme.extended_palette().background.weak.color;
                background.a = 0.20;
                iced::widget::container::Style {
                    background: Some(background.into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        width: 1.0,
                        color: theme.extended_palette().background.strong.color,
                    },
                    ..Default::default()
                }
            })
            .into()
    }
}

fn summary_cell(
    label: &'static str,
    value: String,
    label_color: Color,
    value_color: Color,
) -> Element<'static, Message> {
    container(
        row![
            text(label).size(10).color(label_color),
            text(value)
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(value_color),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .into()
}

fn summary_cell_with_action(
    label: &'static str,
    value: String,
    label_color: Color,
    value_color: Color,
    action: Option<Message>,
) -> Element<'static, Message> {
    container(
        row![
            text(label).size(10).color(label_color),
            text(value)
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(value_color),
            pnl_card_icon_button(action, "Open summary PnL card"),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .into()
}
