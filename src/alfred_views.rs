use crate::alfred_state::AlfredCommand;
use crate::app_state::TradingTerminal;
use crate::helpers::{self, text_input_style};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, Space, button, column, container, row, rule, stack, text, text_input};
use iced::{Color, Element, Fill, Theme, color};

// ---------------------------------------------------------------------------
// Alfred overlay
// ---------------------------------------------------------------------------

const BASE_ALFRED_WIDTH: f32 = 560.0;
const ALFRED_MAX_RESULTS: usize = 7;

impl TradingTerminal {
    pub(crate) fn view_alfred_overlay<'a>(&'a self, theme: &Theme) -> Option<Element<'a, Message>> {
        if !self.alfred.open {
            return None;
        }

        let popup_scale = self.alfred_popup_scale;
        let commands = self.alfred_filtered_commands();
        let selected_index = self
            .alfred
            .selected_index
            .min(commands.len().saturating_sub(1));

        let input = text_input("alfred", &self.alfred.query)
            .id(Self::alfred_input_id())
            .on_input(Message::AlfredQueryChanged)
            .on_submit(Message::AlfredSubmit)
            .padding([scaled_px(9.0, popup_scale), scaled_px(12.0, popup_scale)])
            .size(scaled_text(14.0, popup_scale))
            .style(text_input_style);

        let mut results = Column::new().spacing(2).width(Fill);
        for (index, command) in commands.iter().take(ALFRED_MAX_RESULTS).enumerate() {
            results = results.push(alfred_result_row(
                command,
                index == selected_index,
                theme,
                popup_scale,
            ));
        }

        if commands.is_empty() {
            results = results.push(
                container(
                    text("No matches")
                        .size(scaled_text(11.0, popup_scale))
                        .color(theme.extended_palette().background.weak.text),
                )
                .padding([scaled_px(8.0, popup_scale), scaled_px(10.0, popup_scale)])
                .width(Fill),
            );
        }

        let card = container(column![input, rule::horizontal(1), results].spacing(6))
            .padding(scaled_px(8.0, popup_scale))
            .width(Fill)
            .max_width(BASE_ALFRED_WIDTH * popup_scale)
            .style(|theme: &Theme| alfred_card_style(theme));

        let close_layer = button(Space::new().width(Fill).height(Fill))
            .on_press(Message::CloseAlfred)
            .width(Fill)
            .height(Fill)
            .padding(0)
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(Color::TRANSPARENT.into()),
                text_color: Color::TRANSPARENT,
                border: iced::Border::default(),
                ..Default::default()
            });

        let card_layer = container(
            column![card]
                .width(Fill)
                .height(Fill)
                .align_x(iced::Alignment::Center),
        )
        .padding([52, 12])
        .width(Fill)
        .height(Fill);

        Some(
            stack![close_layer, card_layer]
                .width(Fill)
                .height(Fill)
                .into(),
        )
    }
}

fn alfred_result_row(
    command: &AlfredCommand,
    selected: bool,
    theme: &Theme,
    popup_scale: f32,
) -> Element<'static, Message> {
    let enabled = command.enabled;
    let command_id = command.id;
    let title_color = if enabled {
        theme.palette().text
    } else {
        theme.extended_palette().background.weak.text
    };
    let detail_color = theme.extended_palette().background.weak.text;
    let tag = alfred_tag(&command.tag, theme, popup_scale);
    let title = alfred_title(command, title_color, popup_scale);

    button(
        row![
            column![
                title,
                text(command.detail.clone())
                    .size(scaled_text(10.0, popup_scale))
                    .color(detail_color),
            ]
            .spacing(scaled_px(2.0, popup_scale) as f32)
            .width(Fill),
            tag,
        ]
        .spacing(scaled_px(10.0, popup_scale) as f32)
        .align_y(iced::Alignment::Center),
    )
    .on_press_maybe(enabled.then_some(Message::AlfredCommandSelected(command_id)))
    .padding([scaled_px(7.0, popup_scale), scaled_px(9.0, popup_scale)])
    .width(Fill)
    .style(move |theme: &Theme, status| {
        let bg = match (selected, status) {
            (true, _) => theme.extended_palette().background.strong.color,
            (false, button::Status::Hovered) if enabled => {
                theme.extended_palette().background.weak.color
            }
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(bg.into()),
            text_color: if enabled {
                theme.palette().text
            } else {
                theme.extended_palette().background.weak.text
            },
            border: iced::Border {
                radius: 3.0.into(),
                width: if selected { 1.0 } else { 0.0 },
                color: if selected {
                    theme.palette().primary
                } else {
                    Color::TRANSPARENT
                },
            },
            ..Default::default()
        }
    })
    .into()
}

fn alfred_title(
    command: &AlfredCommand,
    title_color: Color,
    popup_scale: f32,
) -> Element<'static, Message> {
    let Some(icon_symbol) = command.icon_symbol.as_deref() else {
        return alfred_plain_title(&command.title, title_color, popup_scale);
    };
    let Some(anchor) = command.icon_title_anchor.as_deref() else {
        return alfred_plain_title(&command.title, title_color, popup_scale);
    };
    let Some(icon) = helpers::symbol_icon(
        icon_symbol,
        scaled_text(14.0, popup_scale) as u16,
        title_color,
    ) else {
        return alfred_plain_title(&command.title, title_color, popup_scale);
    };
    let Some(start) = command.title.rfind(anchor) else {
        return alfred_plain_title(&command.title, title_color, popup_scale);
    };

    let end = start + anchor.len();
    let before = command.title[..start].trim_end();
    let ticker = &command.title[start..end];
    let after = command.title[end..].trim_start();

    let mut title = row![].align_y(iced::Alignment::Center);
    if !before.is_empty() {
        title = title
            .push(
                text(before.to_string())
                    .size(scaled_text(12.0, popup_scale))
                    .color(title_color),
            )
            .push(Space::new().width(scaled_px(5.0, popup_scale) as f32));
    }
    title = title
        .push(icon)
        .push(Space::new().width(scaled_px(4.0, popup_scale) as f32))
        .push(
            text(ticker.to_string())
                .size(scaled_text(12.0, popup_scale))
                .color(title_color),
        );
    if !after.is_empty() {
        title = title
            .push(Space::new().width(scaled_px(4.0, popup_scale) as f32))
            .push(
                text(after.to_string())
                    .size(scaled_text(12.0, popup_scale))
                    .color(title_color),
            );
    }

    title.into()
}

fn alfred_plain_title(
    title: &str,
    title_color: Color,
    popup_scale: f32,
) -> Element<'static, Message> {
    text(title.to_string())
        .size(scaled_text(12.0, popup_scale))
        .color(title_color)
        .into()
}

fn alfred_tag(label: &str, theme: &Theme, popup_scale: f32) -> Element<'static, Message> {
    let color = match label {
        "Open" => theme.palette().success,
        "Window" => theme.palette().primary,
        "Limit" | "Market" | "Trade" | "Chase" => theme.palette().primary,
        "Close" | "NUKE" => color!(0xff5555),
        "Requires PM" => color!(0xffb86c),
        _ => theme.extended_palette().background.weak.text,
    };

    container(
        text(label.to_string())
            .size(scaled_text(9.0, popup_scale))
            .color(color),
    )
    .padding([scaled_px(1.0, popup_scale), scaled_px(5.0, popup_scale)])
    .style(move |_theme: &Theme| container_style::Style {
        background: Some(Color { a: 0.12, ..color }.into()),
        border: iced::Border {
            radius: 3.0.into(),
            width: 1.0,
            color: Color { a: 0.45, ..color },
        },
        ..Default::default()
    })
    .into()
}

fn scaled_text(size: f32, scale: f32) -> u32 {
    (size * scale.clamp(0.90, 1.35)).round().clamp(1.0, 48.0) as u32
}

fn scaled_px(size: f32, scale: f32) -> u16 {
    (size * scale.clamp(0.85, 1.60)).round().clamp(1.0, 64.0) as u16
}

fn alfred_card_style(theme: &Theme) -> container_style::Style {
    let mut shadow_color = Color::BLACK;
    shadow_color.a = 0.28;

    container_style::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: iced::Border {
            radius: 6.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        shadow: iced::Shadow {
            color: shadow_color,
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        ..Default::default()
    }
}
