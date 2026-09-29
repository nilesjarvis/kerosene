use crate::journal_views::analytics::JournalKpis;
use crate::journal_views::style::{journal_dim, journal_muted, journal_surface_sunken};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{column, container, row, text};
use iced::{Border, Color, Element, Fill, Theme};

pub(super) fn view_journal_kpi_tiles(
    kpis: &JournalKpis,
    denomination: &crate::denomination::DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    let signed = |value: f64| crate::helpers::signed_number_color(value, theme);
    let text_color = theme.palette().text;

    let ratio = match (kpis.avg_win, kpis.avg_loss) {
        (Some(win), Some(loss)) if loss.abs() > 0.0 => format!("{:.2} : 1", win / loss.abs()),
        _ => "—".to_string(),
    };

    column![
        row![
            kpi_tile(
                "Expectancy / Trade",
                kpis.expectancy
                    .map(|value| denomination.format_signed_value(value, 2))
                    .unwrap_or_else(|| "—".to_string()),
                kpis.expectancy.map(signed).unwrap_or(text_color),
                "per scored trade",
                theme,
            ),
            kpi_tile(
                "Avg Win : Avg Loss",
                ratio,
                text_color,
                "reward / risk",
                theme,
            ),
        ]
        .spacing(12),
        row![
            kpi_tile(
                "Avg R Multiple",
                kpis.avg_r
                    .map(|value| format!("{value:+.2}R"))
                    .unwrap_or_else(|| "—".to_string()),
                kpis.avg_r.map(signed).unwrap_or(text_color),
                "vs avg loss",
                theme,
            ),
            kpi_tile(
                "Total Fees",
                denomination.format_value(kpis.total_fees, 2),
                theme.palette().warning,
                "all trades",
                theme,
            ),
        ]
        .spacing(12),
    ]
    .spacing(12)
    .into()
}

fn kpi_tile(
    label: &'static str,
    value: String,
    value_color: Color,
    caption: &'static str,
    theme: &Theme,
) -> Element<'static, Message> {
    container(
        column![
            text(label)
                .size(9)
                .font(crate::app_fonts::monospace_font())
                .color(journal_muted(theme)),
            text(value)
                .size(18)
                .font(crate::app_fonts::monospace_font())
                .color(value_color),
            text(caption)
                .size(9)
                .font(crate::app_fonts::monospace_font())
                .color(journal_dim(theme)),
        ]
        .spacing(4),
    )
    .width(Fill)
    .padding(10)
    .style(move |theme: &Theme| container_style::Style {
        background: Some(journal_surface_sunken(theme).into()),
        border: Border {
            color: crate::journal_views::style::journal_hairline(theme),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    })
    .into()
}
