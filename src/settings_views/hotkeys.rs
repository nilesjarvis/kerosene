use crate::app_state::TradingTerminal;
use crate::config;
use crate::hotkey_state::HotkeyActionGroup;
use crate::message::Message;
use iced::widget::{Column, Space, button, column, row, rule, scrollable, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_settings_hotkeys_section(&self) -> Element<'_, Message> {
        let current_theme = self.theme();
        let mut hotkeys_col: iced::widget::Column<'_, Message> = column![
            text("Hotkeys").size(16).color(current_theme.palette().text),
            Space::new().height(2.0),
        ]
        .spacing(14);

        for group in self.available_hotkey_action_groups() {
            hotkeys_col = hotkeys_col.push(self.view_hotkey_group(group));
        }

        scrollable(hotkeys_col).into()
    }

    fn view_hotkey_group(&self, group: HotkeyActionGroup) -> Element<'_, Message> {
        let current_theme = self.theme();
        let mut group_col = Column::new()
            .spacing(8)
            .push(
                text(group.title)
                    .size(13)
                    .color(current_theme.palette().text),
            )
            .push(rule::horizontal(1));

        for (action, label) in group.actions {
            group_col = group_col.push(self.view_hotkey_row(action, label));
        }

        group_col.into()
    }

    fn view_hotkey_row(&self, action: config::HotkeyAction, label: String) -> Element<'_, Message> {
        let is_recording = self.recording_hotkey_for.as_ref() == Some(&action);
        let current_hk = self.hotkeys.iter().find(|h| h.action == action);
        let current_prefix = (action == config::HotkeyAction::ChartTimeframePrefix)
            .then_some(self.chart_timeframe_hotkey_prefix)
            .flatten();

        let btn_text = if is_recording {
            if action == config::HotkeyAction::ChartTimeframePrefix {
                "Press prefix (Esc cancels)...".to_string()
            } else {
                "Press any key (Esc to cancel)...".to_string()
            }
        } else if let Some(hk) = current_hk {
            Self::hotkey_display(hk)
        } else if let Some(prefix) = current_prefix {
            Self::hotkey_prefix_display(&prefix)
        } else {
            "None".to_string()
        };

        let mut hk_btn = button(text(btn_text).size(12))
            .padding([6, 12])
            .width(iced::Length::Fixed(220.0));

        if !is_recording {
            hk_btn = hk_btn.on_press(Message::StartRecordingHotkey(action.clone()));
        }

        let clear_cell: Element<'_, Message> = if current_hk.is_some() || current_prefix.is_some() {
            button(text("Clear").size(12))
                .padding([6, 12])
                .on_press(Message::ClearHotkey(action.clone()))
                .into()
        } else {
            Space::new().width(64.0).into()
        };

        row![text(label).size(14).width(Fill), hk_btn, clear_cell,]
            .align_y(iced::Alignment::Center)
            .spacing(12)
            .into()
    }
}
