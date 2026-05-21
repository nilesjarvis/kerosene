use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use iced::widget::{Column, button, column, row, text};
use iced::{Element, Fill, Length, Theme};

// ---------------------------------------------------------------------------
// Display Font Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_settings_display_font_section(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let weak_text = theme.extended_palette().background.weak.text;
        let mut font_list = Column::new().spacing(4).push(display_font_option_button(
            "System Default".to_string(),
            Message::DisplayFontChanged(config::DisplayFontConfig::System),
            matches!(self.display_font, config::DisplayFontConfig::System),
        ));

        for family in config::BUNDLED_DISPLAY_FONT_FAMILIES {
            let display_font = config::DisplayFontConfig::Custom {
                family: (*family).to_string(),
            };
            let is_active = self.display_font == display_font;
            font_list = font_list.push(display_font_option_button(
                (*family).to_string(),
                Message::DisplayFontChanged(display_font),
                is_active,
            ));
        }

        for font in &self.custom_fonts {
            if config::bundled_display_font_family(&font.family).is_some() {
                continue;
            }

            let display_font = config::DisplayFontConfig::Custom {
                family: font.family.clone(),
            };
            let is_active = self.display_font == display_font;
            font_list = font_list.push(display_font_option_button(
                font.family.clone(),
                Message::DisplayFontChanged(display_font),
                is_active,
            ));
        }

        column![
            text("Display Font").size(14).color(theme.palette().text),
            text("Restart Kerosene after changing fonts. Prices, tables, and chart scales stay monospaced.")
                .size(11)
                .color(weak_text),
            font_list,
            row![
                button(text("Import Font").size(12))
                    .padding([6, 10])
                    .on_press(Message::ImportDisplayFont),
                button(text("Reset").size(12))
                    .padding([6, 10])
                    .on_press(Message::DisplayFontChanged(config::DisplayFontConfig::System)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(10)
        .into()
    }
}

fn display_font_option_button(
    label: String,
    message: Message,
    is_active: bool,
) -> Element<'static, Message> {
    let mut content = row![text(label).size(12).width(Fill)]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .width(Fill);

    if is_active {
        content = content.push(
            text("Active")
                .size(11)
                .align_x(iced::alignment::Horizontal::Right)
                .width(Length::Fixed(52.0)),
        );
    }

    button(content)
        .padding([6, 10])
        .width(Fill)
        .on_press(message)
        .style(move |theme: &Theme, status| {
            let extended = theme.extended_palette();
            let bg = match status {
                button::Status::Hovered => extended.background.strong.color.into(),
                _ => extended.background.weak.color.into(),
            };

            button::Style {
                background: Some(bg),
                text_color: if is_active {
                    theme.palette().primary
                } else {
                    theme.palette().text
                },
                border: iced::Border {
                    radius: 4.0.into(),
                    width: if is_active { 1.0 } else { 0.0 },
                    color: if is_active {
                        theme.palette().primary
                    } else {
                        iced::Color::TRANSPARENT
                    },
                },
                ..Default::default()
            }
        })
        .into()
}
