use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::ws_user_data_stream;
use iced::Subscription;

// ---------------------------------------------------------------------------
// User Data Subscriptions
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_user_data_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        // Real-time user data stream (positions, orders, fills, balances) + allMids.
        // The stream filters private subscriptions internally when no address is connected.
        let sub_id = (
            self.connected_address.clone(),
            Self::known_mids_dexes_from_symbols(&self.exchange_symbols),
        );
        subs.push(Subscription::run_with(sub_id, ws_user_data_stream).map(
            |(source_address, data)| Message::WsUserDataUpdate(source_address, Box::new(data)),
        ));

        let mut wallet_detail_addresses: Vec<String> = self
            .wallet_detail_windows
            .values()
            .filter_map(|state| Self::normalize_wallet_address(&state.address))
            .filter(|address| self.connected_address.as_ref() != Some(address))
            .collect();
        wallet_detail_addresses.sort();
        wallet_detail_addresses.dedup();
        for address in wallet_detail_addresses {
            let sub_id = (
                Some(address),
                Self::known_mids_dexes_from_symbols(&self.exchange_symbols),
            );
            subs.push(Subscription::run_with(sub_id, ws_user_data_stream).map(
                |(source_address, data)| {
                    Message::WalletDetailsWsUpdate(source_address, Box::new(data))
                },
            ));
        }
    }
}
