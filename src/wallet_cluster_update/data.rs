use crate::account::{
    AccountData, AccountDataFetchScope, WalletDetailsData, WalletOpenOrderDetail,
    fetch_wallet_details_scoped_with_provider, normalize_dex_open_order_coins,
};
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::read_data_provider::ReadDataRequestContext;
use crate::wallet_cluster_state::{WalletCluster, WalletClusterMemberData};
use crate::ws::WsUserData;
use iced::Task;

impl TradingTerminal {
    fn wallet_cluster_member_fetch_task(
        &self,
        cluster_id: String,
        profile_secret_id: String,
        address: String,
        scope: AccountDataFetchScope,
        read_context: ReadDataRequestContext,
    ) -> Task<Message> {
        let provider = self.read_data_provider;
        let hydromancer_key = self.hydromancer_api_key_for_task();
        Task::perform(
            fetch_wallet_details_scoped_with_provider(
                address.clone(),
                scope,
                provider,
                hydromancer_key,
            ),
            move |result| {
                Message::WalletClusterMemberLoaded(
                    cluster_id,
                    Some(profile_secret_id).into(),
                    address.into(),
                    read_context,
                    Box::new(result),
                )
            },
        )
    }

    pub(crate) fn refresh_selected_wallet_cluster(&mut self) -> Task<Message> {
        let Some(cluster) = self.wallet_clusters.selected_cluster().cloned() else {
            return Task::none();
        };
        self.refresh_wallet_cluster_members(cluster)
    }

    pub(super) fn refresh_wallet_cluster_member(
        &mut self,
        profile_secret_id: String,
    ) -> Task<Message> {
        let Some(cluster) = self.wallet_clusters.selected_cluster().cloned() else {
            return Task::none();
        };
        if !cluster
            .members
            .iter()
            .any(|member| member.profile_secret_id == profile_secret_id)
        {
            return Task::none();
        }
        self.refresh_wallet_cluster_members(WalletCluster {
            members: cluster
                .members
                .into_iter()
                .filter(|member| member.profile_secret_id == profile_secret_id)
                .collect(),
            ..cluster
        })
    }

    fn refresh_wallet_cluster_members(&mut self, cluster: WalletCluster) -> Task<Message> {
        let read_context = self.read_data_request_context();
        let scope = self.account_data_fetch_scope();
        let mut tasks = Vec::new();
        let mut missing = 0usize;

        for member in cluster.members {
            let Some(profile) = self
                .accounts
                .iter()
                .find(|profile| profile.secret_id == member.profile_secret_id)
            else {
                self.wallet_clusters
                    .member_data
                    .remove(&member.profile_secret_id);
                missing += 1;
                continue;
            };
            let Some(address) = Self::normalize_wallet_address(&profile.wallet_address) else {
                self.wallet_clusters.member_data.insert(
                    member.profile_secret_id.clone(),
                    WalletClusterMemberData {
                        error: Some("Profile is missing a valid wallet address".to_string()),
                        ..WalletClusterMemberData::default()
                    },
                );
                continue;
            };
            let state = self
                .wallet_clusters
                .member_data
                .entry(member.profile_secret_id.clone())
                .or_default();
            state.address = address.clone();
            state.loading = true;
            state.loading_context = Some(read_context);
            state.error = None;
            state.stale = false;
            tasks.push(self.wallet_cluster_member_fetch_task(
                cluster.id.clone(),
                member.profile_secret_id,
                address,
                scope.clone(),
                read_context,
            ));
        }

        if missing > 0 {
            let plural = if missing == 1 {
                "profile was"
            } else {
                "profiles were"
            };
            self.set_wallet_cluster_status(
                format!("{missing} cluster member {plural} removed"),
                true,
            );
        }

        Task::batch(tasks)
    }

    pub(super) fn apply_wallet_cluster_member_loaded(
        &mut self,
        cluster_id: String,
        profile_secret_id: Option<String>,
        address: String,
        context: ReadDataRequestContext,
        result: Result<WalletDetailsData, String>,
    ) -> Task<Message> {
        let Some(profile_secret_id) = profile_secret_id else {
            return Task::none();
        };
        if self.wallet_clusters.selected_cluster_id.as_deref() != Some(&cluster_id) {
            return Task::none();
        }
        let context_is_current = self.read_data_request_context_is_current(context);
        let Some(state) = self.wallet_clusters.member_data.get_mut(&profile_secret_id) else {
            return Task::none();
        };
        let Some(address) = Self::normalize_wallet_address(&address) else {
            return Task::none();
        };
        if state.address != address {
            return Task::none();
        }
        if !context_is_current {
            if state.loading && state.loading_context == Some(context) {
                state.loading = false;
                state.loading_context = None;
            }
            return Task::none();
        }

        state.loading = false;
        state.loading_context = None;
        match result {
            Ok(data) => {
                let data = Self::filter_wallet_details_for_hidden_symbols_with(
                    &self.exchange_symbols,
                    &self.muted_tickers,
                    &self.market_universe,
                    data,
                );
                // Full REST snapshot includes positions.
                state.positions_refreshed_ms = Some(data.fetched_at_ms);
                state.data = Some(data);
                state.error = None;
                state.stale = false;
            }
            Err(error) => {
                state.error = Some(redact_sensitive_response_text(&error));
            }
        }
        Task::none()
    }

    pub(super) fn apply_wallet_cluster_ws_update(
        &mut self,
        address: Option<String>,
        data: WsUserData,
    ) -> Task<Message> {
        let Some(address) = address.as_deref().and_then(Self::normalize_wallet_address) else {
            if let WsUserData::AllMids(mids) = data {
                return self.handle_mids_update(mids);
            }
            return Task::none();
        };

        let now_ms = Self::now_ms();
        let is_hidden = |symbol: &str| {
            Self::symbol_key_is_hidden_with(
                &self.exchange_symbols,
                &self.muted_tickers,
                &self.market_universe,
                symbol,
            )
        };

        match data {
            WsUserData::AllDexPositions {
                main_state,
                states_by_dex: _,
                all_positions,
                position_details,
            } => {
                let all_positions: Vec<_> = all_positions
                    .into_iter()
                    .filter(|position| !is_hidden(&position.position.coin))
                    .collect();
                let position_details: Vec<_> = position_details
                    .into_iter()
                    .filter(|position| !is_hidden(&position.asset_position.position.coin))
                    .collect();
                for state in self
                    .wallet_clusters
                    .member_data
                    .values_mut()
                    .filter(|state| state.address == address)
                {
                    if let Some(details) = state.data.as_mut() {
                        details.clearinghouse.margin_summary = main_state.margin_summary.clone();
                        details.clearinghouse.withdrawable = main_state.withdrawable.clone();
                        details.clearinghouse.cross_margin_summary =
                            main_state.cross_margin_summary.clone();
                        details.clearinghouse.cross_maintenance_margin_used =
                            main_state.cross_maintenance_margin_used.clone();
                        details.clearinghouse.asset_positions = all_positions.clone();
                        details.positions = position_details.clone();
                        details.fetched_at_ms = now_ms;
                    }
                    // This frame delivers a fresh full position set.
                    state.positions_refreshed_ms = Some(now_ms);
                    state.error = None;
                    state.stale = false;
                }
            }
            WsUserData::OpenOrders { dex, orders } => {
                let mut orders = orders;
                normalize_dex_open_order_coins(&dex, &mut orders);
                let orders: Vec<_> = orders
                    .into_iter()
                    .filter(|order| !is_hidden(&order.coin))
                    .collect();
                for state in self
                    .wallet_clusters
                    .member_data
                    .values_mut()
                    .filter(|state| state.address == address)
                {
                    if let Some(details) = state.data.as_mut() {
                        details
                            .open_orders
                            .retain(|order| order.dex != dex && !is_hidden(&order.order.coin));
                        details
                            .open_orders
                            .extend(orders.iter().cloned().map(|order| WalletOpenOrderDetail {
                                dex: dex.clone(),
                                order,
                            }));
                        details.fetched_at_ms = now_ms;
                    }
                    // Open-orders frames do not refresh positions: they must not
                    // bump positions_refreshed_ms or clear the stale flag the
                    // close-action freshness gate depends on.
                    state.error = None;
                }
            }
            WsUserData::SpotBalances(balances) => {
                let balances: Vec<_> = balances
                    .into_iter()
                    .filter(|balance| !is_hidden(&balance.coin))
                    .collect();
                for state in self
                    .wallet_clusters
                    .member_data
                    .values_mut()
                    .filter(|state| state.address == address)
                {
                    if let Some(details) = state.data.as_mut() {
                        details.spot.balances = balances.clone();
                        details.fetched_at_ms = now_ms;
                    }
                    // Spot-balance frames do not refresh positions: they must not
                    // bump positions_refreshed_ms or clear the stale flag the
                    // close-action freshness gate depends on.
                    state.error = None;
                }
            }
            WsUserData::AllMids(mids) => return self.handle_mids_update(mids),
            WsUserData::Fills { .. } => {}
            WsUserData::Lagged { skipped } => {
                let mut refreshes = Vec::new();
                let read_context = self.read_data_request_context();
                for (profile_secret_id, state) in self
                    .wallet_clusters
                    .member_data
                    .iter_mut()
                    .filter(|(_, state)| state.address == address)
                {
                    state.error = Some(format!(
                        "Cluster member stream lagged ({skipped} updates skipped); refreshing snapshot"
                    ));
                    state.stale = true;
                    // A lag means we may have missed a position update, so the
                    // current positions are untrustworthy. Invalidate the
                    // position timestamp directly: the synchronous lag-triggered
                    // refresh below clears `stale` optimistically, so the gate
                    // must not rely on `stale` alone to stay closed until fresh
                    // positions actually land.
                    state.positions_refreshed_ms = None;
                    if !state.loading {
                        state.loading = true;
                        state.loading_context = Some(read_context);
                        refreshes.push(profile_secret_id.clone());
                    }
                }
                if !refreshes.is_empty() {
                    let tasks = refreshes.into_iter().map(|profile_secret_id| {
                        self.refresh_wallet_cluster_member(profile_secret_id)
                    });
                    return Task::batch(tasks);
                }
            }
        }

        Task::none()
    }
}

pub(super) fn cluster_member_snapshot_is_fresh(
    state: &WalletClusterMemberData,
    now_ms: u64,
) -> bool {
    state
        .positions_refreshed_ms
        .and_then(|fetched_at| now_ms.checked_sub(fetched_at))
        .is_some_and(|age| age <= AccountData::POSITION_ACTION_MAX_AGE_MS)
        && !state.stale
        && state.data.is_some()
}

#[cfg(test)]
mod tests;
