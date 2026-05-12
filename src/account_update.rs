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
            Message::ToggleHiddenPosition(coin) => self.toggle_hidden_position(coin),
            Message::ToggleShowHiddenPositions => self.toggle_show_hidden_positions(),
            Message::OpenPnlCard(target) => self.open_pnl_card_window(target),
            Message::SetPnlCardDisplayMode(window_id, mode) => {
                self.set_pnl_card_display_mode(window_id, mode)
            }
            Message::SetPnlCardPercentMode(window_id, mode) => {
                self.set_pnl_card_percent_mode(window_id, mode)
            }
            Message::TogglePnlCardPricePrivacy(window_id, obscure) => {
                self.toggle_pnl_card_price_privacy(window_id, obscure)
            }
            Message::TogglePnlCardPositionSize(window_id, show) => {
                self.toggle_pnl_card_position_size(window_id, show)
            }
            Message::CopyPnlCard(window_id) => self.copy_pnl_card_image(window_id),
            Message::PnlCardCopied(result) => self.handle_pnl_card_copied(result),
            Message::SavePnlCard(window_id) => self.save_pnl_card_image(window_id),
            Message::PnlCardSaved(result) => self.handle_pnl_card_saved(result),
            Message::WalletKeyInputChanged(value) => self.update_wallet_key_input(value),
            Message::WalletAddressInputChanged(value) => self.update_wallet_address_input(value),
            Message::AccountLabelChanged(value) => self.update_account_label(value),
            Message::ToggleAccountPicker => self.toggle_account_picker(),
            Message::AccountPickerSelected(index) => self.select_account_from_picker(index),
            Message::AddAccount => self.add_account_from_picker(),
            Message::GhostWallet(address) => self.add_ghost_wallet_from_picker(address),
            Message::ForgetGhostAccount(index) => self.forget_ghost_account_from_picker(index),
            Message::DeleteSavedAccount(index) => self.delete_saved_account_task(index),
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
