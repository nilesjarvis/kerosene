use crate::helpers::format_decimal_with_commas;
use crate::hype_unstaking_state::{format_countdown, format_hype_wei};
use crate::message::Message;

use iced::widget::{Column, column, container, row, text};
use iced::{Element, Fill, Theme, color};

// ---------------------------------------------------------------------------
// Summary Metrics
// ---------------------------------------------------------------------------

pub(super) fn hype_unstaking_summary_grid(
    summary: &crate::hype_unstaking_state::HypeUnstakingSummary,
    now_ms: u64,
    available_width: f32,
    theme: &Theme,
) -> Element<'static, Message> {
    let next_unlock = summary
        .next_unlock_time_ms
        .map(|unlock_time_ms| format_countdown(unlock_time_ms, now_ms))
        .unwrap_or_else(|| "-".to_string());
    let largest_unlock = summary
        .largest_amount_wei
        .map(|amount| format_hype_wei(amount as u128))
        .unwrap_or_else(|| "-".to_string());

    let metrics = [
        (
            "Wallets",
            format_decimal_with_commas(summary.unique_wallet_count as f64, 0),
        ),
        ("Total", format_hype_wei(summary.total_wei)),
        ("Next", next_unlock),
        ("Largest", largest_unlock),
    ];
    let columns = if available_width >= 680.0 {
        5
    } else if available_width >= 480.0 {
        3
    } else if available_width >= 320.0 {
        2
    } else {
        1
    };

    let mut grid = Column::new().spacing(6);
    let mut metrics = metrics.into_iter();
    while metrics.len() > 0 {
        grid = grid.push(
            row(metrics
                .by_ref()
                .take(columns)
                .map(|(label, value)| metric_card(label, value, theme)))
            .spacing(6)
            .width(Fill),
        );
    }
    grid.into()
}

fn metric_card(label: &'static str, value: String, theme: &Theme) -> Element<'static, Message> {
    let text_color = theme.palette().text;
    let border_color = theme.extended_palette().background.strong.color;
    let background = theme.extended_palette().background.base.color;

    container(
        column![
            text(label).size(10).color(color!(0x888888)).width(Fill),
            text(value)
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(text_color)
                .width(Fill),
        ]
        .spacing(2),
    )
    .width(Fill)
    .padding([6, 8])
    .style(move |_theme: &Theme| container::Style {
        background: Some(background.into()),
        border: iced::Border {
            radius: 3.0.into(),
            width: 1.0,
            color: border_color,
        },
        ..Default::default()
    })
    .into()
}
