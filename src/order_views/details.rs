use crate::message::Message;
use iced::widget::{column, container, text};
use iced::{Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Advanced Order Detail Components
// ---------------------------------------------------------------------------

pub(super) fn short_id(value: &str) -> String {
    if value.len() <= 10 {
        value.to_string()
    } else {
        format!("{}...", &value[..10])
    }
}

pub(super) fn order_child_id_text(oid: Option<u64>, cloid: Option<&str>) -> String {
    match (oid, cloid) {
        (Some(oid), Some(cloid)) => format!("#{oid} {}", short_id(cloid)),
        (Some(oid), None) => format!("#{oid}"),
        (None, Some(cloid)) => short_id(cloid),
        (None, None) => "-".to_string(),
    }
}

pub(super) fn section_title<'a>(label: &'static str, theme: &Theme) -> iced::widget::Text<'a> {
    text(label)
        .size(12)
        .color(theme.extended_palette().background.weak.text)
}

pub(super) fn metric<'a>(
    label: &'static str,
    value: String,
    weak: iced::Color,
) -> Element<'a, Message> {
    container(column![text(label).size(10).color(weak), text(value).size(12)].spacing(2))
        .padding([6, 8])
        .width(Fill)
        .style(|theme: &Theme| iced::widget::container::Style {
            background: Some(theme.extended_palette().background.weak.color.into()),
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
            },
            ..Default::default()
        })
        .into()
}
