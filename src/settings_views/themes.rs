mod button;
mod chrome;
mod fonts;
mod lists;
mod notifications;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::settings_state::ThemeSettingsPage;
use iced::widget::{button as widget_button, column, row, rule, scrollable, text};
use iced::{Alignment, Element, Fill, Length, Theme};

impl TradingTerminal {
    pub(crate) fn view_settings_themes_section(&self) -> Element<'_, Message> {
        match self.settings_theme_page {
            ThemeSettingsPage::Overview => self.view_settings_theme_overview(),
            ThemeSettingsPage::WidgetChrome => self.view_settings_theme_subpage(
                "Appearance",
                self.view_settings_widget_chrome_section(),
            ),
            ThemeSettingsPage::Crosshair => self.view_settings_theme_subpage(
                "Crosshair & HUD",
                self.view_settings_crosshair_section(),
            ),
            ThemeSettingsPage::Notifications => self.view_settings_theme_subpage(
                "Notifications",
                self.view_settings_notifications_section(),
            ),
            ThemeSettingsPage::Fonts => {
                self.view_settings_theme_subpage("Fonts", self.view_settings_display_font_section())
            }
            ThemeSettingsPage::BuiltInThemes => {
                self.view_settings_theme_subpage("Built-in Themes", self.view_builtin_theme_page())
            }
            ThemeSettingsPage::CustomThemes => {
                self.view_settings_theme_subpage("Custom Themes", self.view_custom_theme_page())
            }
        }
    }

    fn view_settings_theme_overview(&self) -> Element<'_, Message> {
        let current_theme = self.theme();
        let dots = if self.chart_dotted_background {
            "dots on"
        } else {
            "dots off"
        };
        let gradient = if self.chart_gradient_background {
            "gradient on"
        } else {
            "gradient off"
        };
        let lens = if self.chart_fisheye_enabled {
            "lens on"
        } else {
            "lens off"
        };
        let hollow = self.chart_hollow_candle_mode.label();
        let fringe = if self.chart_chromatic_aberration_enabled {
            "fringe on"
        } else {
            "fringe off"
        };
        let blur = if self.chart_edge_blur_enabled {
            "blur on"
        } else {
            "blur off"
        };
        let border = if self.outer_widget_border_enabled {
            "border on"
        } else {
            "border off"
        };
        let os_bar = if self.custom_window_chrome_enabled {
            "OS bar custom"
        } else {
            "OS bar native"
        };
        let series_style = self.chart_series_style.label().to_lowercase();
        let chrome_summary = format!(
            "{:.0}% scale, {:.0}px divider, {series_style}, {dots}, {gradient}, hollow {hollow}, {lens}, {fringe}, {blur}, {border}, {os_bar}",
            self.ui_scale * 100.0,
            self.pane_border_thickness
        );
        let guides = if self.chart_crosshair_guides_enabled {
            "guides on"
        } else {
            "guides off"
        };
        let crosshair_summary = format!(
            "{}, {guides}, {:.0}% size",
            self.chart_crosshair_style.label(),
            self.chart_crosshair_scale * 100.0
        );
        let toast_animations = if self.toast_animations_enabled {
            "animated"
        } else {
            "instant"
        };
        let notifications_summary = format!(
            "{}, {toast_animations}",
            self.toast_position.label().to_lowercase()
        );
        let font_summary = format!(
            "Display: {}; Mono: {}",
            self.display_font, self.monospace_font
        );
        let custom_count = self.custom_themes.len();
        let custom_summary = if custom_count == 1 {
            "1 saved theme".to_string()
        } else {
            format!("{custom_count} saved themes")
        };

        let mut page_links = column![
            theme_overview_button(
                "Appearance",
                chrome_summary,
                ThemeSettingsPage::WidgetChrome,
            ),
            theme_overview_button(
                "Crosshair & HUD",
                crosshair_summary,
                ThemeSettingsPage::Crosshair,
            ),
            theme_overview_button(
                "Notifications",
                notifications_summary,
                ThemeSettingsPage::Notifications,
            ),
            theme_overview_button("Fonts", font_summary, ThemeSettingsPage::Fonts),
            theme_overview_button(
                "Built-in Themes",
                format!("Active: {}", self.active_theme),
                ThemeSettingsPage::BuiltInThemes,
            ),
        ]
        .spacing(8);

        if !self.custom_themes.is_empty() {
            page_links = page_links.push(theme_overview_button(
                "Custom Themes",
                custom_summary,
                ThemeSettingsPage::CustomThemes,
            ));
        }

        column![
            text("Theme").size(16).color(current_theme.palette().text),
            rule::horizontal(1),
            scrollable(page_links).height(Fill),
        ]
        .spacing(12)
        .into()
    }

    fn view_settings_theme_subpage<'a>(
        &'a self,
        title: &'static str,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let current_theme = self.theme();

        column![
            row![
                widget_button(text("< Back").size(12))
                    .padding([6, 10])
                    .on_press(Message::ThemeSettingsPageSelected(
                        ThemeSettingsPage::Overview
                    )),
                text(title).size(16).color(current_theme.palette().text),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            rule::horizontal(1),
            scrollable(content).height(Fill),
        ]
        .spacing(12)
        .into()
    }

    fn view_builtin_theme_page(&self) -> Element<'_, Message> {
        self.view_builtin_theme_list().into()
    }

    fn view_custom_theme_page(&self) -> Element<'_, Message> {
        let current_theme = self.theme();

        if self.custom_themes.is_empty() {
            text("No custom themes configured.")
                .size(12)
                .color(current_theme.extended_palette().background.weak.text)
                .into()
        } else {
            self.view_custom_theme_list().into()
        }
    }
}

fn theme_overview_button(
    label: &'static str,
    detail: String,
    page: ThemeSettingsPage,
) -> Element<'static, Message> {
    widget_button(
        row![
            column![text(label).size(13), text(detail).size(11),]
                .spacing(3)
                .width(Fill),
            text(">")
                .size(14)
                .font(crate::app_fonts::monospace_font())
                .align_x(iced::alignment::Horizontal::Right)
                .width(Length::Fixed(18.0)),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .width(Fill),
    )
    .padding([8, 12])
    .width(Fill)
    .on_press(Message::ThemeSettingsPageSelected(page))
    .style(|theme: &Theme, status| {
        let extended = theme.extended_palette();
        let background = match status {
            iced::widget::button::Status::Hovered => extended.background.strong.color.into(),
            _ => extended.background.weak.color.into(),
        };

        iced::widget::button::Style {
            background: Some(background),
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
