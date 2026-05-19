use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers;
use crate::layout_preview::saved_layout_preview;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, button, container, row, rule, scrollable, text, text_input};
use iced::{Color, Element, Fill, Length, Theme};

const BUTTON_LABEL_CHARS: usize = 14;
const ROW_LABEL_CHARS: usize = 24;
const RENAME_ICON: &str = "✎";

// ---------------------------------------------------------------------------
// Account Summary Layout Switcher
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn layout_switcher_button_label(&self) -> String {
        layout_switcher_button_label(self.active_layout_name.as_deref(), BUTTON_LABEL_CHARS)
    }

    pub(crate) fn view_layout_switcher_dropdown(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let mut menu = Column::new()
            .spacing(2)
            .width(Fill)
            .push(
                row![
                    text("Layouts")
                        .size(11)
                        .color(theme.extended_palette().background.weak.text)
                        .width(Fill),
                    layout_header_update_button(self.active_layout_name.is_some()),
                ]
                .align_y(iced::Alignment::Center),
            )
            .push(rule::horizontal(1));

        if self.saved_layouts.is_empty() {
            menu = menu.push(
                container(
                    text("No saved layouts")
                        .size(11)
                        .color(theme.extended_palette().background.weak.text),
                )
                .padding([8, 8])
                .width(Fill),
            );
        } else {
            for (index, layout) in self.saved_layouts.iter().enumerate() {
                menu = menu.push(self.view_layout_switcher_row(index, layout, &theme));
            }
        }

        container(scrollable(menu).height(Length::Shrink))
            .padding(6)
            .width(340)
            .max_height(360.0)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: theme.extended_palette().background.strong.color,
                },
                ..Default::default()
            })
            .into()
    }

    fn view_layout_switcher_row<'a>(
        &'a self,
        index: usize,
        layout: &'a crate::config::SavedLayout,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let is_active = self.active_layout_name.as_deref() == Some(layout.name.as_str());
        let is_renaming = self.layout_rename_index == Some(index);
        let delete_message = Message::DeleteLayout(layout.name.clone());

        if is_renaming {
            return container(
                row![
                    text_input("Layout name", &self.layout_rename_input)
                        .style(helpers::text_input_style)
                        .on_input(Message::LayoutRenameChanged)
                        .on_submit(Message::LayoutRenameSubmitted(index))
                        .size(11)
                        .padding([4, 6])
                        .width(Fill),
                    layout_action_button(
                        "Save",
                        Message::LayoutRenameSubmitted(index),
                        theme.palette().primary,
                        true,
                        44.0,
                    ),
                    layout_action_button(
                        "Delete",
                        delete_message,
                        theme.palette().danger,
                        false,
                        56.0,
                    ),
                ]
                .spacing(4)
                .align_y(iced::Alignment::Center),
            )
            .padding([2, 0])
            .width(Fill)
            .into();
        }

        let marker = if is_active { ">" } else { "" };
        let label = layout_switcher_label(Some(layout.name.as_str()), ROW_LABEL_CHARS);
        let preview = saved_layout_preview(layout.pane_layout.as_ref(), theme, is_active);
        let mut load_contents = row![
            text(marker)
                .size(11)
                .color(theme.palette().primary)
                .width(Length::Fixed(12.0)),
            preview,
            text(label).size(12).color(theme.palette().text).width(Fill),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);
        if let Some(hotkey) = self.layout_switcher_hotkey_display(&layout.name) {
            load_contents = load_contents.push(
                text(hotkey)
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        let load_button: Element<'_, Message> = button(load_contents)
            .on_press(Message::LoadLayout(layout.clone()))
            .padding([7, 8])
            .width(Fill)
            .style(move |theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ if is_active => theme.extended_palette().background.weak.color,
                    _ => Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().text,
                    border: iced::Border {
                        radius: 4.0.into(),
                        width: if is_active { 1.0 } else { 0.0 },
                        color: if is_active {
                            theme.palette().primary
                        } else {
                            Color::TRANSPARENT
                        },
                    },
                    ..Default::default()
                }
            })
            .into();

        container(
            row![
                container(load_button).width(Fill),
                layout_action_button(
                    RENAME_ICON,
                    Message::LayoutRenameToggled(index),
                    theme.palette().primary,
                    false,
                    30.0,
                ),
                layout_action_button(
                    "Delete",
                    delete_message,
                    theme.palette().danger,
                    false,
                    56.0
                ),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .padding([2, 0])
        .width(Fill)
        .into()
    }

    fn layout_switcher_hotkey_display(&self, name: &str) -> Option<String> {
        let action = config::HotkeyAction::SwitchLayout {
            name: name.to_string(),
        };
        self.hotkeys
            .iter()
            .find(|hotkey| hotkey.action == action)
            .map(Self::hotkey_display)
    }
}

fn layout_header_update_button(enabled: bool) -> Element<'static, Message> {
    let button = button(text("Update").size(10).center())
        .padding([4, 8])
        .style(layout_action_style);

    if enabled {
        button.on_press(Message::UpdateActiveLayout).into()
    } else {
        button.into()
    }
}

fn layout_action_button(
    label: &'static str,
    message: Message,
    color: Color,
    active: bool,
    width: f32,
) -> Element<'static, Message> {
    button(text(label).size(10).center())
        .on_press(message)
        .padding([6, 6])
        .width(Length::Fixed(width))
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ if active => theme.extended_palette().background.weak.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: color,
                border: iced::Border {
                    radius: 4.0.into(),
                    width: if active { 1.0 } else { 0.0 },
                    color: if active { color } else { Color::TRANSPARENT },
                },
                ..Default::default()
            }
        })
        .into()
}

fn layout_action_style(theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => theme.extended_palette().background.strong.color,
        _ => theme.extended_palette().background.weak.color,
    };

    button::Style {
        background: Some(bg.into()),
        text_color: theme.palette().text,
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    }
}

fn layout_switcher_label(name: Option<&str>, max_chars: usize) -> String {
    let label = name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("Layouts");
    truncate_label(label, max_chars)
}

fn layout_switcher_button_label(name: Option<&str>, max_chars: usize) -> String {
    let Some(label) = name.map(str::trim).filter(|name| !name.is_empty()) else {
        return "Layout".to_string();
    };
    format!("Layout: {}", truncate_label(label, max_chars))
}

fn truncate_label(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return value.chars().take(max_chars).collect();
    }
    let prefix: String = value.chars().take(max_chars - 3).collect();
    format!("{prefix}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_switcher_label_falls_back_for_missing_or_empty_names() {
        assert_eq!(layout_switcher_label(None, 14), "Layouts");
        assert_eq!(layout_switcher_label(Some("   "), 14), "Layouts");
    }

    #[test]
    fn layout_switcher_label_truncates_long_names() {
        assert_eq!(
            layout_switcher_label(Some("Very Long Trading Layout"), 14),
            "Very Long T..."
        );
    }

    #[test]
    fn layout_switcher_button_label_identifies_the_dropdown() {
        assert_eq!(layout_switcher_button_label(None, 14), "Layout");
        assert_eq!(
            layout_switcher_button_label(Some("Scalp"), 14),
            "Layout: Scalp"
        );
    }
}
