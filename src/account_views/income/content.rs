#[path = "content/summary.rs"]
mod summary;
#[path = "content/tables.rs"]
mod tables;

use super::projection::projected_income_bars;
use super::rows::{view_income_hourly_rows, view_income_token_rows};
use crate::account_analytics::IncomeSnapshot;
use crate::account_views::portfolio::tokens;
use crate::app_state::TradingTerminal;
use crate::app_time::utc_datetime_from_unix_ms;
use crate::message::Message;
use crate::portfolio_state::{IncomePaneView, IncomeProjectionChart};
use iced::widget::{Space, canvas, column, container, responsive, row, rule, scrollable, text};
use iced::{Element, Fill};

const PROJECTION_CHART_HEIGHT: f32 = 150.0;
const COMPACT_TABLE_BREAKPOINT: f32 = 470.0;

impl TradingTerminal {
    pub(super) fn view_income_data<'a>(&'a self, data: &'a IncomeSnapshot) -> Element<'a, Message> {
        responsive(move |size| self.view_income_data_sized(data, size.width))
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn view_income_data_sized<'a>(
        &'a self,
        data: &'a IncomeSnapshot,
        available_width: f32,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let denomination = self.display_denomination_context();
        let compact_table = available_width < COMPACT_TABLE_BREAKPOINT;

        let body = match self.income.view {
            IncomePaneView::Overview => {
                let projection_bars =
                    projected_income_bars(data, utc_datetime_from_unix_ms(self.status_bar_now_ms));
                let chart = canvas(IncomeProjectionChart {
                    bars: projection_bars,
                    denomination: denomination.clone(),
                })
                .width(Fill)
                .height(PROJECTION_CHART_HEIGHT);

                let chart_header = row![
                    text("12-month projection")
                        .size(12)
                        .font(tokens::mono())
                        .color(tokens::text(&theme)),
                    text("at current rates")
                        .size(10)
                        .font(tokens::mono())
                        .color(tokens::dim(&theme)),
                    Space::new().width(Fill),
                ]
                .spacing(7)
                .align_y(iced::Alignment::Center);

                column![
                    summary::income_hero(data, &denomination, &theme),
                    summary::hairline(&theme),
                    summary::income_windows(data, &denomination, &theme),
                    summary::hairline(&theme),
                    summary::income_carry(data, &denomination, &theme, compact_table),
                    summary::hairline(&theme),
                    chart_header,
                    chart,
                ]
                .spacing(9)
                .width(Fill)
            }
            IncomePaneView::Tokens => column![
                tables::income_token_section_header(data, &denomination, &theme),
                summary::hairline(&theme),
                tables::income_token_table_header(&theme, compact_table),
                view_income_token_rows(&data.token_rows, &denomination, &theme, compact_table,),
            ]
            .spacing(8)
            .width(Fill),
            IncomePaneView::Payments => column![
                tables::income_hourly_section_header(data, &denomination, &theme),
                summary::hairline(&theme),
                tables::income_hourly_table_header(&theme, compact_table),
                view_income_hourly_rows(
                    &data.recent_hourly_payments,
                    &denomination,
                    &theme,
                    compact_table,
                ),
            ]
            .spacing(8)
            .width(Fill),
        };

        let mut body = column![body].spacing(8).width(Fill);
        if data.invalid_token_rows > 0 || data.invalid_interest_rows > 0 {
            let mut skipped = Vec::new();
            if data.invalid_token_rows > 0 {
                skipped.push(format!("{} token rows", data.invalid_token_rows));
            }
            if data.invalid_interest_rows > 0 {
                skipped.push(format!("{} interest rows", data.invalid_interest_rows));
            }
            body = body.push(
                text(format!(
                    "Invalid income data skipped: {}",
                    skipped.join(", ")
                ))
                .size(11)
                .font(tokens::mono())
                .color(theme.palette().danger),
            );
        }

        if let Some(err) = &self.income.last_error {
            body = body.push(
                text(format!("Stale data: {err}"))
                    .size(11)
                    .font(tokens::mono())
                    .color(theme.palette().primary),
            );
        }

        let scroll = scrollable(container(body).width(Fill).padding(iced::Padding {
            top: 0.0,
            right: 12.0,
            bottom: 0.0,
            left: 0.0,
        }))
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new()
                .width(4.0)
                .scroller_width(4.0)
                .margin(0.0),
        ))
        .height(Fill);

        column![
            self.view_income_tabs(Some(data)),
            rule::horizontal(1),
            scroll,
        ]
        .spacing(8)
        .width(Fill)
        .height(Fill)
        .into()
    }
}
