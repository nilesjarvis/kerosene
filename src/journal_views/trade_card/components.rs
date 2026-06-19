use crate::journal_views::style::{
    JOURNAL_CHIP_RADIUS, journal_accent_soft, journal_chip_style, journal_dim,
};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, column, container, text};
use iced::{Border, Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Trade Card / Detail Components
// ---------------------------------------------------------------------------

/// Reflection block: an orange-soft mono uppercase label over Inter body copy,
/// seated in a sunken well with a hairline accent edge.
pub(in crate::journal_views) fn journal_note_block<'a>(
    label: &'static str,
    body: &'a str,
    theme: &Theme,
) -> Element<'a, Message> {
    let accent = journal_accent_soft(theme);
    let body: Element<'a, Message> = if body.trim().is_empty() {
        text("—").size(13).color(journal_dim(theme)).into()
    } else {
        text(body)
            .size(13)
            .font(crate::app_fonts::sans_font())
            .color(theme.palette().text)
            .into()
    };

    container(
        column![
            text(label)
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(accent),
            body,
        ]
        .spacing(6),
    )
    .width(Fill)
    .padding([10, 12])
    .style(move |theme: &Theme| container_style::Style {
        background: Some(
            Color {
                a: 0.05,
                ..theme.palette().primary
            }
            .into(),
        ),
        border: Border {
            color: Color {
                a: 0.18,
                ..theme.palette().primary
            },
            width: 1.0,
            radius: JOURNAL_CHIP_RADIUS.into(),
        },
        ..Default::default()
    })
    .into()
}

/// Mono UPPERCASE pill with a tinted fill + border (side/status/tag chips).
pub(in crate::journal_views) fn journal_chip(
    label: impl Into<String>,
    tint: Color,
) -> Element<'static, Message> {
    container(
        text(label.into())
            .size(10)
            .font(crate::app_fonts::monospace_font()),
    )
    .padding([2, 6])
    .style(journal_chip_style(tint))
    .into()
}

/// Build a row of `#tag` chips. Returns `None` when there are no tags.
pub(in crate::journal_views) fn journal_tag_chips(
    tags: &[String],
    theme: &Theme,
) -> Option<Element<'static, Message>> {
    if tags.is_empty() {
        return None;
    }
    let accent = theme.palette().primary;
    let mut row = iced::widget::Row::new().spacing(6);
    for tag in tags {
        row = row.push(journal_chip(format!("#{tag}"), accent));
    }
    Some(row.into())
}

/// Helper to push an optional element onto a column.
pub(in crate::journal_views) fn push_opt<'a>(
    column: Column<'a, Message>,
    element: Option<Element<'a, Message>>,
) -> Column<'a, Message> {
    match element {
        Some(element) => column.push(element),
        None => column,
    }
}
