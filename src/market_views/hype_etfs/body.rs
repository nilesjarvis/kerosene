use super::chart::daily_inflow_chart;
use super::sections::{fund_section, summary_section};
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{Column, row, scrollable, text};
use iced::{Element, Fill, color};

// ---------------------------------------------------------------------------
// Body
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_hype_etfs_body(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let denomination = self.display_denomination_context();
        let mut body = Column::new().spacing(8);

        if self.hype_etfs.loading {
            body = body.push(
                row![
                    self.view_spinner(18),
                    text("Loading ETF data")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            );
        }

        if let Some(error) = &self.hype_etfs.error {
            body = body.push(
                text(error.clone())
                    .size(11)
                    .color(color!(0xff5555))
                    .width(Fill),
            );
        }

        if let Some(data) = &self.hype_etfs.data {
            for warning in &data.warnings {
                body = body.push(
                    text(warning.clone())
                        .size(11)
                        .color(color!(0xffb86c))
                        .width(Fill),
                );
            }

            let selected_funds = data.selected_funds(self.hype_etfs.view);
            if selected_funds.is_empty() {
                body = body.push(
                    text("No data returned for this ETF")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                );
            } else {
                body = body.push(summary_section(
                    &theme,
                    self.hype_etfs.view,
                    data.totals_for(self.hype_etfs.view),
                    selected_funds.len(),
                    available_width,
                    &denomination,
                ));

                let daily_flows = data.daily_flows_for(self.hype_etfs.view);
                let missing_flow_labels = selected_funds
                    .iter()
                    .filter(|fund| fund.daily_flows.is_empty())
                    .map(|fund| fund.ticker.label())
                    .collect::<Vec<_>>();
                body = body.push(daily_inflow_chart(
                    &theme,
                    self.hype_etfs.view,
                    &daily_flows,
                    &missing_flow_labels,
                    available_width,
                    &denomination,
                ));

                for fund in selected_funds {
                    body = body.push(fund_section(&theme, fund, available_width, &denomination));
                }
            }
        } else if !self.hype_etfs.loading {
            body = body.push(
                text("No ETF data loaded")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        scrollable(body).height(Fill).into()
    }
}
