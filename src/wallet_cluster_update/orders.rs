use super::data::cluster_member_snapshot_is_fresh;
use super::positions::cluster_close_size_for_member;
use super::{ClusterTradingMember, POSITION_EPSILON, PreparedClusterLeg};
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::parse_positive_number;
use crate::message::Message;
use crate::order_execution::{
    MarketUsdSizeReference, OrderSurface, PlaceIntent, PriceSource, QuantityDenomination,
    QuantitySource, ReduceOnlySource,
};
use crate::signing::{ExchangeOrderKind, OrderKind};
use crate::wallet_cluster_state::{
    WalletCluster, WalletClusterCloseSide, WalletClusterExecutionKind,
};
use iced::Task;

impl TradingTerminal {
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

    pub(super) fn submit_wallet_cluster_order(&mut self, is_buy: bool) -> Task<Message> {
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
                market_type: order.market_type,
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

    pub(super) fn submit_wallet_cluster_close_position(
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
                market_type: order.market_type,
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
            if profile.agent_key.trim().is_empty() {
                return Err(format!("{} needs a committed agent key", profile.name));
            }
            let agent_key = Self::capture_profile_signing_key(profile)?;
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
}

#[cfg(test)]
mod tests;
