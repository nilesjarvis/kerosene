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
        let display_font_controls = font_picker_section(FontPickerSection {
            theme: &theme,
            title: "Display Font",
            description: "Restart Kerosene after changing fonts.",
            system_label: "System Default",
            active_font: &self.display_font,
            custom_fonts: &self.custom_fonts,
            change_message: Message::DisplayFontChanged,
            import_message: Message::ImportDisplayFont,
            import_label: "Import Display Font",
        });
        let monospace_font_controls = font_picker_section(FontPickerSection {
            theme: &theme,
            title: "Monospace Font",
            description: "Used for prices, tables, chart scales, and other aligned terminal readouts.",
            system_label: "System Monospace",
            active_font: &self.monospace_font,
            custom_fonts: &self.custom_fonts,
            change_message: Message::MonospaceFontChanged,
            import_message: Message::ImportMonospaceFont,
            import_label: "Import Monospace Font",
        });

        column![
            display_font_controls,
            rule::horizontal(1),
            monospace_font_controls,
        ]
        .spacing(12)
        .into()
    }
}

struct FontPickerSection<'a> {
    theme: &'a Theme,
    title: &'static str,
    description: &'static str,
    system_label: &'static str,
    active_font: &'a config::DisplayFontConfig,
    custom_fonts: &'a [config::CustomFontConfig],
    change_message: fn(config::DisplayFontConfig) -> Message,
    import_message: Message,
    import_label: &'static str,
}

fn font_picker_section(section: FontPickerSection<'_>) -> Element<'static, Message> {
    let theme = section.theme;
    let weak_text = theme.extended_palette().background.weak.text;
    let mut font_list = Column::new().spacing(4).push(display_font_option_button(
        section.system_label.to_string(),
        (section.change_message)(config::DisplayFontConfig::System),
        matches!(section.active_font, config::DisplayFontConfig::System),
    ));

    for family in config::BUNDLED_DISPLAY_FONT_FAMILIES {
        let selected_font = config::DisplayFontConfig::Custom {
            family: (*family).to_string(),
        };
        let is_active = section.active_font == &selected_font;
        font_list = font_list.push(display_font_option_button(
            (*family).to_string(),
            (section.change_message)(selected_font),
            is_active,
        ));
    }

    for font in section.custom_fonts {
        if config::bundled_display_font_family(&font.family).is_some() {
            continue;
        }

        let selected_font = config::DisplayFontConfig::Custom {
            family: font.family.clone(),
        };
        let is_active = section.active_font == &selected_font;
        font_list = font_list.push(display_font_option_button(
            font.family.clone(),
            (section.change_message)(selected_font),
            is_active,
        ));
    }

    column![
        text(section.title).size(14).color(theme.palette().text),
        text(section.description).size(11).color(weak_text),
        font_list,
        row![
            button(text(section.import_label).size(12))
                .padding([6, 10])
                .on_press(section.import_message),
            button(text("Reset").size(12))
                .padding([6, 10])
                .on_press((section.change_message)(config::DisplayFontConfig::System)),
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
