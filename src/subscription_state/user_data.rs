use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::{WsUserDataStreamParams, ws_user_data_stream};
use iced::Subscription;

// ---------------------------------------------------------------------------
// User Data Subscriptions
// ---------------------------------------------------------------------------

impl TradingTerminal {
    fn user_data_subscription_params(
        &self,
    ) -> (WsUserDataStreamParams, Vec<WsUserDataStreamParams>) {
        let dexes = self.visible_mids_dexes();
        let connected_address = self
            .connected_address
            .as_deref()
            .and_then(Self::normalize_wallet_address);
        let sub_id = WsUserDataStreamParams::new(connected_address.clone(), dexes.clone());

        let mut wallet_detail_addresses: Vec<String> = self
            .wallet_detail_windows
            .values()
            .filter_map(|state| Self::normalize_wallet_address(&state.address))
            .filter(|address| connected_address.as_ref() != Some(address))
            .collect();
        wallet_detail_addresses.sort();
        wallet_detail_addresses.dedup();
        let wallet_details = wallet_detail_addresses
            .into_iter()
            .map(|address| WsUserDataStreamParams::without_mids(Some(address), dexes.clone()))
            .collect();

        (sub_id, wallet_details)
    }

    pub(super) fn push_user_data_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        // Real-time user data stream (positions, orders, fills, balances) + allMids.
        // The stream filters private subscriptions internally when no address is connected.
        let (base_sub_id, wallet_detail_sub_ids) = self.user_data_subscription_params();
        subs.push(
            Subscription::run_with(base_sub_id, ws_user_data_stream).map(
                |(source_address, data)| {
                    Message::WsUserDataUpdate(source_address.map(Into::into), Box::new(data))
                },
            ),
        );

        for sub_id in wallet_detail_sub_ids {
            subs.push(Subscription::run_with(sub_id, ws_user_data_stream).map(
                |(source_address, data)| {
                    Message::WalletDetailsWsUpdate(source_address.map(Into::into), Box::new(data))
                },
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ClearConfigSummary;
    use crate::wallet_state::WalletDetailsWindowState;

    const CONNECTED: &str = "0xabc0000000000000000000000000000000000000";
    const OTHER: &str = "0xdef0000000000000000000000000000000000000";

    #[test]
    fn wallet_detail_stream_params_dedup_and_opt_out_of_mids() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(CONNECTED.to_ascii_uppercase());

        terminal.wallet_detail_windows.insert(
            iced::window::Id::unique(),
            WalletDetailsWindowState::new(CONNECTED.to_string()),
        );
        terminal.wallet_detail_windows.insert(
            iced::window::Id::unique(),
            WalletDetailsWindowState::new(OTHER.to_ascii_uppercase()),
        );
        terminal.wallet_detail_windows.insert(
            iced::window::Id::unique(),
            WalletDetailsWindowState::new(OTHER.to_string()),
        );

        let (base, wallet_details) = terminal.user_data_subscription_params();

        assert!(base.include_mids);
        assert_eq!(base.address.as_deref(), Some(CONNECTED));
        assert_eq!(wallet_details.len(), 1);
        assert!(!wallet_details[0].include_mids);
        assert_eq!(wallet_details[0].address.as_deref(), Some(OTHER));
    }

    #[test]
    fn config_clear_removes_wallet_detail_stream_params() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(CONNECTED.to_string());
        terminal.wallet_detail_windows.insert(
            iced::window::Id::unique(),
            WalletDetailsWindowState::new(OTHER.to_string()),
        );

        let (_, before_clear) = terminal.user_data_subscription_params();
        assert_eq!(before_clear.len(), 1);

        let _task = terminal.apply_config_clear_to_runtime(ClearConfigSummary {
            files_removed: 0,
            file_cleanup_failed: false,
            keychain_entries_cleared: 0,
            warnings: Vec::new(),
        });
        let (_, after_clear) = terminal.user_data_subscription_params();

        assert!(after_clear.is_empty());
    }
}
