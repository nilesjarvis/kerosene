use crate::alfred_state::AlfredCommand;
use crate::app_state::TradingTerminal;
use crate::helpers::text_input_style;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, Space, button, column, container, row, rule, stack, text, text_input};
use iced::{Color, Element, Fill, Theme, color};

// ---------------------------------------------------------------------------
// Alfred overlay
// ---------------------------------------------------------------------------

const ALFRED_MAX_RESULTS: usize = 7;

impl TradingTerminal {
    pub(crate) fn view_alfred_overlay<'a>(&'a self, theme: &Theme) -> Option<Element<'a, Message>> {
        if !self.alfred.open {
            return None;
        }

        let commands = self.alfred_filtered_commands();
        let selected_index = self
            .alfred
            .selected_index
            .min(commands.len().saturating_sub(1));

        let input = text_input("alfred", &self.alfred.query)
            .id(Self::alfred_input_id())
            .on_input(Message::AlfredQueryChanged)
            .on_submit(Message::AlfredSubmit)
            .padding([9, 12])
            .size(14)
            .style(text_input_style);

        let mut results = Column::new().spacing(2).width(Fill);
        for (index, command) in commands.iter().take(ALFRED_MAX_RESULTS).enumerate() {
            results = results.push(alfred_result_row(command, index == selected_index, theme));
        }

        if commands.is_empty() {
            results = results.push(
                container(
                    text("No matches")
                        .size(11)
                        .color(theme.extended_palette().background.weak.text),
                )
                .padding([8, 10])
                .width(Fill),
            );
        }

        let card = container(column![input, rule::horizontal(1), results].spacing(6))
            .padding(8)
            .width(Fill)
            .max_width(560.0)
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
) -> Element<'static, Message> {
    let enabled = command.enabled;
    let command_id = command.id;
    let title_color = if enabled {
        theme.palette().text
    } else {
        theme.extended_palette().background.weak.text
    };
    let detail_color = theme.extended_palette().background.weak.text;
    let tag = alfred_tag(&command.tag, theme);

    button(
        row![
            column![
                text(command.title.clone()).size(12).color(title_color),
                text(command.detail.clone()).size(10).color(detail_color),
            ]
            .spacing(2)
            .width(Fill),
            tag,
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
    )
    .on_press_maybe(enabled.then_some(Message::AlfredCommandSelected(command_id)))
    .padding([7, 9])
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

fn alfred_tag(label: &str, theme: &Theme) -> Element<'static, Message> {
    let color = match label {
        "Open" => theme.palette().success,
        "Window" => theme.palette().primary,
        "Limit" | "Market" | "Trade" | "Chase" => theme.palette().primary,
        "Close" | "NUKE" => color!(0xff5555),
        "Requires PM" => color!(0xffb86c),
        _ => theme.extended_palette().background.weak.text,
    };

    container(text(label.to_string()).size(9).color(color))
        .padding([1, 5])
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
