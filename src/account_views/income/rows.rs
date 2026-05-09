use crate::account_analytics::{IncomeHourlyPayment, IncomeTokenRow};
use crate::account_metrics::format_signed_usd_value;
use crate::message::Message;
use chrono::{DateTime, Utc};
use iced::widget::{Column, column, row, text};
use iced::{Theme, color};

// ---------------------------------------------------------------------------
// Income Data Rows
// ---------------------------------------------------------------------------

pub(super) fn view_income_token_rows<'a>(
    rows: &'a [IncomeTokenRow],
    theme: &Theme,
) -> Column<'a, Message> {
    if rows.is_empty() {
        column![
            text("No borrow/lend positions")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
        ]
    } else {
        rows.iter()
            .take(8)
            .fold(Column::new().spacing(4), |col, row_data| {
                let net_color = if row_data.net_yearly_usd >= 0.0 {
                    theme.palette().success
                } else {
                    theme.palette().danger
                };
                col.push(
                    row![
                        text(format!("{} ({})", row_data.token_label, row_data.token))
                            .size(11)
                            .color(color!(0xaaaaaa))
                            .width(120),
                        text(format_signed_usd_value(row_data.supply_usd))
                            .size(11)
                            .color(color!(0x8be9fd))
                            .width(90),
                        text(format!("{:.2}%", row_data.supply_rate * 100.0))
                            .size(11)
                            .color(color!(0x8be9fd))
                            .width(56),
                        text(format_signed_usd_value(row_data.borrow_usd))
                            .size(11)
                            .color(color!(0xffb86c))
                            .width(90),
                        text(format_signed_usd_value(row_data.net_yearly_usd))
                            .size(11)
                            .color(net_color),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                )
            })
    }
}

pub(super) fn view_income_hourly_rows<'a>(
    rows: &'a [IncomeHourlyPayment],
    theme: &Theme,
) -> Column<'a, Message> {
    if rows.is_empty() {
        column![
            text("No recent hourly interest rows")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
        ]
    } else {
        rows.iter().fold(Column::new().spacing(3), |col, row_data| {
            let time_label = i64::try_from(row_data.time)
                .ok()
                .and_then(DateTime::<Utc>::from_timestamp_millis)
                .map(|dt| dt.format("%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "--".to_string());
            let net_color = if row_data.net >= 0.0 {
                theme.palette().success
            } else {
                theme.palette().danger
            };
            col.push(
                row![
                    text(format!("{time_label} UTC"))
                        .size(10)
                        .color(color!(0xaaaaaa))
                        .width(92),
                    text(&row_data.token_label)
                        .size(10)
                        .color(color!(0x9ec2ff))
                        .width(70),
                    text(format_signed_usd_value(row_data.supply))
                        .size(10)
                        .color(color!(0x8be9fd))
                        .width(84),
                    text(format_signed_usd_value(row_data.borrow))
                        .size(10)
                        .color(color!(0xffb86c))
                        .width(84),
                    text(format_signed_usd_value(row_data.net))
                        .size(10)
                        .color(net_color),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            )
        })
    }
}
