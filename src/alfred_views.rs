use self::rows::{alfred_result_row, scaled_px, scaled_text};
use crate::app_state::TradingTerminal;
use crate::helpers::text_input_style;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, Space, button, column, container, rule, stack, text, text_input};
use iced::{Color, Element, Fill, Theme};

mod rows;

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
