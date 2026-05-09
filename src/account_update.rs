mod connection;
mod profile;
mod stream;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_account(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PositionsSortChanged(col) => self.update_positions_sort(col),
            Message::WalletKeyInputChanged(value) => self.update_wallet_key_input(value),
            Message::WalletAddressInputChanged(value) => self.update_wallet_address_input(value),
            Message::AccountLabelChanged(value) => self.update_account_label(value),
            Message::ToggleAccountPicker => self.toggle_account_picker(),
            Message::AccountPickerSelected(index) => self.select_account_from_picker(index),
            Message::AddAccount => self.add_account_from_picker(),
            Message::GhostWallet(address) => self.add_ghost_wallet_from_picker(address),
            Message::ForgetGhostAccount(index) => self.forget_ghost_account_from_picker(index),
            Message::SaveCredentials => self.save_active_account_credentials(),
            Message::ConnectWallet => self.connect_wallet(),
            Message::DisconnectWallet => self.disconnect_wallet(),
            Message::AccountDataLoaded(address, result) => {
                self.apply_account_data_loaded(address, *result)
            }
            Message::RefreshAccountData => self.refresh_account_data(),
            Message::AllMidsBootstrapLoaded(_dex, Ok(mids)) => {
                self.handle_mids_update(mids);
                Task::none()
            }
            Message::WsUserDataUpdate(source_address, ws_data) => {
                self.apply_ws_user_data_update(source_address, *ws_data)
            }
            _ => Task::none(),
        }
    }
}
