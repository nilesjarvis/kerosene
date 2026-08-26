use crate::account_analytics::IncomeSnapshot;
use crate::account_views::portfolio::tokens;
use crate::denomination::DisplayDenominationContext;
use crate::message::Message;
use iced::widget::{Space, column, container, row, text};
use iced::{Element, Fill, Length, Theme};

pub(super) fn income_token_section_header(
    data: &IncomeSnapshot,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    section_header(
        theme,
        "Token contribution",
        "Annualized at current rates",
        denomination.format_signed_value(data.net_yearly_projection, 2),
        if data.net_yearly_projection >= 0.0 {
            tokens::up(theme)
        } else {
            tokens::down(theme)
        },
    )
}

pub(super) fn income_hourly_section_header(
    data: &IncomeSnapshot,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    section_header(
        theme,
        "Hourly payments",
        "Recent realized interest · S APR at current rates",
        format!(
            "{} / 24H",
            denomination.format_signed_value(data.earned_24h, 2)
        ),
        if data.earned_24h >= 0.0 {
            tokens::up(theme)
        } else {
            tokens::down(theme)
        },
    )
}

pub(super) fn income_token_table_header(theme: &Theme, compact: bool) -> Element<'static, Message> {
    let row = if compact {
        row![
            header_cell(theme, "Token", 3, false),
            header_cell(theme, "S APR", 2, true),
            header_cell(theme, "Net / Y", 3, true),
        ]
    } else {
        row![
            header_cell(theme, "Token", 3, false),
            header_cell(theme, "Supply", 3, true),
            header_cell(theme, "S APR", 2, true),
            header_cell(theme, "Borrow", 3, true),
            header_cell(theme, "Net / Y", 3, true),
        ]
    };

    row.spacing(8).width(Fill).into()
}

pub(super) fn income_hourly_table_header(
    theme: &Theme,
    compact: bool,
) -> Element<'static, Message> {
    let row = if compact {
        row![
            header_cell(theme, "Time", 3, false),
            header_cell(theme, "Token", 2, false),
            header_cell(theme, "S / B", 3, true),
            header_cell(theme, "S APR", 2, true),
            header_cell(theme, "Net", 3, true),
        ]
    } else {
        row![
            header_cell(theme, "Time", 3, false),
            header_cell(theme, "Token", 2, false),
            header_cell(theme, "Supply", 2, true),
            header_cell(theme, "Borrow", 2, true),
            header_cell(theme, "S APR", 2, true),
            header_cell(theme, "Net", 3, true),
        ]
    };

    row.spacing(8).width(Fill).into()
}

fn section_header(
    theme: &Theme,
    title: &str,
    subtitle: &str,
    value: String,
    value_color: iced::Color,
) -> Element<'static, Message> {
    row![
        column![
            text(title.to_string())
                .size(17)
                .font(tokens::mono_semibold())
                .color(tokens::text(theme)),
            text(subtitle.to_string())
                .size(10)
                .font(tokens::mono())
                .color(tokens::dim(theme)),
        ]
        .spacing(4),
        Space::new().width(Fill),
        text(value).size(15).font(tokens::mono()).color(value_color),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center)
    .width(Fill)
    .padding([8, 0])
    .into()
}

fn header_cell(
    theme: &Theme,
    label: &str,
    portion: u16,
    right_aligned: bool,
) -> Element<'static, Message> {
    container(
        text(label.to_string())
            .size(10)
            .font(tokens::mono())
            .color(tokens::dim(theme)),
    )
    .width(Length::FillPortion(portion))
    .align_x(if right_aligned {
        iced::alignment::Horizontal::Right
    } else {
        iced::alignment::Horizontal::Left
    })
    .into()
}
