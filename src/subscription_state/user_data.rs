use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::{WsUserDataStreamParams, WsUserDataStreamPurpose, ws_user_data_stream};
use iced::Subscription;

// ---------------------------------------------------------------------------
// User Data Subscriptions
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn user_data_subscription_params(
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
        let sub_id = WsUserDataStreamParams::new(connected_address.clone(), dexes.clone())
            .with_generation(self.account_user_data_stream_generation);

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
                let generation = self
                    .wallet_detail_user_data_stream_generations
                    .get(&address)
                    .copied()
                    .unwrap_or_default();
                WsUserDataStreamParams::without_mids(Some(address), dexes.clone())
                    .with_purpose(WsUserDataStreamPurpose::WalletDetail)
                    .with_generation(generation)
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
                    .with_generation(self.wallet_cluster_user_data_stream_generation)
            })
            .collect();

        (sub_id, wallet_details, cluster_members)
    }

    pub(crate) fn rotate_account_user_data_stream(&mut self) {
        self.account_user_data_stream_generation =
            self.account_user_data_stream_generation.wrapping_add(1);
    }

    pub(crate) fn rotate_wallet_detail_user_data_stream(&mut self, address: &str) {
        let Some(address) = Self::normalize_wallet_address(address) else {
            return;
        };
        let generation = self.next_wallet_detail_user_data_stream_generation;
        self.next_wallet_detail_user_data_stream_generation = generation.wrapping_add(1);
        self.wallet_detail_user_data_stream_generations
            .insert(address, generation);
    }

    pub(crate) fn rotate_wallet_detail_user_data_stream_if_open(&mut self, address: &str) {
        let Some(address) = Self::normalize_wallet_address(address) else {
            return;
        };
        if self
            .wallet_detail_windows
            .values()
            .any(|state| Self::normalize_wallet_address(&state.address).as_ref() == Some(&address))
        {
            self.rotate_wallet_detail_user_data_stream(&address);
        }
    }

    pub(crate) fn remove_wallet_detail_user_data_stream(&mut self, address: &str) {
        let Some(address) = Self::normalize_wallet_address(address) else {
            return;
        };
        let still_open = self
            .wallet_detail_windows
            .values()
            .any(|state| Self::normalize_wallet_address(&state.address).as_ref() == Some(&address));
        if !still_open {
            self.wallet_detail_user_data_stream_generations
                .remove(&address);
        }
    }

    pub(crate) fn rotate_wallet_cluster_user_data_streams(&mut self) {
        self.wallet_cluster_user_data_stream_generation = self
            .wallet_cluster_user_data_stream_generation
            .wrapping_add(1);
    }

    pub(crate) fn selected_wallet_cluster_uses_profile(&self, profile_secret_id: &str) -> bool {
        self.wallet_clusters
            .selected_cluster()
            .is_some_and(|cluster| {
                cluster
                    .members
                    .iter()
                    .any(|member| member.profile_secret_id == profile_secret_id)
            })
    }

    pub(crate) fn rotate_all_user_data_streams(&mut self) {
        self.rotate_account_user_data_stream();
        let mut wallet_detail_addresses: Vec<_> = self
            .wallet_detail_windows
            .values()
            .filter_map(|state| Self::normalize_wallet_address(&state.address))
            .collect();
        wallet_detail_addresses.sort();
        wallet_detail_addresses.dedup();
        for address in wallet_detail_addresses {
            self.rotate_wallet_detail_user_data_stream(&address);
        }
        self.rotate_wallet_cluster_user_data_streams();
    }

    /// Confirms that a queued iced message still belongs to the exact recipe
    /// the current application state requests and that the stream emitted it
    /// for that recipe's normalized source address.
    pub(crate) fn user_data_stream_message_is_current(
        &self,
        params: &WsUserDataStreamParams,
        source_address: Option<&str>,
    ) -> bool {
        let source_address = match source_address {
            Some(address) => {
                let Some(address) = Self::normalize_wallet_address(address) else {
                    return false;
                };
                Some(address)
            }
            None => None,
        };
        if source_address != params.address {
            return false;
        }

        let (account, wallet_details, cluster_members) = self.user_data_subscription_params();
        match params.purpose {
            WsUserDataStreamPurpose::Account => params == &account,
            WsUserDataStreamPurpose::WalletDetail => wallet_details.iter().any(|id| id == params),
            WsUserDataStreamPurpose::WalletCluster => cluster_members.iter().any(|id| id == params),
        }
    }

    pub(super) fn push_user_data_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        // Real-time user data stream (positions, orders, fills, balances) + allMids.
        // The stream filters private subscriptions internally when no address is connected.
        let (base_sub_id, wallet_detail_sub_ids, cluster_member_sub_ids) =
            self.user_data_subscription_params();
        let base_context = base_sub_id.clone();
        subs.push(
            Subscription::run_with(base_sub_id, ws_user_data_stream)
                .with(base_context)
                .map(|(params, (source_address, data))| {
                    Message::WsUserDataUpdate(
                        params,
                        source_address.map(Into::into),
                        Box::new(data),
                    )
                }),
        );

        for sub_id in wallet_detail_sub_ids {
            let context = sub_id.clone();
            subs.push(
                Subscription::run_with(sub_id, ws_user_data_stream)
                    .with(context)
                    .map(|(params, (source_address, data))| {
                        Message::WalletDetailsWsUpdate(
                            params,
                            source_address.map(Into::into),
                            Box::new(data),
                        )
                    }),
            );
        }

        for sub_id in cluster_member_sub_ids {
            let context = sub_id.clone();
            subs.push(
                Subscription::run_with(sub_id, ws_user_data_stream)
                    .with(context)
                    .map(|(params, (source_address, data))| {
                        Message::WalletClusterWsUpdate(
                            params,
                            source_address.map(Into::into),
                            Box::new(data),
                        )
                    }),
            );
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
    fn account_recipe_generation_and_source_are_required_for_current_message() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(CONNECTED.to_string());
        let previous = terminal.user_data_subscription_params().0;

        terminal.rotate_account_user_data_stream();
        let current = terminal.user_data_subscription_params().0;

        assert_eq!(current.address, previous.address);
        assert_ne!(current.generation, previous.generation);
        assert!(!terminal.user_data_stream_message_is_current(&previous, Some(CONNECTED)));
        assert!(terminal.user_data_stream_message_is_current(&current, Some(CONNECTED)));
        assert!(!terminal.user_data_stream_message_is_current(&current, Some(OTHER)));
    }

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
    fn reopened_wallet_detail_gets_new_generation_without_rotating_other_address() {
        let mut terminal = TradingTerminal::boot().0;
        let first_id = iced::window::Id::unique();
        let other_id = iced::window::Id::unique();
        terminal.wallet_detail_windows.insert(
            first_id,
            WalletDetailsWindowState::new(CONNECTED.to_string()),
        );
        terminal
            .wallet_detail_windows
            .insert(other_id, WalletDetailsWindowState::new(OTHER.to_string()));
        terminal.rotate_wallet_detail_user_data_stream(CONNECTED);
        terminal.rotate_wallet_detail_user_data_stream(OTHER);
        let (_, before, _) = terminal.user_data_subscription_params();
        let first_generation = before
            .iter()
            .find(|params| params.address.as_deref() == Some(CONNECTED))
            .expect("first detail params")
            .generation;
        let other_generation = before
            .iter()
            .find(|params| params.address.as_deref() == Some(OTHER))
            .expect("other detail params")
            .generation;

        terminal.wallet_detail_windows.remove(&first_id);
        terminal.remove_wallet_detail_user_data_stream(CONNECTED);
        terminal.wallet_detail_windows.insert(
            iced::window::Id::unique(),
            WalletDetailsWindowState::new(CONNECTED.to_string()),
        );
        terminal.rotate_wallet_detail_user_data_stream(CONNECTED);
        let (_, after, _) = terminal.user_data_subscription_params();

        assert_ne!(
            after
                .iter()
                .find(|params| params.address.as_deref() == Some(CONNECTED))
                .expect("reopened detail params")
                .generation,
            first_generation
        );
        assert_eq!(
            after
                .iter()
                .find(|params| params.address.as_deref() == Some(OTHER))
                .expect("unchanged detail params")
                .generation,
            other_generation
        );
    }

    #[test]
    fn config_clear_removes_wallet_detail_stream_params() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(CONNECTED.to_string());
        terminal.wallet_detail_windows.insert(
            iced::window::Id::unique(),
            WalletDetailsWindowState::new(OTHER.to_string()),
        );
        terminal.rotate_wallet_detail_user_data_stream(OTHER);

        let (_, before_clear, _) = terminal.user_data_subscription_params();
        assert_eq!(before_clear.len(), 1);
        assert!(
            !terminal
                .wallet_detail_user_data_stream_generations
                .is_empty()
        );

        let _task = terminal.apply_config_clear_to_runtime(ClearConfigSummary {
            files_removed: 0,
            file_cleanup_failed: false,
            keychain_entries_cleared: 0,
            warnings: Vec::new(),
        });
        let (_, after_clear, cluster_after_clear) = terminal.user_data_subscription_params();

        assert!(after_clear.is_empty());
        assert!(cluster_after_clear.is_empty());
        assert!(
            terminal
                .wallet_detail_user_data_stream_generations
                .is_empty()
        );
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
