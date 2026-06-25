mod connection;
mod position_pnl;
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
            Message::WalletAddressInputChanged(value) => {
                self.update_wallet_address_input(value.into_string())
            }
            Message::ToggleAccountPicker => self.toggle_account_picker(),
            Message::AccountPickerSelected(index) => self.select_account_from_picker(index),
            Message::AccountPickerRenameToggled(index) => self.toggle_account_picker_rename(index),
            Message::AccountPickerLabelChanged(index, value) => {
                self.update_account_picker_label(index, value)
            }
            Message::AddAccount => self.add_account_from_picker(),
            Message::GhostWallet(address) => {
                self.add_ghost_wallet_from_picker(address.into_string())
            }
            Message::ForgetGhostAccount(index) => self.forget_ghost_account_from_picker(index),
            Message::DeleteSavedAccount(index) => self.delete_saved_account_task(index),
            Message::SaveCredentials => self.save_active_account_credentials(),
            Message::ConnectWallet => self.connect_wallet(),
            Message::DisconnectWallet => self.disconnect_wallet(),
            Message::AccountDataLoaded(address, context, result) => {
                self.apply_account_data_loaded(address.into_string(), context, *result)
            }
            Message::RetryTwapReconciliationAccountData(address) => {
                self.retry_twap_reconciliation_account_data(address.into_string())
            }
            Message::RefreshAccountData => self.refresh_account_data(),
            Message::AccountRefreshBackoffElapsed(due_ms) => {
                self.handle_account_refresh_backoff_elapsed(due_ms)
            }
            Message::AllMidsBootstrapLoaded(_dex, Ok(mids)) => self.handle_mids_update(mids),
            Message::PositionPnlWsAssetCtxUpdate(symbol, source_context, ctx) => {
                self.apply_position_pnl_asset_ctx_update(symbol, source_context, ctx)
            }
            Message::PositionPnlWsAssetCtxLagged(symbol, source_context, skipped) => {
                self.apply_position_pnl_asset_ctx_lag(symbol, source_context, skipped)
            }
            Message::WsUserDataUpdate(source_address, ws_data) => self.apply_ws_user_data_update(
                source_address.map(|address| address.into_string()),
                *ws_data,
            ),
            _ => Task::none(),
        }
    }
}
