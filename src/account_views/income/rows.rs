use crate::account_analytics::{IncomeHourlyPayment, IncomeTokenRow};
use crate::account_views::portfolio::tokens;
use crate::denomination::DisplayDenominationContext;
use crate::message::Message;
use chrono::{DateTime, Utc};
use iced::widget::{Column, column, container, row, text};
use iced::{Element, Fill, Length, Theme};

pub(super) fn view_income_token_rows(
    rows: &[IncomeTokenRow],
    denomination: &DisplayDenominationContext,
    theme: &Theme,
    compact: bool,
) -> Column<'static, Message> {
    if rows.is_empty() {
        return column![
            text("No borrow/lend positions")
                .size(12)
                .font(tokens::mono())
                .color(tokens::muted(theme))
        ];
    }

    rows.iter()
        .fold(Column::new().spacing(0), |rows, row_data| {
            let value_color = if row_data.net_yearly_usd >= 0.0 {
                tokens::up(theme)
            } else {
                tokens::down(theme)
            };
            let cells = if compact {
                row![
                    table_cell(
                        format!("{}  #{}", row_data.token_label, row_data.token),
                        3,
                        false,
                        tokens::text(theme),
                    ),
                    table_cell(
                        format!("{:.2}%", row_data.supply_rate * 100.0),
                        2,
                        true,
                        theme.palette().primary,
                    ),
                    table_cell(
                        denomination.format_signed_value(row_data.net_yearly_usd, 2),
                        3,
                        true,
                        value_color,
                    ),
                ]
            } else {
                row![
                    table_cell(
                        format!("{}  #{}", row_data.token_label, row_data.token),
                        3,
                        false,
                        tokens::text(theme),
                    ),
                    table_cell(
                        denomination.format_signed_value(row_data.supply_usd, 2),
                        3,
                        true,
                        theme.palette().primary,
                    ),
                    table_cell(
                        format!("{:.2}%", row_data.supply_rate * 100.0),
                        2,
                        true,
                        theme.palette().primary,
                    ),
                    table_cell(
                        denomination.format_value(row_data.borrow_usd, 2),
                        3,
                        true,
                        theme.palette().warning,
                    ),
                    table_cell(
                        denomination.format_signed_value(row_data.net_yearly_usd, 2),
                        3,
                        true,
                        value_color,
                    ),
                ]
            };

            rows.push(
                container(
                    cells
                        .spacing(8)
                        .align_y(iced::Alignment::Center)
                        .width(Fill),
                )
                .width(Fill)
                .padding([9, 0]),
            )
        })
}

pub(super) fn view_income_hourly_rows(
    rows: &[IncomeHourlyPayment],
    _denomination: &DisplayDenominationContext,
    theme: &Theme,
    compact: bool,
) -> Column<'static, Message> {
    if rows.is_empty() {
        return column![
            text("No recent hourly interest payments")
                .size(12)
                .font(tokens::mono())
                .color(tokens::muted(theme))
        ];
    }

    rows.iter()
        .fold(Column::new().spacing(0), |rows, row_data| {
            let time_label = i64::try_from(row_data.time)
                .ok()
                .and_then(DateTime::<Utc>::from_timestamp_millis)
                .map(|dt| dt.format("%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "—".to_string());
            let value_color = if row_data.net >= 0.0 {
                tokens::up(theme)
            } else {
                tokens::down(theme)
            };
            let apr = format!("{:.2}%", row_data.supply_rate * 100.0);
            let size = if compact { 11.0 } else { 12.0 };
            let cells = if compact {
                row![
                    table_cell_sized(time_label, 3, false, tokens::muted(theme), size),
                    table_cell_sized(
                        row_data.token_label.clone(),
                        2,
                        false,
                        tokens::text(theme),
                        size
                    ),
                    supply_borrow_cell(theme, row_data.supply, row_data.borrow, 3, size),
                    table_cell_sized(apr, 2, true, theme.palette().primary, size),
                    table_cell_sized(
                        signed_token_amount(row_data.net),
                        3,
                        true,
                        value_color,
                        size
                    ),
                ]
            } else {
                row![
                    table_cell_sized(time_label, 3, false, tokens::muted(theme), size),
                    table_cell_sized(
                        row_data.token_label.clone(),
                        2,
                        false,
                        tokens::text(theme),
                        size
                    ),
                    table_cell_sized(
                        token_amount(row_data.supply),
                        2,
                        true,
                        theme.palette().primary,
                        size
                    ),
                    table_cell_sized(
                        token_amount(row_data.borrow),
                        2,
                        true,
                        theme.palette().warning,
                        size
                    ),
                    table_cell_sized(apr, 2, true, theme.palette().primary, size),
                    table_cell_sized(
                        signed_token_amount(row_data.net),
                        3,
                        true,
                        value_color,
                        size
                    ),
                ]
            };

            rows.push(
                container(
                    cells
                        .spacing(6)
                        .align_y(iced::Alignment::Center)
                        .width(Fill),
                )
                .width(Fill)
                .padding([if compact { 4.0 } else { 6.0 }, 0.0]),
            )
        })
}

/// Supply and borrow accrual for one payment, merged into a single cell so the
/// compact layout can carry both without growing the row.
fn supply_borrow_cell(
    theme: &Theme,
    supply: f64,
    borrow: f64,
    portion: u16,
    size: f32,
) -> Element<'static, Message> {
    container(
        row![
            text(token_amount(supply))
                .size(size)
                .font(tokens::mono())
                .color(theme.palette().primary),
            text("·")
                .size(size)
                .font(tokens::mono())
                .color(tokens::dim(theme)),
            text(token_amount(borrow))
                .size(size)
                .font(tokens::mono())
                .color(theme.palette().warning),
        ]
        .spacing(3),
    )
    .width(Length::FillPortion(portion))
    .align_x(iced::alignment::Horizontal::Right)
    .into()
}

/// Render an accrual in raw token units with enough decimals for tiny hourly
/// amounts, trimming trailing zeros (`0.000135` instead of `0.00`).
fn token_amount(value: f64) -> String {
    let trimmed = format!("{value:.8}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string();
    if trimmed == "-" {
        "0".to_string()
    } else {
        trimmed
    }
}

fn signed_token_amount(value: f64) -> String {
    let prefix = if value < 0.0 { "-" } else { "+" };
    format!("{prefix}{}", token_amount(value.abs()))
}

fn table_cell(
    value: String,
    portion: u16,
    right_aligned: bool,
    color: iced::Color,
) -> Element<'static, Message> {
    table_cell_sized(value, portion, right_aligned, color, 12.0)
}

fn table_cell_sized(
    value: String,
    portion: u16,
    right_aligned: bool,
    color: iced::Color,
    size: f32,
) -> Element<'static, Message> {
    container(text(value).size(size).font(tokens::mono()).color(color))
        .width(Length::FillPortion(portion))
        .align_x(if right_aligned {
            iced::alignment::Horizontal::Right
        } else {
            iced::alignment::Horizontal::Left
        })
        .into()
}
