mod data;
mod execution;
mod management;
mod orders;
mod positions;

use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::trim_decimal_zeros;
use crate::message::Message;
use crate::order_execution::{OneShotPlacementContext, PreparedExchangeOrder};
use iced::Task;

const POSITION_EPSILON: f64 = 1e-12;

struct ClusterTradingMember {
    profile_secret_id: String,
    label: String,
    address: String,
    agent_key: crate::signing::CapturedAgentKey,
    weight: f64,
}

struct PreparedClusterLeg {
    member: ClusterTradingMember,
    request: crate::signing::PlaceOrderRequest,
    context: OneShotPlacementContext,
    is_buy: bool,
    size: String,
    price: String,
    market_type: MarketType,
}

impl PreparedClusterLeg {
    fn new(member: ClusterTradingMember, order: PreparedExchangeOrder, is_buy: bool) -> Self {
        let (request, context) = order.place_request_with_context(&member.address);
        Self {
            member,
            request,
            context,
            is_buy,
            size: order.size,
            price: order.price,
            market_type: order.market_type,
        }
    }
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

    fn set_wallet_cluster_status(&mut self, message: impl Into<String>, is_error: bool) {
        self.wallet_clusters.status = Some((message.into(), is_error));
    }
}
