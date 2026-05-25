use crate::message::Message;

use super::super::model::{PnlCardDisplayMode, PnlCardPercentMode, PnlCardWindowState};

use iced::widget::container as container_style;
use iced::widget::{Column, button, checkbox, column, container, radio, row, rule, text};
use iced::{Alignment, Color, Element, Fill, Theme, window};

// ---------------------------------------------------------------------------
// PnL Card Editor
// ---------------------------------------------------------------------------

pub(super) fn view_pnl_card_editor<'a>(
    window_id: window::Id,
    state: &'a PnlCardWindowState,
    theme: &Theme,
) -> Element<'a, Message> {
    let display_modes =
        PnlCardDisplayMode::ALL
            .into_iter()
            .fold(Column::new().spacing(5), |col, mode| {
                col.push(radio(
                    mode.to_string(),
                    mode,
                    Some(state.display_mode),
                    move |selected| Message::SetPnlCardDisplayMode(window_id, selected),
                ))
            });

    let percent_modes =
        PnlCardPercentMode::ALL
            .into_iter()
            .fold(Column::new().spacing(5), |col, mode| {
                col.push(radio(
                    mode.to_string(),
                    mode,
                    Some(state.percent_mode),
                    move |selected| Message::SetPnlCardPercentMode(window_id, selected),
                ))
            });

    let controls = column![
        text("Card display").size(13).color(theme.palette().text),
        rule::horizontal(1),
        row![
            pnl_card_action_button("Copy Image", Message::CopyPnlCard(window_id)),
            pnl_card_action_button("Save PNG", Message::SavePnlCard(window_id)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        settings_group("PnL", display_modes.into()),
        settings_group("Percent", percent_modes.into()),
        settings_group(
            "Privacy",
            column![
                checkbox(state.obscure_prices)
                    .label("Obscure entry and exit digits")
                    .on_toggle(move |checked| Message::TogglePnlCardPricePrivacy(
                        window_id, checked
                    ))
                    .size(12)
                    .spacing(6)
                    .text_size(12)
                    .font(crate::app_fonts::monospace_font())
                    .width(Fill),
                checkbox(state.show_position_size)
                    .label("Show position size")
                    .on_toggle(move |checked| Message::TogglePnlCardPositionSize(
                        window_id, checked
                    ))
                    .size(12)
                    .spacing(6)
                    .text_size(12)
                    .font(crate::app_fonts::monospace_font())
                    .width(Fill),
            ]
            .spacing(6)
            .into(),
        ),
    ]
    .spacing(10)
    .width(Fill);

    container(controls)
        .width(Fill)
        .padding(12)
        .style(|theme: &Theme| container_style::Style {
            background: Some(theme.extended_palette().background.weak.color.into()),
            border: iced::Border {
                radius: 6.0.into(),
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
            },
            ..Default::default()
        })
        .into()
}

fn pnl_card_action_button(label: &'static str, msg: Message) -> Element<'static, Message> {
    button(text(label).size(12).center())
        .on_press(msg)
        .padding([6, 12])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn settings_group<'a>(label: &'static str, content: Element<'a, Message>) -> Element<'a, Message> {
    column![
        text(label)
            .size(10)
            .font(crate::app_fonts::monospace_font())
            .color(Color::from_rgb8(0x88, 0x88, 0x88)),
        content,
    ]
    .spacing(5)
    .width(Fill)
    .into()
}
