use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Space, button, column, row, scrollable, text};
use iced::{Color, Element, Theme};

impl TradingTerminal {
    pub(crate) fn view_settings_hotkeys_section(&self) -> Element<'_, Message> {
        let current_theme = self.theme();
        let mut hotkeys_col: iced::widget::Column<'_, Message> = column![
            text("Global Hotkeys")
                .size(16)
                .color(current_theme.palette().text),
            text("Click a button to record a hotkey. Press Escape to cancel recording.")
                .size(12)
                .style(|t: &Theme| text::Style {
                    color: Some(Color {
                        a: 0.7,
                        ..t.palette().text
                    })
                }),
            Space::new().height(10.0),
        ]
        .spacing(12);

        for (action, label) in self.available_hotkey_actions() {
            let is_recording = self.recording_hotkey_for.as_ref() == Some(&action);

            let current_hk = self.hotkeys.iter().find(|h| h.action == action);

            let btn_text = if is_recording {
                "Press any key (Esc to cancel)...".to_string()
            } else if let Some(hk) = current_hk {
                Self::hotkey_display(hk)
            } else {
                "None (Click to set)".to_string()
            };

            let mut hk_btn = button(text(btn_text).size(12)).padding([6, 12]);

            if !is_recording {
                hk_btn = hk_btn.on_press(Message::StartRecordingHotkey(action));
            }

            hotkeys_col = hotkeys_col.push(
                row![
                    text(label).size(14).width(iced::Length::Fixed(220.0)),
                    hk_btn,
                ]
                .align_y(iced::Alignment::Center)
                .spacing(16),
            );
        }

        scrollable(hotkeys_col).into()
    }
}
