mod button;
mod chrome;
mod lists;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{column, rule, scrollable, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_settings_themes_section(&self) -> Element<'_, Message> {
        let current_theme = self.theme();
        let chrome_controls = self.view_settings_widget_chrome_section();
        let theme_list = self.view_builtin_theme_list();
        let custom_list = self.view_custom_theme_list();
        let has_custom_themes = !self.custom_themes.is_empty();

        if !has_custom_themes {
            column![
                text("Theme").size(16).color(current_theme.palette().text),
                rule::horizontal(1),
                scrollable(column![chrome_controls, rule::horizontal(1), theme_list].spacing(12))
                    .height(Fill),
            ]
            .spacing(12)
            .into()
        } else {
            column![
                text("Theme").size(16).color(current_theme.palette().text),
                rule::horizontal(1),
                scrollable(
                    column![
                        chrome_controls,
                        rule::horizontal(1),
                        theme_list,
                        text("Custom Themes")
                            .size(16)
                            .color(current_theme.palette().text),
                        rule::horizontal(1),
                        custom_list
                    ]
                    .spacing(12)
                )
                .height(Fill),
            ]
            .spacing(12)
            .into()
        }
    }
}
