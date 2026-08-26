use crate::account_analytics::IncomeSnapshot;
use crate::account_views::portfolio::tokens;
use crate::denomination::DisplayDenominationContext;
use crate::message::Message;
use iced::widget::{Space, column, container, row, text};
use iced::{Element, Fill, Length, Theme};

pub(super) fn income_hero(
    data: &IncomeSnapshot,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    let earned_color = if data.earned_total >= 0.0 {
        tokens::up(theme)
    } else {
        tokens::down(theme)
    };
    let health = if data.health.trim().is_empty() {
        "—"
    } else {
        data.health.as_str()
    };
    let health_factor = data.health_factor.as_deref().unwrap_or("—");

    row![
        column![
            label(theme, "INTEREST EARNED · TOTAL"),
            text(denomination.format_signed_value(data.earned_total, 2))
                .size(25)
                .font(tokens::mono_semibold())
                .color(earned_color),
        ]
        .spacing(4)
        .width(Fill),
        column![
            label(theme, "ACCOUNT HEALTH"),
            row![
                text(health.to_string())
                    .size(14)
                    .font(tokens::mono())
                    .color(theme.palette().success),
                text(format!("HF {health_factor}"))
                    .size(11)
                    .font(tokens::mono())
                    .color(tokens::muted(theme)),
            ]
            .spacing(7)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(5)
        .align_x(iced::Alignment::End),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center)
    .width(Fill)
    .padding([8, 0])
    .into()
}

pub(super) fn income_windows(
    data: &IncomeSnapshot,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    row![
        metric(
            theme,
            "24H · UTC",
            denomination.format_signed_value(data.earned_24h, 2),
            signed_color(theme, data.earned_24h),
        ),
        vertical_hairline(theme, 42.0),
        metric(
            theme,
            "7D · UTC",
            denomination.format_signed_value(data.earned_7d, 2),
            signed_color(theme, data.earned_7d),
        ),
        vertical_hairline(theme, 42.0),
        metric(
            theme,
            "30D · UTC",
            denomination.format_signed_value(data.earned_30d, 2),
            signed_color(theme, data.earned_30d),
        ),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center)
    .width(Fill)
    .padding([7, 0])
    .into()
}

pub(super) fn income_carry(
    data: &IncomeSnapshot,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
    compact: bool,
) -> Element<'static, Message> {
    let projected = metric(
        theme,
        "PROJECTED / YEAR",
        denomination.format_signed_value(data.net_yearly_projection, 2),
        signed_color(theme, data.net_yearly_projection),
    );
    let supplied = metric(
        theme,
        "SUPPLIED",
        denomination.format_signed_value(data.current_supply_usd, 2),
        tokens::text(theme),
    );
    let borrowed = metric(
        theme,
        "BORROWED",
        denomination.format_value(data.current_borrow_usd, 2),
        tokens::text(theme),
    );

    if compact {
        column![
            row![projected, vertical_hairline(theme, 42.0), supplied]
                .spacing(10)
                .width(Fill),
            hairline(theme),
            borrowed,
        ]
        .spacing(8)
        .padding([6, 0])
        .into()
    } else {
        row![
            projected,
            vertical_hairline(theme, 42.0),
            supplied,
            vertical_hairline(theme, 42.0),
            borrowed,
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .width(Fill)
        .padding([7, 0])
        .into()
    }
}

pub(super) fn hairline(theme: &Theme) -> Element<'static, Message> {
    let color = tokens::border(theme);
    container(Space::new().width(Fill).height(1))
        .width(Fill)
        .height(1)
        .style(move |_theme: &Theme| container::Style {
            background: Some(color.into()),
            ..container::Style::default()
        })
        .into()
}

fn vertical_hairline(theme: &Theme, height: f32) -> Element<'static, Message> {
    let color = tokens::border(theme);
    container(Space::new().width(1).height(height))
        .width(1)
        .height(Length::Fixed(height))
        .style(move |_theme: &Theme| container::Style {
            background: Some(color.into()),
            ..container::Style::default()
        })
        .into()
}

fn metric(
    theme: &Theme,
    metric_label: &str,
    value: String,
    value_color: iced::Color,
) -> Element<'static, Message> {
    column![
        label(theme, metric_label),
        text(value).size(15).font(tokens::mono()).color(value_color),
    ]
    .spacing(4)
    .width(Fill)
    .into()
}

fn label(theme: &Theme, value: &str) -> Element<'static, Message> {
    text(value.to_string())
        .size(10)
        .font(tokens::mono())
        .color(tokens::dim(theme))
        .into()
}

fn signed_color(theme: &Theme, value: f64) -> iced::Color {
    if value >= 0.0 {
        tokens::up(theme)
    } else {
        tokens::down(theme)
    }
}
