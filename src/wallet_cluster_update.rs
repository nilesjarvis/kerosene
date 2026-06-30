use crate::account::{
    AccountData, AccountDataFetchScope, WalletDetailsData, WalletOpenOrderDetail,
    fetch_wallet_details_scoped_with_provider, normalize_dex_open_order_coins,
};
use crate::api::{MarketType, OrderStatusResult, fetch_order_status_by_cloid};
use crate::app_state::TradingTerminal;
use crate::helpers::{
    parse_finite_number, parse_positive_number, redact_sensitive_response_text, trim_decimal_zeros,
};
use crate::message::{Message, RedactedAccountKey};
use crate::order_execution::{
    MarketUsdSizeReference, OneShotPlacementContext, OrderSurface, PlaceIntent, PriceSource,
    QuantityDenomination, QuantitySource, ReduceOnlySource, place_order_task,
};
use crate::order_update::{ExecutionOutcomeKind, classify_execution_result};
use crate::read_data_provider::ReadDataRequestContext;
use crate::signing::{ExchangeOrderKind, ExchangeResponse, OrderKind};
use crate::wallet_cluster_state::{
    MAX_WALLET_CLUSTER_MEMBERS, WalletCluster, WalletClusterCloseSide, WalletClusterExecution,
    WalletClusterExecutionKind, WalletClusterExecutionLeg, WalletClusterLegStatus,
    WalletClusterMember, WalletClusterMemberData, WalletClusterPositionMember,
    WalletClusterPositionSummary, format_weight_input, parse_member_weight,
    wallet_cluster_window_settings,
};
use crate::ws::WsUserData;
use iced::{Task, window};
use zeroize::Zeroizing;

const POSITION_EPSILON: f64 = 1e-12;

#[derive(Clone)]
struct ClusterTradingMember {
    profile_secret_id: String,
    label: String,
    address: String,
    agent_key: Zeroizing<String>,
    weight: f64,
}

#[derive(Clone)]
struct PreparedClusterLeg {
    member: ClusterTradingMember,
    request: crate::signing::PlaceOrderRequest,
    context: OneShotPlacementContext,
    is_buy: bool,
    size: String,
    price: String,
}

impl TradingTerminal {
    pub(crate) fn update_wallet_cluster(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenWalletClustersWindow => self.open_wallet_clusters_window(),
            Message::WalletClusterNameInputChanged(value) => {
                self.wallet_clusters.new_cluster_name_input = value;
                Task::none()
            }
            Message::WalletClusterCreate => self.create_wallet_cluster(),
            Message::WalletClusterSelected(cluster_id) => self.select_wallet_cluster(cluster_id),
            Message::WalletClusterRenamed(cluster_id, value) => {
                self.rename_wallet_cluster(cluster_id, value)
            }
            Message::WalletClusterDeleted(cluster_id) => self.delete_wallet_cluster(cluster_id),
            Message::WalletClusterAddMember(profile_secret_id) => {
                self.add_wallet_cluster_member(profile_secret_id)
            }
            Message::WalletClusterRemoveMember(cluster_id, profile_key) => {
                self.remove_wallet_cluster_member(cluster_id, profile_key.into_option())
            }
            Message::WalletClusterMemberWeightChanged(cluster_id, profile_key, value) => self
                .change_wallet_cluster_member_weight(
                    cluster_id,
                    profile_key.into_option(),
                    value.into_string(),
                ),
            Message::WalletClusterRefresh => self.refresh_selected_wallet_cluster(),
            Message::WalletClusterMemberLoaded(
                cluster_id,
                profile_key,
                address,
                context,
                result,
            ) => self.apply_wallet_cluster_member_loaded(
                cluster_id,
                profile_key.into_option(),
                address.into_string(),
                context,
                *result,
            ),
            Message::WalletClusterWsUpdate(source_address, data) => self
                .apply_wallet_cluster_ws_update(
                    source_address.map(|address| address.into_string()),
                    *data,
                ),
            Message::WalletClusterOrderPriceChanged(value) => {
                self.wallet_clusters.order_price = value.into_string();
                Task::none()
            }
            Message::WalletClusterOrderQuantityChanged(value) => {
                self.wallet_clusters.order_quantity = value.into_string();
                Task::none()
            }
            Message::WalletClusterToggleOrderDenomination => {
                self.wallet_clusters.order_quantity_is_usd =
                    !self.wallet_clusters.order_quantity_is_usd;
                Task::none()
            }
            Message::WalletClusterSetOrderKind(order_kind) => {
                self.wallet_clusters.order_kind = order_kind;
                Task::none()
            }
            Message::WalletClusterToggleReduceOnly => {
                self.wallet_clusters.reduce_only = !self.wallet_clusters.reduce_only;
                Task::none()
            }
            Message::WalletClusterSetMidPrice => {
                if let Some(mid) = self.resolve_mid_for_symbol(&self.active_symbol) {
                    self.wallet_clusters.order_price = trim_decimal_zeros(mid.to_string());
                } else {
                    self.set_wallet_cluster_status(
                        format!(
                            "No mid price for {}",
                            self.display_name_for_symbol(&self.active_symbol)
                        ),
                        true,
                    );
                }
                Task::none()
            }
            Message::WalletClusterSubmitOrder { is_buy } => {
                self.submit_wallet_cluster_order(is_buy)
            }
            Message::WalletClusterClosePosition {
                symbol,
                side,
                fraction,
                use_market,
            } => self.submit_wallet_cluster_close_position(symbol, side, fraction, use_market),
            Message::WalletClusterOrderResult {
                execution_id,
                member_key,
                context,
                result,
            } => self.apply_wallet_cluster_order_result(
                execution_id,
                member_key.into_option(),
                context,
                *result,
            ),
            Message::WalletClusterOrderStatusLoaded {
                execution_id,
                member_key,
                context,
                result,
            } => self.apply_wallet_cluster_order_status_result(
                execution_id,
                member_key.into_option(),
                context,
                *result,
            ),
            _ => Task::none(),
        }
    }

    fn open_wallet_clusters_window(&mut self) -> Task<Message> {
        if let Some(window_id) = self.wallet_clusters.window_id {
            return window::gain_focus(window_id);
        }

        let settings =
            wallet_cluster_window_settings(&self.wallet_clusters, self.custom_window_chrome_active);
        let (window_id, open_task) = window::open(settings);
        self.wallet_clusters.window_id = Some(window_id);
        self.wallet_clusters.open = true;
        self.persist_config();
        Task::batch([
            open_task.map(Message::WindowOpened),
            self.refresh_selected_wallet_cluster(),
        ])
    }

    fn create_wallet_cluster(&mut self) -> Task<Message> {
        let name = self.wallet_clusters.new_cluster_name_input.trim();
        let name = if name.is_empty() {
            format!("Cluster {}", self.wallet_clusters.clusters.len() + 1)
        } else {
            name.to_string()
        };
        let id = crate::config::new_secret_id();
        self.wallet_clusters.clusters.push(WalletCluster {
            id: id.clone(),
            name,
            members: Vec::new(),
        });
        self.wallet_clusters.selected_cluster_id = Some(id);
        self.wallet_clusters.new_cluster_name_input.clear();
        self.wallet_clusters.status = None;
        self.persist_config();
        Task::none()
    }

    fn select_wallet_cluster(&mut self, cluster_id: String) -> Task<Message> {
        if self
            .wallet_clusters
            .clusters
            .iter()
            .any(|cluster| cluster.id == cluster_id)
        {
            self.wallet_clusters.selected_cluster_id = Some(cluster_id);
            self.wallet_clusters.status = None;
            self.persist_config();
            self.refresh_selected_wallet_cluster()
        } else {
            Task::none()
        }
    }

    fn rename_wallet_cluster(&mut self, cluster_id: String, value: String) -> Task<Message> {
        if let Some(cluster) = self
            .wallet_clusters
            .clusters
            .iter_mut()
            .find(|cluster| cluster.id == cluster_id)
        {
            cluster.name = value;
            self.persist_config();
        }
        Task::none()
    }

    fn delete_wallet_cluster(&mut self, cluster_id: String) -> Task<Message> {
        if self.wallet_clusters.has_pending_execution() {
            self.set_wallet_cluster_status(
                "Wait for pending cluster executions to finish before deleting a cluster",
                true,
            );
            return Task::none();
        }
        let before = self.wallet_clusters.clusters.len();
        self.wallet_clusters
            .clusters
            .retain(|cluster| cluster.id != cluster_id);
        if self.wallet_clusters.clusters.len() == before {
            return Task::none();
        }

        if self.wallet_clusters.selected_cluster_id.as_deref() == Some(&cluster_id) {
            self.wallet_clusters.selected_cluster_id = self
                .wallet_clusters
                .clusters
                .first()
                .map(|cluster| cluster.id.clone());
        }
        self.wallet_clusters.member_data.clear();
        self.wallet_clusters.status = None;
        self.persist_config();
        self.refresh_selected_wallet_cluster()
    }

    fn add_wallet_cluster_member(&mut self, profile_secret_id: String) -> Task<Message> {
        if self
            .accounts
            .iter()
            .all(|profile| profile.secret_id != profile_secret_id)
        {
            self.set_wallet_cluster_status("Account profile no longer exists", true);
            return Task::none();
        }
        // Watch-only accounts can't sign, so they can never produce a valid
        // cluster leg. Reject them at the boundary (the add-row UI also filters
        // them) instead of only failing later at submission time.
        if self.ghost_account_secret_ids.contains(&profile_secret_id) {
            self.set_wallet_cluster_status(
                "Watch-only accounts cannot be added to a trading cluster",
                true,
            );
            return Task::none();
        }
        // Likewise require a committed agent key (the add-row UI hides keyless
        // profiles); without one the member could never sign a leg.
        if self.accounts.iter().any(|profile| {
            profile.secret_id == profile_secret_id && profile.agent_key.trim().is_empty()
        }) {
            self.set_wallet_cluster_status(
                "Account needs a committed agent key to join a trading cluster",
                true,
            );
            return Task::none();
        }

        let Some(cluster) = self.wallet_clusters.selected_cluster_mut() else {
            self.set_wallet_cluster_status("Create or select a cluster first", true);
            return Task::none();
        };
        if cluster
            .members
            .iter()
            .any(|member| member.profile_secret_id == profile_secret_id)
        {
            self.set_wallet_cluster_status("Account is already in this cluster", true);
            return Task::none();
        }
        if cluster.members.len() >= MAX_WALLET_CLUSTER_MEMBERS {
            self.set_wallet_cluster_status(
                format!("A cluster can include at most {MAX_WALLET_CLUSTER_MEMBERS} wallets"),
                true,
            );
            return Task::none();
        }

        cluster.members.push(WalletClusterMember {
            profile_secret_id: profile_secret_id.clone(),
            weight: crate::config::default_wallet_cluster_member_weight(),
            weight_input: format_weight_input(crate::config::default_wallet_cluster_member_weight()),
        });
        self.persist_config();
        self.refresh_wallet_cluster_member(profile_secret_id)
    }

    fn remove_wallet_cluster_member(
        &mut self,
        cluster_id: String,
        profile_secret_id: Option<String>,
    ) -> Task<Message> {
        let Some(profile_secret_id) = profile_secret_id else {
            return Task::none();
        };
        let Some(cluster) = self
            .wallet_clusters
            .clusters
            .iter_mut()
            .find(|cluster| cluster.id == cluster_id)
        else {
            return Task::none();
        };
        cluster
            .members
            .retain(|member| member.profile_secret_id != profile_secret_id);
        self.wallet_clusters.member_data.remove(&profile_secret_id);
        self.persist_config();
        Task::none()
    }

    fn change_wallet_cluster_member_weight(
        &mut self,
        cluster_id: String,
        profile_secret_id: Option<String>,
        value: String,
    ) -> Task<Message> {
        let Some(profile_secret_id) = profile_secret_id else {
            return Task::none();
        };
        let Some(cluster) = self
            .wallet_clusters
            .clusters
            .iter_mut()
            .find(|cluster| cluster.id == cluster_id)
        else {
            return Task::none();
        };
        let Some(member) = cluster
            .members
            .iter_mut()
            .find(|member| member.profile_secret_id == profile_secret_id)
        else {
            return Task::none();
        };

        member.weight_input = value.clone();
        if let Some(weight) = parse_member_weight(&value) {
            member.weight = weight;
            self.wallet_clusters.status = None;
            self.persist_config();
        } else {
            self.set_wallet_cluster_status("Member weight must be a non-negative number", true);
        }
        Task::none()
    }

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

    fn refresh_wallet_cluster_member(&mut self, profile_secret_id: String) -> Task<Message> {
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

    fn apply_wallet_cluster_member_loaded(
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
        let exchange_symbols = self.exchange_symbols.clone();
        let muted_tickers = self.muted_tickers.clone();
        let market_universe = self.market_universe.clone();
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
                    &exchange_symbols,
                    &muted_tickers,
                    &market_universe,
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

    fn apply_wallet_cluster_ws_update(
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
        let exchange_symbols = self.exchange_symbols.clone();
        let muted_tickers = self.muted_tickers.clone();
        let market_universe = self.market_universe.clone();
        let is_hidden = |symbol: &str| {
            Self::symbol_key_is_hidden_with(
                &exchange_symbols,
                &muted_tickers,
                &market_universe,
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

    /// Ensure every trading member has a fresh *position* snapshot before a
    /// position-sensitive action. If any are stale or missing, refresh ALL of
    /// them at once and return the batch task; the caller aborts the action and
    /// the user retries once the refreshes land. Returns `None` when all are
    /// fresh. Refreshing the whole batch (rather than the first stale member)
    /// avoids needing one click per stale member.
    fn ensure_cluster_members_fresh(
        &mut self,
        members: &[ClusterTradingMember],
        action: &str,
    ) -> Option<Task<Message>> {
        let now_ms = Self::now_ms();
        let stale: Vec<String> = members
            .iter()
            .filter(|member| {
                !self
                    .wallet_clusters
                    .member_data
                    .get(&member.profile_secret_id)
                    .is_some_and(|data| cluster_member_snapshot_is_fresh(data, now_ms))
            })
            .map(|member| member.profile_secret_id.clone())
            .collect();
        if stale.is_empty() {
            return None;
        }
        self.set_wallet_cluster_status(
            format!(
                "Refreshing {} stale snapshot(s) before {action}",
                stale.len()
            ),
            true,
        );
        let tasks: Vec<_> = stale
            .into_iter()
            .map(|profile_secret_id| self.refresh_wallet_cluster_member(profile_secret_id))
            .collect();
        Some(Task::batch(tasks))
    }

    fn submit_wallet_cluster_order(&mut self, is_buy: bool) -> Task<Message> {
        if self.has_pending_trading_request() {
            self.set_wallet_cluster_status(
                "Wait for pending trading requests to finish before submitting cluster orders",
                true,
            );
            return Task::none();
        }
        let Some(cluster) = self.wallet_clusters.selected_cluster().cloned() else {
            self.set_wallet_cluster_status("Create or select a cluster first", true);
            return Task::none();
        };
        let symbol = self.active_symbol.clone();
        let order_kind = match ExchangeOrderKind::try_from(self.wallet_clusters.order_kind) {
            Ok(kind) => kind,
            Err(error) => {
                self.set_wallet_cluster_status(error, true);
                return Task::none();
            }
        };
        let total_quantity = match parse_positive_number(&self.wallet_clusters.order_quantity) {
            Some(quantity) => quantity,
            None => {
                self.set_wallet_cluster_status("Enter a positive cluster order size", true);
                return Task::none();
            }
        };

        let members = match self.cluster_trading_members(&cluster, true) {
            Ok(members) => members,
            Err(error) => {
                self.set_wallet_cluster_status(error, true);
                return Task::none();
            }
        };
        let total_weight: f64 = members.iter().map(|member| member.weight).sum();
        if total_weight <= POSITION_EPSILON {
            self.set_wallet_cluster_status(
                "At least one cluster member needs a positive weight",
                true,
            );
            return Task::none();
        }

        // Reduce-only orders are gated on the local position snapshot (see
        // cluster_leg_reduces_position); require it to be fresh first, like the
        // close path, rather than trusting possibly-stale data.
        if self.wallet_clusters.reduce_only
            && let Some(task) =
                self.ensure_cluster_members_fresh(&members, "submitting reduce-only orders")
        {
            return task;
        }

        let price_source = match order_kind {
            ExchangeOrderKind::Market => PriceSource::MarketWithSlippage {
                invalid_message: Some("Invalid market price"),
                usd_size_reference: MarketUsdSizeReference::ExecutionPrice,
            },
            ExchangeOrderKind::Limit | ExchangeOrderKind::LimitIoc => PriceSource::LimitInput {
                value: self.wallet_clusters.order_price.clone(),
                invalid_message: "Invalid limit price",
            },
        };
        let denomination = if self.wallet_clusters.order_quantity_is_usd {
            QuantityDenomination::UsdNotional
        } else {
            QuantityDenomination::Coin
        };

        let mut prepared = Vec::new();
        for member in members {
            let allocated_quantity = total_quantity * member.weight / total_weight;
            if allocated_quantity <= POSITION_EPSILON {
                continue;
            }
            let intent = PlaceIntent {
                surface: OrderSurface::Cluster,
                symbol_key: symbol.clone(),
                is_buy,
                order_kind,
                price_source: price_source.clone(),
                quantity_source: QuantitySource::UserInput {
                    value: allocated_quantity.to_string(),
                    denomination,
                    invalid_message: "Invalid cluster order size",
                    precision_invalid_message: "Cluster order size is below asset precision",
                },
                reduce_only_source: ReduceOnlySource::Form(self.wallet_clusters.reduce_only),
            };
            let order = match self.prepare_place_order(intent) {
                Ok(order) => order,
                Err(error) => {
                    self.set_wallet_cluster_status(format!("{}: {error}", member.label), true);
                    return Task::none();
                }
            };
            // The opposite-side guard only applies to perp positions (szi).
            // Spot has no perp position to inspect (and prepare_place_order
            // strips the Form reduce-only flag to false for spot anyway), so the
            // guard would only ever wrongly block a spot leg — skip it there.
            if self.wallet_clusters.reduce_only
                && self.market_type_for_symbol(&symbol) == Some(MarketType::Perp)
                && !self.cluster_leg_reduces_position(
                    &member.profile_secret_id,
                    &symbol,
                    is_buy,
                    &order.size,
                )
            {
                self.set_wallet_cluster_status(
                    format!(
                        "{} does not have enough opposite-side position to reduce",
                        member.label
                    ),
                    true,
                );
                return Task::none();
            }
            let (request, context) = order.place_request_with_context(&member.address);
            prepared.push(PreparedClusterLeg {
                member,
                request,
                context,
                is_buy,
                size: order.size,
                price: order.price,
            });
        }

        self.start_wallet_cluster_execution(
            cluster,
            WalletClusterExecutionKind::Order,
            symbol,
            self.wallet_clusters.order_kind,
            prepared,
        )
    }

    fn submit_wallet_cluster_close_position(
        &mut self,
        symbol: String,
        side: WalletClusterCloseSide,
        fraction: f64,
        use_market: bool,
    ) -> Task<Message> {
        if self.has_pending_trading_request() {
            self.set_wallet_cluster_status(
                "Wait for pending trading requests to finish before closing cluster positions",
                true,
            );
            return Task::none();
        }
        let Some(cluster) = self.wallet_clusters.selected_cluster().cloned() else {
            self.set_wallet_cluster_status("Create or select a cluster first", true);
            return Task::none();
        };
        let fraction = if fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if fraction <= POSITION_EPSILON {
            self.set_wallet_cluster_status("Close fraction must be positive", true);
            return Task::none();
        }
        // Non-market closes rest a Limit at the reference mid, matching the
        // connected-account close. A LimitIoc at the bare (non-crossing) mid
        // would demand an immediate match at mid and routinely cancel with zero
        // fill, so the partial-close buttons would silently do nothing.
        let order_kind = if use_market {
            ExchangeOrderKind::Market
        } else {
            ExchangeOrderKind::Limit
        };
        let price_source = if use_market {
            PriceSource::MarketWithSlippage {
                invalid_message: Some("Invalid close price"),
                usd_size_reference: MarketUsdSizeReference::ExecutionPrice,
            }
        } else {
            PriceSource::ReferenceMid
        };
        let is_buy = side.is_buy_to_close();

        let members = match self.cluster_trading_members(&cluster, true) {
            Ok(members) => members,
            Err(error) => {
                self.set_wallet_cluster_status(error, true);
                return Task::none();
            }
        };
        // Closes must be sized from fresh positions. Refresh any stale/missing
        // members in one batch and let the user retry, rather than sizing a
        // reduce-only close from stale szi.
        if let Some(task) = self.ensure_cluster_members_fresh(&members, "closing positions") {
            return task;
        }
        let summaries = self.wallet_cluster_position_summaries();
        let mut prepared = Vec::new();

        for member in members {
            let size = cluster_close_size_for_member(
                &summaries,
                &symbol,
                &member.profile_secret_id,
                side,
                fraction,
            );
            let Some(size) = size else {
                continue;
            };

            let intent = PlaceIntent {
                surface: OrderSurface::ClusterClose,
                symbol_key: symbol.clone(),
                is_buy,
                order_kind,
                price_source: price_source.clone(),
                quantity_source: QuantitySource::CoinSize {
                    size,
                    invalid_message: "Invalid cluster close size",
                    precision_invalid_message: "Cluster close size is below asset precision",
                },
                reduce_only_source: ReduceOnlySource::Fixed(true),
            };
            let order = match self.prepare_place_order(intent) {
                Ok(order) => order,
                Err(error) => {
                    self.set_wallet_cluster_status(format!("{}: {error}", member.label), true);
                    return Task::none();
                }
            };
            let (request, context) = order.place_request_with_context(&member.address);
            prepared.push(PreparedClusterLeg {
                member,
                request,
                context,
                is_buy,
                size: order.size,
                price: order.price,
            });
        }

        if prepared.is_empty() {
            self.set_wallet_cluster_status(
                format!(
                    "No {} positions to close for {}",
                    side.label(),
                    self.display_name_for_symbol(&symbol)
                ),
                true,
            );
            return Task::none();
        }

        self.start_wallet_cluster_execution(
            cluster,
            WalletClusterExecutionKind::Close,
            symbol,
            // Record the kind actually sent for the close legs (Market or a
            // resting Limit), not the open-ticket selector.
            if use_market {
                OrderKind::Market
            } else {
                OrderKind::Limit
            },
            prepared,
        )
    }

    fn start_wallet_cluster_execution(
        &mut self,
        cluster: WalletCluster,
        kind: WalletClusterExecutionKind,
        symbol: String,
        order_kind: OrderKind,
        prepared: Vec<PreparedClusterLeg>,
    ) -> Task<Message> {
        if prepared.is_empty() {
            self.set_wallet_cluster_status("No eligible cluster members to submit", true);
            return Task::none();
        }
        let execution_id = self.wallet_clusters.next_execution_id;
        self.wallet_clusters.next_execution_id =
            self.wallet_clusters.next_execution_id.wrapping_add(1);

        let mut tasks = Vec::with_capacity(prepared.len());
        let mut legs = Vec::with_capacity(prepared.len());
        for leg in prepared {
            let member_key: RedactedAccountKey = Some(leg.member.profile_secret_id.clone()).into();
            let context = leg.context.clone();
            let key = leg.member.agent_key.clone();
            tasks.push(place_order_task(key, leg.request, move |result| {
                Message::WalletClusterOrderResult {
                    execution_id,
                    member_key,
                    context,
                    result: Box::new(result),
                }
            }));
            legs.push(WalletClusterExecutionLeg {
                profile_secret_id: leg.member.profile_secret_id,
                address: leg.member.address,
                label: leg.member.label,
                symbol: symbol.clone(),
                is_buy: leg.is_buy,
                size: leg.size,
                price: leg.price,
                cloid: leg.context.cloid,
                status: WalletClusterLegStatus::Pending,
                message: "Submitted".to_string(),
            });
        }

        let execution = WalletClusterExecution {
            id: execution_id,
            cluster_name: cluster.display_name(),
            kind,
            symbol: symbol.clone(),
            order_kind,
            created_at_ms: Self::now_ms(),
            legs,
        };
        self.wallet_clusters.push_execution(execution);
        self.set_wallet_cluster_status(
            format!(
                "Submitted {} cluster legs for {}",
                tasks.len(),
                self.display_name_for_symbol(&symbol)
            ),
            false,
        );
        Task::batch(tasks)
    }

    fn apply_wallet_cluster_order_result(
        &mut self,
        execution_id: u64,
        profile_secret_id: Option<String>,
        context: OneShotPlacementContext,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let Some(profile_secret_id) = profile_secret_id else {
            return Task::none();
        };
        let outcome = classify_execution_result(result);
        if matches!(
            outcome.kind,
            ExecutionOutcomeKind::Ambiguous | ExecutionOutcomeKind::TransportUnknown
        ) {
            self.update_wallet_cluster_leg(
                execution_id,
                &profile_secret_id,
                &context.cloid,
                WalletClusterLegStatus::Checking,
                format!("Status unknown: {}; checking orderStatus", outcome.status),
            );
            let request_context = context.clone();
            let followup = Task::batch([
                self.refresh_wallet_cluster_member(profile_secret_id.clone()),
                Task::perform(
                    fetch_order_status_by_cloid(
                        context.account_address.clone(),
                        context.cloid.clone(),
                    ),
                    move |result| Message::WalletClusterOrderStatusLoaded {
                        execution_id,
                        member_key: Some(profile_secret_id).into(),
                        context: request_context,
                        result: Box::new(result),
                    },
                ),
            ]);
            return self.finish_wallet_cluster_execution_update(execution_id, followup);
        }

        let (status, message) = if outcome.kind == ExecutionOutcomeKind::AcceptedResting
            && !context.order_kind.allows_resting_response()
        {
            (
                WalletClusterLegStatus::Uncertain,
                format!(
                    "Unexpected resting response for non-resting order: {}; refresh and cancel {} if needed",
                    outcome.status, context.cloid
                ),
            )
        } else if matches!(
            outcome.kind,
            ExecutionOutcomeKind::AcceptedResting | ExecutionOutcomeKind::Filled
        ) {
            (WalletClusterLegStatus::Confirmed, outcome.status)
        } else {
            (WalletClusterLegStatus::Failed, outcome.status)
        };
        self.update_wallet_cluster_leg(
            execution_id,
            &profile_secret_id,
            &context.cloid,
            status,
            message,
        );
        let refresh = outcome
            .refresh_account
            .then(|| self.refresh_wallet_cluster_member(profile_secret_id));
        self.finish_wallet_cluster_execution_update(
            execution_id,
            refresh.unwrap_or_else(Task::none),
        )
    }

    fn apply_wallet_cluster_order_status_result(
        &mut self,
        execution_id: u64,
        profile_secret_id: Option<String>,
        context: OneShotPlacementContext,
        result: Result<OrderStatusResult, String>,
    ) -> Task<Message> {
        let Some(profile_secret_id) = profile_secret_id else {
            return Task::none();
        };
        let (status, message) = match result {
            Ok(result) if result.is_open() && !context.order_kind.allows_resting_response() => (
                WalletClusterLegStatus::Uncertain,
                format!(
                    "orderStatus reports an unexpected resting order: {}; cancel {} if needed",
                    result.raw_summary, context.cloid
                ),
            ),
            Ok(result) if result.is_open() || result.is_filled() => {
                (WalletClusterLegStatus::Confirmed, result.raw_summary)
            }
            Ok(result) if result.is_definitive_no_fill_terminal() => {
                (WalletClusterLegStatus::Failed, result.raw_summary)
            }
            // Note: is_missing() (Hyperliquid "unknownOid") is deliberately NOT
            // treated as Failed here. orderStatus-by-cloid can report unknownOid
            // for an order that WAS accepted (post-placement indexing lag), so
            // the one-shot and NUKE paths classify it as uncertain too; calling
            // it Failed would invite a re-submit that doubles exposure.
            Ok(result) => (WalletClusterLegStatus::Uncertain, result.raw_summary),
            Err(error) => (
                WalletClusterLegStatus::Uncertain,
                redact_sensitive_response_text(&error),
            ),
        };
        self.update_wallet_cluster_leg(
            execution_id,
            &profile_secret_id,
            &context.cloid,
            status,
            message,
        );
        let followup = self.refresh_wallet_cluster_member(profile_secret_id);
        self.finish_wallet_cluster_execution_update(execution_id, followup)
    }

    fn update_wallet_cluster_leg(
        &mut self,
        execution_id: u64,
        profile_secret_id: &str,
        cloid: &str,
        status: WalletClusterLegStatus,
        message: String,
    ) {
        let Some(execution) = self
            .wallet_clusters
            .executions
            .iter_mut()
            .find(|execution| execution.id == execution_id)
        else {
            return;
        };
        if let Some(leg) = execution
            .legs
            .iter_mut()
            .find(|leg| leg.profile_secret_id == profile_secret_id && leg.cloid == cloid)
        {
            leg.status = status;
            leg.message = message;
        }
    }

    fn finish_wallet_cluster_execution_update(
        &mut self,
        execution_id: u64,
        followup: Task<Message>,
    ) -> Task<Message> {
        let Some(execution) = self
            .wallet_clusters
            .executions
            .iter()
            .find(|execution| execution.id == execution_id)
        else {
            return followup;
        };
        let complete = execution.is_complete();
        let problem_count = execution.problem_count();
        let status = if complete {
            if problem_count == 0 {
                format!(
                    "Cluster execution completed: {}/{} confirmed",
                    execution.completed_count(),
                    execution.legs.len()
                )
            } else {
                format!(
                    "Cluster execution completed with {} problem legs: {}/{} finished",
                    problem_count,
                    execution.completed_count(),
                    execution.legs.len()
                )
            }
        } else {
            format!(
                "Cluster execution progress: {}/{} legs finished",
                execution.completed_count(),
                execution.legs.len()
            )
        };
        self.set_wallet_cluster_status(status, problem_count > 0);
        followup
    }

    fn cluster_trading_members(
        &self,
        cluster: &WalletCluster,
        require_positive_weight: bool,
    ) -> Result<Vec<ClusterTradingMember>, String> {
        let mut members = Vec::new();
        let mut seen_addresses = std::collections::HashSet::new();
        for member in &cluster.members {
            if require_positive_weight && member.weight <= POSITION_EPSILON {
                continue;
            }
            let Some(profile) = self
                .accounts
                .iter()
                .find(|profile| profile.secret_id == member.profile_secret_id)
            else {
                return Err("A cluster member profile no longer exists".to_string());
            };
            if self.ghost_account_secret_ids.contains(&profile.secret_id) {
                return Err(format!(
                    "{} is watch-only and cannot sign cluster orders",
                    profile.name
                ));
            }
            let Some(address) = Self::normalize_wallet_address(&profile.wallet_address) else {
                return Err(format!("{} needs a valid wallet address", profile.name));
            };
            if !seen_addresses.insert(address.clone()) {
                return Err(format!(
                    "Cluster contains duplicate wallet address {}",
                    Self::short_address(&address)
                ));
            }
            let agent_key = Zeroizing::new(profile.agent_key.trim().to_string());
            if agent_key.is_empty() {
                return Err(format!("{} needs a committed agent key", profile.name));
            }
            let label = if profile.name.trim().is_empty() {
                self.wallet_display(&address).primary
            } else {
                profile.name.trim().to_string()
            };
            members.push(ClusterTradingMember {
                profile_secret_id: profile.secret_id.clone(),
                label,
                address,
                agent_key,
                weight: member.weight,
            });
        }
        if members.is_empty() {
            Err("No eligible cluster members".to_string())
        } else {
            Ok(members)
        }
    }

    fn cluster_leg_reduces_position(
        &self,
        profile_secret_id: &str,
        symbol: &str,
        is_buy: bool,
        size: &str,
    ) -> bool {
        let Some(size) = parse_finite_number(size).filter(|size| *size > POSITION_EPSILON) else {
            return false;
        };
        let Some(state) = self.wallet_clusters.member_data.get(profile_secret_id) else {
            return false;
        };
        let Some(data) = state.data.as_ref() else {
            return false;
        };
        let available: f64 = data
            .positions
            .iter()
            .filter(|position| position.asset_position.position.coin == symbol)
            .filter_map(|position| parse_finite_number(&position.asset_position.position.szi))
            .filter(|szi| {
                if is_buy {
                    *szi < -POSITION_EPSILON
                } else {
                    *szi > POSITION_EPSILON
                }
            })
            .map(f64::abs)
            .sum();
        available + POSITION_EPSILON >= size
    }

    pub(crate) fn wallet_cluster_position_summaries(&self) -> Vec<WalletClusterPositionSummary> {
        let mut summaries: Vec<WalletClusterPositionSummary> = Vec::new();
        let Some(cluster) = self.wallet_clusters.selected_cluster() else {
            return summaries;
        };

        for member in &cluster.members {
            let Some(state) = self
                .wallet_clusters
                .member_data
                .get(&member.profile_secret_id)
            else {
                continue;
            };
            let Some(data) = state.data.as_ref() else {
                continue;
            };
            let label = self
                .accounts
                .iter()
                .find(|profile| profile.secret_id == member.profile_secret_id)
                .map(|profile| profile.name.trim().to_string())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| self.wallet_display(&state.address).primary);
            for position in &data.positions {
                let coin = position.asset_position.position.coin.clone();
                let Some(size) = parse_finite_number(&position.asset_position.position.szi) else {
                    continue;
                };
                if size.abs() <= POSITION_EPSILON {
                    continue;
                }
                let entry_price = parse_finite_number(&position.asset_position.position.entry_px);
                let value = parse_finite_number(&position.asset_position.position.position_value)
                    .or_else(|| {
                        self.resolve_mid_for_symbol(&coin)
                            .map(|mid| mid * size.abs())
                    });
                let unrealized_pnl =
                    parse_finite_number(&position.asset_position.position.unrealized_pnl);
                let summary_index = summaries.iter().position(|summary| summary.symbol == coin);
                let index = if let Some(index) = summary_index {
                    index
                } else {
                    summaries.push(WalletClusterPositionSummary {
                        symbol: coin.clone(),
                        net_size: 0.0,
                        long_size: 0.0,
                        short_size: 0.0,
                        value: Some(0.0),
                        unrealized_pnl: Some(0.0),
                        members: Vec::new(),
                    });
                    summaries.len() - 1
                };
                let summary = &mut summaries[index];
                summary.net_size += size;
                if size > 0.0 {
                    summary.long_size += size;
                } else {
                    summary.short_size += size.abs();
                }
                add_optional(&mut summary.value, value);
                add_optional(&mut summary.unrealized_pnl, unrealized_pnl);
                summary.members.push(WalletClusterPositionMember {
                    profile_secret_id: member.profile_secret_id.clone(),
                    address: state.address.clone(),
                    label: label.clone(),
                    dex: position.dex.clone(),
                    size,
                    entry_price,
                    value,
                    unrealized_pnl,
                });
            }
        }

        summaries.sort_by(|a, b| {
            b.value
                .unwrap_or(0.0)
                .partial_cmp(&a.value.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        summaries
    }

    fn set_wallet_cluster_status(&mut self, message: impl Into<String>, is_error: bool) {
        self.wallet_clusters.status = Some((message.into(), is_error));
    }
}

fn add_optional(target: &mut Option<f64>, value: Option<f64>) {
    match (target.as_mut(), value) {
        (Some(total), Some(value)) => *total += value,
        (None, Some(value)) => *target = Some(value),
        (Some(_), None) => *target = None,
        (None, None) => {}
    }
}

/// Total size to close for one member on `symbol`, summed across every dex it
/// holds a same-side position on, scaled by `fraction`. Returns `None` when the
/// member has no closeable position on that side (or the result rounds to ~0).
///
/// Summing across dexes is safe because cluster closes are reduce-only
/// (`Fixed(true)`): the exchange caps each leg's fill at the on-venue position,
/// so an over-estimate can only under-close, never over-close or flip.
fn cluster_close_size_for_member(
    summaries: &[WalletClusterPositionSummary],
    symbol: &str,
    profile_secret_id: &str,
    side: WalletClusterCloseSide,
    fraction: f64,
) -> Option<f64> {
    summaries
        .iter()
        .find(|summary| summary.symbol == symbol)
        .map(|summary| {
            summary
                .members
                .iter()
                .filter(|position| {
                    position.profile_secret_id == profile_secret_id
                        && ((matches!(side, WalletClusterCloseSide::Long) && position.size > 0.0)
                            || (matches!(side, WalletClusterCloseSide::Short)
                                && position.size < 0.0))
                })
                .map(|position| position.size.abs())
                .sum::<f64>()
        })
        .map(|total| total * fraction)
        .filter(|size| *size > POSITION_EPSILON)
}

fn cluster_member_snapshot_is_fresh(state: &WalletClusterMemberData, now_ms: u64) -> bool {
    state
        .positions_refreshed_ms
        .and_then(|fetched_at| now_ms.checked_sub(fetched_at))
        .is_some_and(|age| age <= AccountData::POSITION_ACTION_MAX_AGE_MS)
        && !state.stale
        && state.data.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{ClearinghouseState, MarginSummary, SpotClearinghouseState};

    const ADDRESS: &str = "0x1111111111111111111111111111111111111111";

    fn empty_details() -> WalletDetailsData {
        WalletDetailsData {
            clearinghouse: ClearinghouseState {
                margin_summary: MarginSummary {
                    account_value: "0".to_string(),
                    total_ntl_pos: "0".to_string(),
                    total_margin_used: "0".to_string(),
                },
                cross_margin_summary: None,
                cross_maintenance_margin_used: None,
                withdrawable: "0".to_string(),
                asset_positions: Vec::new(),
            },
            spot: SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            positions: Vec::new(),
            open_orders: Vec::new(),
            fills: Vec::new(),
            warnings: Vec::new(),
            fetched_at_ms: 0,
        }
    }

    fn member_data_at(
        address: &str,
        positions_refreshed_ms: Option<u64>,
        stale: bool,
    ) -> WalletClusterMemberData {
        WalletClusterMemberData {
            address: address.to_string(),
            positions_refreshed_ms,
            stale,
            ..WalletClusterMemberData::default()
        }
    }

    #[test]
    fn snapshot_freshness_requires_recent_positions_not_stale() {
        let now = 1_000_000;
        let recent = now - 1_000;
        let old = now - (AccountData::POSITION_ACTION_MAX_AGE_MS + 1);

        // No position timestamp -> never fresh.
        assert!(!cluster_member_snapshot_is_fresh(
            &member_data_at(ADDRESS, None, false),
            now
        ));
        // Recent positions but no data -> not fresh.
        assert!(!cluster_member_snapshot_is_fresh(
            &member_data_at(ADDRESS, Some(recent), false),
            now
        ));
        // Old positions -> not fresh.
        let mut old_with_data = member_data_at(ADDRESS, Some(old), false);
        old_with_data.data = Some(empty_details());
        assert!(!cluster_member_snapshot_is_fresh(&old_with_data, now));
        // Recent positions + data + not stale -> fresh.
        let mut fresh = member_data_at(ADDRESS, Some(recent), false);
        fresh.data = Some(empty_details());
        assert!(cluster_member_snapshot_is_fresh(&fresh, now));
        // Stale flag overrides recent positions.
        fresh.stale = true;
        assert!(!cluster_member_snapshot_is_fresh(&fresh, now));
    }

    fn position_member(profile: &str, dex: &str, size: f64) -> WalletClusterPositionMember {
        WalletClusterPositionMember {
            profile_secret_id: profile.to_string(),
            address: ADDRESS.to_string(),
            label: profile.to_string(),
            dex: dex.to_string(),
            size,
            entry_price: None,
            value: None,
            unrealized_pnl: None,
        }
    }

    fn summary(
        symbol: &str,
        members: Vec<WalletClusterPositionMember>,
    ) -> WalletClusterPositionSummary {
        WalletClusterPositionSummary {
            symbol: symbol.to_string(),
            net_size: 0.0,
            long_size: 0.0,
            short_size: 0.0,
            value: None,
            unrealized_pnl: None,
            members,
        }
    }

    #[test]
    fn cluster_close_size_sums_same_side_positions_across_dexes() {
        let summaries = vec![summary(
            "BTC",
            vec![
                position_member("m1", "", 2.0),
                position_member("m1", "builder", 3.0),
                position_member("m2", "", -4.0),
            ],
        )];

        // m1 is long 2 + 3 across two dexes; full close -> 5, half close -> 2.5.
        assert_eq!(
            cluster_close_size_for_member(
                &summaries,
                "BTC",
                "m1",
                WalletClusterCloseSide::Long,
                1.0
            ),
            Some(5.0)
        );
        assert_eq!(
            cluster_close_size_for_member(
                &summaries,
                "BTC",
                "m1",
                WalletClusterCloseSide::Long,
                0.5
            ),
            Some(2.5)
        );
        // m1 has no short; m2 has no long.
        assert_eq!(
            cluster_close_size_for_member(
                &summaries,
                "BTC",
                "m1",
                WalletClusterCloseSide::Short,
                1.0
            ),
            None
        );
        assert_eq!(
            cluster_close_size_for_member(
                &summaries,
                "BTC",
                "m2",
                WalletClusterCloseSide::Short,
                1.0
            ),
            Some(4.0)
        );
        assert_eq!(
            cluster_close_size_for_member(
                &summaries,
                "BTC",
                "m2",
                WalletClusterCloseSide::Long,
                1.0
            ),
            None
        );
        // Unknown member or symbol -> None.
        assert_eq!(
            cluster_close_size_for_member(
                &summaries,
                "BTC",
                "ghost",
                WalletClusterCloseSide::Long,
                1.0
            ),
            None
        );
        assert_eq!(
            cluster_close_size_for_member(
                &summaries,
                "ETH",
                "m1",
                WalletClusterCloseSide::Long,
                1.0
            ),
            None
        );
    }

    #[test]
    fn cluster_close_size_filters_to_requested_side_when_member_is_hedged() {
        // Same member + symbol, hedged across dexes: long on main, short on builder.
        let summaries = vec![summary(
            "BTC",
            vec![
                position_member("m1", "", 2.0),
                position_member("m1", "builder", -1.0),
            ],
        )];
        assert_eq!(
            cluster_close_size_for_member(
                &summaries,
                "BTC",
                "m1",
                WalletClusterCloseSide::Long,
                1.0
            ),
            Some(2.0)
        );
        assert_eq!(
            cluster_close_size_for_member(
                &summaries,
                "BTC",
                "m1",
                WalletClusterCloseSide::Short,
                1.0
            ),
            Some(1.0)
        );
    }

    #[test]
    fn open_orders_ws_frame_does_not_refresh_position_freshness() {
        // Regression for the freshness-gate defeat: a non-position frame
        // (open orders) must NOT bump positions_refreshed_ms or clear the
        // stale flag the close gate depends on. The timestamp/stale bookkeeping
        // runs after the optional `details` update, so it is independent of
        // whether `data` is populated.
        let mut terminal = TradingTerminal::boot().0;
        let stale_ts = 123;
        terminal.wallet_clusters.member_data.insert(
            "member".to_string(),
            member_data_at(ADDRESS, Some(stale_ts), true),
        );

        let _ = terminal.apply_wallet_cluster_ws_update(
            Some(ADDRESS.to_string()),
            WsUserData::OpenOrders {
                dex: String::new(),
                orders: Vec::new(),
            },
        );

        let state = &terminal.wallet_clusters.member_data["member"];
        assert_eq!(state.positions_refreshed_ms, Some(stale_ts));
        assert!(
            state.stale,
            "open-orders frame must not clear the stale flag"
        );
    }

    #[test]
    fn lagged_ws_frame_invalidates_position_freshness_through_optimistic_refresh() {
        use crate::config::AccountProfile;
        use crate::wallet_cluster_state::{WalletCluster, WalletClusterMember};

        let mut terminal = TradingTerminal::boot().0;
        terminal.accounts = vec![AccountProfile {
            secret_id: "member-profile".to_string(),
            name: "Member".to_string(),
            wallet_address: ADDRESS.to_string(),
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
        // Member currently looks fresh (recent positions, not stale).
        let mut data = member_data_at(ADDRESS, Some(1_000_000), false);
        data.data = Some(empty_details());
        terminal
            .wallet_clusters
            .member_data
            .insert("member-profile".to_string(), data);

        let _ = terminal.apply_wallet_cluster_ws_update(
            Some(ADDRESS.to_string()),
            WsUserData::Lagged { skipped: 7 },
        );

        // The lag-triggered refresh clears `stale` optimistically, but the
        // position timestamp is invalidated so the freshness gate stays closed
        // (a close / reduce-only re-checks before acting) until fresh data
        // actually lands.
        let state = &terminal.wallet_clusters.member_data["member-profile"];
        assert_eq!(state.positions_refreshed_ms, None);
        assert!(!cluster_member_snapshot_is_fresh(state, 1_000_000));
    }

    #[test]
    fn cluster_trading_members_excludes_zero_weight_members_when_required() {
        use crate::config::AccountProfile;
        use crate::wallet_cluster_state::{WalletCluster, WalletClusterMember};

        let terminal = {
            let mut terminal = TradingTerminal::boot().0;
            terminal.accounts = vec![
                AccountProfile {
                    secret_id: "disabled-profile".to_string(),
                    name: "Disabled".to_string(),
                    wallet_address: ADDRESS.to_string(),
                    agent_key: "disabled-agent-key".to_string().into(),
                    hydromancer_api_key: String::new().into(),
                },
                AccountProfile {
                    secret_id: "enabled-profile".to_string(),
                    name: "Enabled".to_string(),
                    wallet_address: "0x2222222222222222222222222222222222222222".to_string(),
                    agent_key: "enabled-agent-key".to_string().into(),
                    hydromancer_api_key: String::new().into(),
                },
            ];
            terminal
        };
        let cluster = WalletCluster {
            id: "cluster".to_string(),
            name: "Cluster".to_string(),
            members: vec![
                WalletClusterMember {
                    profile_secret_id: "disabled-profile".to_string(),
                    weight: 0.0,
                    weight_input: "0".to_string(),
                },
                WalletClusterMember {
                    profile_secret_id: "enabled-profile".to_string(),
                    weight: 1.0,
                    weight_input: "1".to_string(),
                },
            ],
        };

        let members = terminal
            .cluster_trading_members(&cluster, true)
            .expect("positive-weight member should be eligible");

        assert_eq!(members.len(), 1);
        assert_eq!(members[0].profile_secret_id, "enabled-profile");
    }

    #[test]
    fn add_member_rejects_profile_without_agent_key() {
        use crate::config::AccountProfile;
        use crate::wallet_cluster_state::WalletCluster;

        let mut terminal = TradingTerminal::boot().0;
        terminal.accounts = vec![AccountProfile {
            secret_id: "keyless".to_string(),
            name: "Keyless".to_string(),
            wallet_address: ADDRESS.to_string(),
            agent_key: String::new().into(),
            hydromancer_api_key: String::new().into(),
        }];
        terminal.wallet_clusters.clusters = vec![WalletCluster {
            id: "cluster".to_string(),
            name: "Cluster".to_string(),
            members: Vec::new(),
        }];
        terminal.wallet_clusters.selected_cluster_id = Some("cluster".to_string());

        let _ = terminal.add_wallet_cluster_member("keyless".to_string());

        assert!(
            terminal.wallet_clusters.clusters[0].members.is_empty(),
            "keyless profile must not be added"
        );
        let (message, is_error) = terminal
            .wallet_clusters
            .status
            .as_ref()
            .expect("a rejection status should be set");
        assert!(is_error);
        assert!(message.contains("agent key"));
    }
}
