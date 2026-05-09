use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Element;
use iced::widget::{button, checkbox, pick_list, row, text};

impl TradingTerminal {
    pub(super) fn view_assistant_controls(&self) -> Element<'_, Message> {
        let model_picker = pick_list(
            self.assistant.models.clone(),
            self.assistant.selected_model.clone(),
            Message::AssistantModelSelected,
        )
        .placeholder("Select local model")
        .text_size(11)
        .padding([2, 6]);

        let refresh_btn = button(text("Refresh Models").size(10))
            .on_press(Message::AssistantModelsRefresh)
            .padding([2, 8]);

        let clear_btn = button(text("Clear Chat").size(10))
            .on_press(Message::AssistantClearChat)
            .padding([2, 8]);

        row![model_picker, refresh_btn, clear_btn]
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .into()
    }

    pub(super) fn view_assistant_toggles(&self) -> Element<'_, Message> {
        row![
            checkbox(self.assistant.use_account_context)
                .label("Use account context")
                .on_toggle(Message::AssistantToggleAccountContext)
                .size(11),
            checkbox(self.assistant.allow_code_execution)
                .label("Allow code execution")
                .on_toggle(Message::AssistantToggleCodeExecution)
                .size(11),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .into()
    }
}
