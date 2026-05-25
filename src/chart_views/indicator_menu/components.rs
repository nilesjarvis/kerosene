use super::IndicatorOption;
use crate::chart_state::ChartId;
use crate::message::Message;

use iced::widget::{Column, Space, checkbox, row, rule, text};
use iced::{Alignment, Color, Element, Fill, Length, Theme};

// ---------------------------------------------------------------------------
// Indicator Menu Components
// ---------------------------------------------------------------------------

pub(super) fn indicator_group<const N: usize>(
    chart_id: ChartId,
    label: &'static str,
    options: [IndicatorOption; N],
) -> Element<'static, Message> {
    let mut rows = Column::new().spacing(2).width(Fill);

    for pair in options.chunks(2) {
        let mut option_row = row![].spacing(8).align_y(Alignment::Center).width(Fill);

        for option in pair {
            option_row = option_row.push(indicator_checkbox(chart_id, *option));
        }

        if pair.len() == 1 {
            option_row = option_row.push(Space::new().width(Length::FillPortion(1)));
        }

        rows = rows.push(option_row);
    }

    row![indicator_group_label(label), rows]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Fill)
        .into()
}

pub(super) fn indicator_footer<const N: usize>(
    chart_id: ChartId,
    options: [IndicatorOption; N],
) -> Element<'static, Message> {
    let mut option_row = row![].spacing(8).align_y(Alignment::Center).width(Fill);
    for option in options {
        option_row = option_row.push(indicator_checkbox(chart_id, option));
    }

    row![Space::new().width(24.0), option_row]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Fill)
        .into()
}

pub(super) fn indicator_group_label(label: &'static str) -> Element<'static, Message> {
    text(label)
        .size(10)
        .font(crate::app_fonts::monospace_font())
        .color(Color::from_rgb8(0x88, 0x88, 0x88))
        .width(24.0)
        .into()
}

pub(super) fn menu_checkbox(
    label: &'static str,
    checked: bool,
    message: Message,
) -> Element<'static, Message> {
    checkbox(checked)
        .label(label)
        .on_toggle(move |_| message.clone())
        .size(10)
        .spacing(4)
        .text_size(10)
        .font(crate::app_fonts::monospace_font())
        .width(Length::FillPortion(1))
        .into()
}

pub(super) fn compact_separator() -> Element<'static, Message> {
    rule::horizontal(1)
        .style(|theme: &Theme| rule::Style {
            color: Color {
                a: 0.16,
                ..theme.extended_palette().background.weak.text
            },
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}

fn indicator_checkbox(chart_id: ChartId, option: IndicatorOption) -> Element<'static, Message> {
    menu_checkbox(
        option.label,
        option.checked,
        Message::ToggleMacroIndicator(chart_id, option.key.to_string()),
    )
}
