use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::Column;

const BUILTIN_THEME_OPTIONS: &[&str] = &[
    "Dark",
    "Light",
    "Dracula",
    "Nord",
    "Solarized Dark",
    "Solarized Light",
    "Gruvbox Dark",
    "Gruvbox Light",
    "Catppuccin Macchiato",
    "Catppuccin Mocha",
    "Tokyo Night",
    "Tokyo Night Storm",
    "Tokyo Night Light",
    "Kanagawa Wave",
    "Kanagawa Dragon",
    "Kanagawa Lotus",
    "Moonfly",
    "Nightfly",
    "Oxocarbon",
    "Ferra",
];

impl TradingTerminal {
    pub(super) fn view_builtin_theme_list(&self) -> Column<'static, Message> {
        BUILTIN_THEME_OPTIONS
            .iter()
            .fold(Column::new().spacing(4), |theme_list, &theme_name| {
                let is_active = self.active_theme == theme_name;
                let preview_theme = self.get_theme_by_name(theme_name);

                theme_list.push(self.view_theme_option_button(
                    theme_name.to_string(),
                    Message::ThemeChanged(theme_name.to_string()),
                    is_active,
                    preview_theme.palette(),
                ))
            })
    }

    pub(super) fn view_custom_theme_list(&self) -> Column<'static, Message> {
        self.custom_themes
            .iter()
            .fold(Column::new().spacing(4), |custom_list, custom_theme| {
                let name = custom_theme.name.clone();
                let full_name = format!("Custom: {}", name);
                let is_active = self.active_theme == full_name;
                let preview_theme = self.get_theme_by_name(&full_name);

                custom_list.push(self.view_theme_option_button(
                    name,
                    Message::ThemeChanged(full_name),
                    is_active,
                    preview_theme.palette(),
                ))
            })
    }
}
