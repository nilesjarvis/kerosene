use crate::account::transfers::{
    TransferHistoryKind, TransferProvider, TransferSnapshot, fetch_ledger_history,
    fetch_unit_history,
};
use crate::account_state::BottomTab;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;

impl TradingTerminal {
    pub(crate) fn transfer_history_is_visible(&self) -> bool {
        self.pane_is_open(|kind| {
            matches!(
                kind,
                PaneKind::BottomTabs {
                    active_tab: BottomTab::DepositsWithdrawals | BottomTab::Transfers
                }
            )
        })
    }

    pub(crate) fn refresh_transfer_history(&mut self) -> Task<Message> {
        if !self.transfer_history_is_visible() {
            return Task::none();
        }
        let Some(address) = self.connected_address.clone() else {
            return Task::none();
        };
        let Some(generation) = self.transfer_history.begin(&address) else {
            return Task::none();
        };
        let start = self.transfer_history.native.next_start.unwrap_or_default();
        let end = Self::now_ms();
        let native_address = address.clone();
        Task::batch([
            Task::perform(
                fetch_ledger_history(address.clone(), start, end),
                move |result| {
                    Message::TransferHistoryLoaded(
                        native_address.clone().into(),
                        generation,
                        TransferProvider::Hyperliquid,
                        Box::new(result),
                    )
                },
            ),
            Task::perform(fetch_unit_history(address.clone()), move |result| {
                Message::TransferHistoryLoaded(
                    address.clone().into(),
                    generation,
                    TransferProvider::Unit,
                    Box::new(result),
                )
            }),
        ])
    }

    pub(super) fn apply_transfer_history(
        &mut self,
        address: String,
        generation: u64,
        provider: TransferProvider,
        result: Result<TransferSnapshot, String>,
    ) -> Task<Message> {
        if self.connected_address.as_deref() == Some(address.as_str()) {
            self.transfer_history
                .apply(&address, generation, provider, result);
        }
        Task::none()
    }

    pub(super) fn change_transfer_history_page(
        &mut self,
        kind: TransferHistoryKind,
        next: bool,
    ) -> Task<Message> {
        self.transfer_history.change_page(kind, next);
        Task::none()
    }

    pub(super) fn toggle_transfer_details(
        &mut self,
        kind: TransferHistoryKind,
        index: usize,
    ) -> Task<Message> {
        self.transfer_history.toggle_details(kind, index);
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_result_cannot_populate_a_disconnected_account() {
        let mut terminal = TradingTerminal::boot().0;
        let generation = terminal.transfer_history.begin("account").expect("request");
        let _ = terminal.apply_transfer_history(
            "account".into(),
            generation,
            TransferProvider::Unit,
            Ok(TransferSnapshot::default()),
        );
        assert!(!terminal.transfer_history.unit.loaded);
    }
}
