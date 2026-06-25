use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::{WsUserDataStreamParams, WsUserDataStreamPurpose, ws_user_data_stream};
use iced::Subscription;

// ---------------------------------------------------------------------------
// User Data Subscriptions
// ---------------------------------------------------------------------------

impl TradingTerminal {
    fn user_data_subscription_params(
        &self,
    ) -> (
        WsUserDataStreamParams,
        Vec<WsUserDataStreamParams>,
        Vec<WsUserDataStreamParams>,
    ) {
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
            .map(|address| {
                WsUserDataStreamParams::without_mids(Some(address), dexes.clone())
                    .with_purpose(WsUserDataStreamPurpose::WalletDetail)
            })
            .collect();

        let mut cluster_addresses: Vec<String> = self
            .wallet_clusters
            .selected_cluster()
            .into_iter()
            .flat_map(|cluster| cluster.members.iter())
            .filter_map(|member| {
                self.accounts
                    .iter()
                    .find(|profile| profile.secret_id == member.profile_secret_id)
                    .and_then(|profile| Self::normalize_wallet_address(&profile.wallet_address))
            })
            .collect();
        cluster_addresses.sort();
        cluster_addresses.dedup();
        let cluster_members = cluster_addresses
            .into_iter()
            .map(|address| {
                WsUserDataStreamParams::without_mids(Some(address), dexes.clone())
                    .with_purpose(WsUserDataStreamPurpose::WalletCluster)
            })
            .collect();

        (sub_id, wallet_details, cluster_members)
    }

    pub(super) fn push_user_data_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        // Real-time user data stream (positions, orders, fills, balances) + allMids.
        // The stream filters private subscriptions internally when no address is connected.
        let (base_sub_id, wallet_detail_sub_ids, cluster_member_sub_ids) =
            self.user_data_subscription_params();
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

        for sub_id in cluster_member_sub_ids {
            subs.push(Subscription::run_with(sub_id, ws_user_data_stream).map(
                |(source_address, data)| {
                    Message::WalletClusterWsUpdate(source_address.map(Into::into), Box::new(data))
                },
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AccountProfile, ClearConfigSummary};
    use crate::wallet_cluster_state::{WalletCluster, WalletClusterMember};
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

        let (base, wallet_details, cluster_members) = terminal.user_data_subscription_params();

        assert!(base.include_mids);
        assert_eq!(base.address.as_deref(), Some(CONNECTED));
        assert_eq!(wallet_details.len(), 1);
        assert!(!wallet_details[0].include_mids);
        assert_eq!(wallet_details[0].address.as_deref(), Some(OTHER));
        assert!(cluster_members.is_empty());
    }

    #[test]
    fn config_clear_removes_wallet_detail_stream_params() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(CONNECTED.to_string());
        terminal.wallet_detail_windows.insert(
            iced::window::Id::unique(),
            WalletDetailsWindowState::new(OTHER.to_string()),
        );

        let (_, before_clear, _) = terminal.user_data_subscription_params();
        assert_eq!(before_clear.len(), 1);

        let _task = terminal.apply_config_clear_to_runtime(ClearConfigSummary {
            files_removed: 0,
            file_cleanup_failed: false,
            keychain_entries_cleared: 0,
            warnings: Vec::new(),
        });
        let (_, after_clear, cluster_after_clear) = terminal.user_data_subscription_params();

        assert!(after_clear.is_empty());
        assert!(cluster_after_clear.is_empty());
    }

    #[test]
    fn selected_wallet_cluster_members_get_private_streams_without_mids() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(CONNECTED.to_string());
        terminal.accounts = vec![AccountProfile {
            secret_id: "member-profile".to_string(),
            name: "Member".to_string(),
            wallet_address: OTHER.to_ascii_uppercase(),
            agent_key: "agent-key".to_string().into(),
            hydromancer_api_key: String::new().into(),
        }];
        terminal.wallet_clusters.clusters = vec![WalletCluster {
            id: "cluster".to_string(),
            name: "Cluster".to_string(),
            members: vec![WalletClusterMember {
                profile_secret_id: "member-profile".to_string(),
                weight: 1.0,
                weight_input: "1".to_string(),
            }],
        }];
        terminal.wallet_clusters.selected_cluster_id = Some("cluster".to_string());

        let (_, wallet_details, cluster_members) = terminal.user_data_subscription_params();

        assert!(wallet_details.is_empty());
        assert_eq!(cluster_members.len(), 1);
        assert!(!cluster_members[0].include_mids);
        assert_eq!(cluster_members[0].address.as_deref(), Some(OTHER));
        assert_eq!(
            cluster_members[0].purpose,
            WsUserDataStreamPurpose::WalletCluster
        );
    }

    #[test]
    fn cluster_member_and_detail_window_for_same_address_have_distinct_identities() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // OTHER is open as BOTH a wallet-detail window and a selected cluster
        // member. Their stream params must differ so iced keeps both recipes
        // alive instead of deduping them by hash (which would silently drop one
        // consumer's updates). `.map()` does not change a subscription's
        // identity, so the params' Hash/Eq is what disambiguates them.
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(CONNECTED.to_string());
        terminal.accounts = vec![AccountProfile {
            secret_id: "member-profile".to_string(),
            name: "Member".to_string(),
            wallet_address: OTHER.to_string(),
            agent_key: "agent-key".to_string().into(),
            hydromancer_api_key: String::new().into(),
        }];
        terminal.wallet_detail_windows.insert(
            iced::window::Id::unique(),
            WalletDetailsWindowState::new(OTHER.to_string()),
        );
        terminal.wallet_clusters.clusters = vec![WalletCluster {
            id: "cluster".to_string(),
            name: "Cluster".to_string(),
            members: vec![WalletClusterMember {
                profile_secret_id: "member-profile".to_string(),
                weight: 1.0,
                weight_input: "1".to_string(),
            }],
        }];
        terminal.wallet_clusters.selected_cluster_id = Some("cluster".to_string());

        let (_, wallet_details, cluster_members) = terminal.user_data_subscription_params();
        assert_eq!(wallet_details.len(), 1);
        assert_eq!(cluster_members.len(), 1);
        assert_eq!(wallet_details[0].address, cluster_members[0].address);

        // Same address, different consumer -> not equal and different hashes.
        assert_ne!(wallet_details[0], cluster_members[0]);
        let hash = |params: &WsUserDataStreamParams| {
            let mut hasher = DefaultHasher::new();
            params.hash(&mut hasher);
            hasher.finish()
        };
        assert_ne!(hash(&wallet_details[0]), hash(&cluster_members[0]));
        assert_eq!(
            wallet_details[0].purpose,
            WsUserDataStreamPurpose::WalletDetail
        );
    }
}
