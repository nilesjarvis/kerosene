use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, button, column, row, rule, text, text_input};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(crate) fn view_settings_layouts_section(&self) -> Element<'_, Message> {
        let current_theme = self.theme();
        let mut layouts_list = Column::<'_, Message>::new().spacing(4);
        for layout in &self.saved_layouts {
            let name = layout.name.clone();
            let is_active = self.active_layout_name.as_deref() == Some(&name);

            let load_btn = button(
                row![
                    text(if is_active { "\u{2713} " } else { "  " })
                        .size(12)
                        .color(if is_active {
                            current_theme.palette().primary
                        } else {
                            current_theme.palette().text
                        }),
                    text(name.clone()).size(12).width(Fill),
                ]
                .align_y(iced::Alignment::Center),
            )
            .width(Fill)
            .padding([4, 8])
            .style(move |theme: &Theme, status| {
                let mut style = iced::widget::button::secondary(theme, status);
                if is_active {
                    style.background =
                        Some(theme.extended_palette().background.strong.color.into());
                    style.text_color = theme.palette().primary;
                } else {
                    style.background = match status {
                        iced::widget::button::Status::Hovered => {
                            Some(theme.extended_palette().background.strong.color.into())
                        }
                        _ => Some(iced::Color::TRANSPARENT.into()),
                    };
                    style.text_color = theme.palette().text;
                }
                style
            })
            .on_press(Message::LoadLayout(layout.clone()));

            let export_btn = button(text("Export").size(10))
                .padding([4, 8])
                .on_press(Message::ExportLayout(layout.clone()));

            let delete_btn = button(text("X").size(10).color(current_theme.palette().danger))
                .padding([4, 8])
                .on_press(Message::DeleteLayout(name.clone()));

            layouts_list = layouts_list.push(
                row![load_btn, export_btn, delete_btn]
                    .spacing(4)
                    .align_y(iced::Alignment::Center)
                    .width(Fill),
            );
        }

        column![
            row![
                text("Workspaces / Layouts")
                    .size(16)
                    .color(current_theme.palette().text)
                    .width(Fill),
                button(text("Import").size(12))
                    .padding([4, 8])
                    .on_press(Message::ImportLayout),
            ]
            .align_y(iced::Alignment::Center),
            rule::horizontal(1),
            row![
                text_input("New layout name...", &self.layout_input)
                    .on_input(Message::LayoutInputChanged)
                    .on_submit(Message::SaveLayout(self.layout_input.clone()))
                    .padding(6)
                    .size(12)
                    .width(Fill),
                button(text("Save").size(12))
                    .padding([6, 12])
                    .on_press_maybe(if self.layout_input.trim().is_empty() {
                        None
                    } else {
                        Some(Message::SaveLayout(self.layout_input.clone()))
                    }),
            ]
            .spacing(6),
            layouts_list,
        ]
        .spacing(12)
        .into()
    }
}
