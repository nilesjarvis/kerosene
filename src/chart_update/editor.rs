mod add_chart;
mod controls;
mod symbol;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_chart_editor(&mut self, message: Message) -> Task<Message> {
        match message {
            message @ Message::ChartSymbolSelected(_, _) => self.select_chart_symbol(message),
            message @ Message::ChartSecondarySymbolSelected(_, _) => {
                self.select_chart_secondary_symbol(message)
            }
            message @ (Message::ToggleChartInvert(_)
            | Message::ToggleChartTradeMarkers(_)
            | Message::ChartOpenEditor(_)
            | Message::ChartCloseEditor(_)
            | Message::ChartEditorSearchChanged(_, _)
            | Message::ChartEditorSubmit(_)
            | Message::ChartSecondarySymbolRemoved(_)
            | Message::ChartSecondaryOpenEditor(_)
            | Message::ChartSecondaryCloseEditor(_)
            | Message::ChartSecondaryEditorSearchChanged(_, _)
            | Message::ChartSecondaryEditorSubmit(_)) => self.update_chart_editor_controls(message),
            message @ Message::AddChart(_) => self.add_chart_pane(message),
            _ => Task::none(),
        }
    }
}
