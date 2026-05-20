#[path = "content/summary.rs"]
mod summary;
#[path = "content/tables.rs"]
mod tables;

use super::projection::projected_income_bars;
use super::rows::{view_income_hourly_rows, view_income_token_rows};
use crate::account_analytics::IncomeSnapshot;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::portfolio_state::IncomeProjectionChart;
use iced::widget::{canvas, column, container, scrollable, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(super) fn view_income_data<'a>(&'a self, data: &'a IncomeSnapshot) -> Element<'a, Message> {
        let theme = self.theme();
        let denomination = self.display_denomination_context();
        let projection_bars = projected_income_bars(data);

        let chart = canvas(IncomeProjectionChart {
            bars: projection_bars,
            denomination: denomination.clone(),
        })
        .width(Fill)
        .height(180);

        let token_rows = view_income_token_rows(&data.token_rows, &denomination, &theme);
        let hourly_rows =
            view_income_hourly_rows(&data.recent_hourly_payments, &denomination, &theme);

        let mut content = column![
            self.view_income_title(),
            summary::income_earned_total_row(data, &denomination, &theme),
            summary::income_earned_windows_row(data, &denomination),
            summary::income_interest_note(),
            summary::income_carrying_top_row(data, &denomination),
            summary::income_carrying_bottom_row(data),
            text("Projected Interest By Upcoming Month (current rates)")
                .size(11)
                .color(theme.palette().text),
            chart,
            text("Token Contributions (annualized)")
                .size(11)
                .color(theme.palette().text),
            tables::income_token_table_header(),
            token_rows,
            text("Recent Hourly Interest Payments")
                .size(11)
                .color(theme.palette().text),
            tables::income_hourly_table_header(),
            hourly_rows,
        ]
        .spacing(8);

        if data.invalid_token_rows > 0 || data.invalid_interest_rows > 0 {
            let mut skipped = Vec::new();
            if data.invalid_token_rows > 0 {
                skipped.push(format!("{} token rows", data.invalid_token_rows));
            }
            if data.invalid_interest_rows > 0 {
                skipped.push(format!("{} interest rows", data.invalid_interest_rows));
            }
            content = content.push(
                text(format!(
                    "Invalid income data skipped: {}",
                    skipped.join(", ")
                ))
                .size(11)
                .color(theme.palette().danger),
            );
        }

        if let Some(err) = &self.income.last_error {
            content = content.push(
                text(format!("Stale data: {err}"))
                    .size(11)
                    .color(theme.palette().primary),
            );
        }

        scrollable(container(content).width(Fill).padding(iced::Padding {
            top: 0.0,
            right: 15.0,
            bottom: 0.0,
            left: 0.0,
        }))
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new()
                .width(4.0)
                .scroller_width(4.0)
                .margin(0.0),
        ))
        .height(Fill)
        .into()
    }
}
