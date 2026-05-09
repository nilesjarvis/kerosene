use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

mod layouts;
mod wallet_labels;

impl TradingTerminal {
    pub(crate) fn update_layout(&mut self, message: Message) -> Task<Message> {
        match message {
            message @ (Message::LayoutInputChanged(_)
            | Message::SaveLayout(_)
            | Message::LoadLayout(_)
            | Message::DeleteLayout(_)
            | Message::ExportLayout(_)
            | Message::ImportLayout
            | Message::LayoutExported(_)
            | Message::LayoutImported(_)) => return self.update_saved_layouts(message),
            message @ (Message::ExportWalletLabels
            | Message::ImportWalletLabels
            | Message::WalletLabelsExported(_)
            | Message::WalletLabelsImported(_)) => return self.update_wallet_label_io(message),
            _ => {}
        }

        Task::none()
    }
}
