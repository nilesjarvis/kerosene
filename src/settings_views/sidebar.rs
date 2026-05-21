use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::settings_state::SettingsTab;
use iced::widget::{Column, button, column, row, rule, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_settings_sidebar(&self) -> Column<'_, Message> {
        let current_theme = self.theme();

        column![
            text("Settings")
                .size(18)
                .color(current_theme.palette().text),
            rule::horizontal(1),
            settings_tab_button(
                self.settings_active_tab,
                SettingsTab::Themes,
                "Themes",
                "Aa"
            ),
            settings_tab_button(
                self.settings_active_tab,
                SettingsTab::Layouts,
                "Layouts",
                "[]"
            ),
            settings_tab_button(self.settings_active_tab, SettingsTab::Risk, "Risk", "!"),
            settings_tab_button(
                self.settings_active_tab,
                SettingsTab::Integrations,
                "Integrations",
                "<>"
            ),
            settings_tab_button(
                self.settings_active_tab,
                SettingsTab::Storage,
                "Storage",
                "!!"
            ),
            settings_tab_button(
                self.settings_active_tab,
                SettingsTab::Hotkeys,
                "Hotkeys",
                "^K"
            ),
        ]
        .spacing(8)
        .width(iced::Length::Fixed(180.0))
    }
}

fn settings_tab_button(
    active_tab: SettingsTab,
    tab: SettingsTab,
    label: &'static str,
    icon: &'static str,
) -> Element<'static, Message> {
    let is_active = active_tab == tab;

    button(
        row![
            text(icon)
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .align_x(iced::alignment::Horizontal::Center)
                .width(iced::Length::Fixed(20.0)),
            text(label).size(13),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(8),
    )
    .width(Fill)
    .padding([8, 12])
    .on_press(Message::SettingsTabSelected(tab))
    .style(move |theme: &Theme, status| {
        let extended = theme.extended_palette();
        let bg = if is_active {
            extended.background.strong.color.into()
        } else {
            match status {
                button::Status::Hovered => extended.background.strong.color.into(),
                _ => iced::Color::TRANSPARENT.into(),
            }
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
                ..Default::default()
            },
            ..Default::default()
        }
    })
    .into()
}
