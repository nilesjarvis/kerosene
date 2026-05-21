use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use iced::widget::{Column, button, column, row, rule, text};
use iced::{Element, Fill, Length, Theme};

// ---------------------------------------------------------------------------
// Font Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_settings_display_font_section(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let display_font_controls = font_picker_section(
            &theme,
            "Display Font",
            "Restart Kerosene after changing fonts.",
            "System Default",
            &self.display_font,
            &self.custom_fonts,
            Message::DisplayFontChanged,
            Message::ImportDisplayFont,
            "Import Display Font",
        );
        let monospace_font_controls = font_picker_section(
            &theme,
            "Monospace Font",
            "Used for prices, tables, chart scales, and other aligned terminal readouts.",
            "System Monospace",
            &self.monospace_font,
            &self.custom_fonts,
            Message::MonospaceFontChanged,
            Message::ImportMonospaceFont,
            "Import Monospace Font",
        );

        column![
            display_font_controls,
            rule::horizontal(1),
            monospace_font_controls,
        ]
        .spacing(12)
        .into()
    }
}

fn font_picker_section(
    theme: &Theme,
    title: &'static str,
    description: &'static str,
    system_label: &'static str,
    active_font: &config::DisplayFontConfig,
    custom_fonts: &[config::CustomFontConfig],
    change_message: fn(config::DisplayFontConfig) -> Message,
    import_message: Message,
    import_label: &'static str,
) -> Element<'static, Message> {
    let weak_text = theme.extended_palette().background.weak.text;
    let mut font_list = Column::new().spacing(4).push(display_font_option_button(
        system_label.to_string(),
        change_message(config::DisplayFontConfig::System),
        matches!(active_font, config::DisplayFontConfig::System),
    ));

    for family in config::BUNDLED_DISPLAY_FONT_FAMILIES {
        let selected_font = config::DisplayFontConfig::Custom {
            family: (*family).to_string(),
        };
        let is_active = active_font == &selected_font;
        font_list = font_list.push(display_font_option_button(
            (*family).to_string(),
            change_message(selected_font),
            is_active,
        ));
    }

    for font in custom_fonts {
        if config::bundled_display_font_family(&font.family).is_some() {
            continue;
        }

        let selected_font = config::DisplayFontConfig::Custom {
            family: font.family.clone(),
        };
        let is_active = active_font == &selected_font;
        font_list = font_list.push(display_font_option_button(
            font.family.clone(),
            change_message(selected_font),
            is_active,
        ));
    }

    column![
        text(title).size(14).color(theme.palette().text),
        text(description).size(11).color(weak_text),
        font_list,
        row![
            button(text(import_label).size(12))
                .padding([6, 10])
                .on_press(import_message),
            button(text("Reset").size(12))
                .padding([6, 10])
                .on_press(change_message(config::DisplayFontConfig::System)),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    ]
    .spacing(10)
    .into()
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
