use crate::account_metrics::format_signed_usd_value;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, column, row, text};
use iced::{Fill, Theme, color};

use super::format_signed_percent_value;

impl TradingTerminal {
    pub(super) fn view_daily_pnl_list(&self, theme: &Theme) -> Column<'static, Message> {
        let daily_rows = if self.hide_pnl {
            let daily_source_points = self
                .daily_source_portfolio_bucket()
                .map(|bucket| {
                    (
                        bucket.pnl_history.clone(),
                        bucket.account_value_history.clone(),
                    )
                })
                .unwrap_or_default();
            Self::compute_daily_percent_rows_from_cumulative(
                &daily_source_points.0,
                &daily_source_points.1,
                7,
            )
        } else {
            let daily_source_points: Vec<(u64, f64)> = self
                .daily_source_portfolio_bucket()
                .map(|bucket| bucket.pnl_history.clone())
                .unwrap_or_default();
            Self::compute_daily_pnl_rows_from_cumulative(&daily_source_points, 7)
        };
        let no_data_text = if self.hide_pnl {
            "No daily performance data"
        } else {
            "No daily PnL data"
        };

        if daily_rows.is_empty() {
            column![
                text(no_data_text)
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
            ]
        } else {
            daily_rows
                .into_iter()
                .fold(Column::new().spacing(4), |column, (day, pnl)| {
                    let pnl_color = if pnl >= 0.0 {
                        theme.palette().success
                    } else {
                        theme.palette().danger
                    };
                    column.push(
                        row![
                            text(day).size(11).color(color!(0xaaaaaa)).width(Fill),
                            text(if self.hide_pnl {
                                format_signed_percent_value(pnl)
                            } else {
                                format_signed_usd_value(pnl)
                            })
                            .size(11)
                            .color(pnl_color),
                        ]
                        .spacing(8),
                    )
                })
        }
    }
}
