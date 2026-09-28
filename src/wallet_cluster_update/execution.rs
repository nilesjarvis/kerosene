use super::PreparedClusterLeg;
use crate::api::{OrderStatusResult, fetch_order_status_by_cloid};
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::{Message, RedactedAccountKey};
use crate::order_execution::{OneShotPlacementContext, place_order_task};
use crate::order_update::{ExecutionOutcomeKind, classify_execution_result};
use crate::signing::{ExchangeResponse, OrderKind};
use crate::wallet_cluster_state::{
    WalletCluster, WalletClusterExecution, WalletClusterExecutionKind, WalletClusterExecutionLeg,
    WalletClusterLegStatus,
};
use iced::Task;

impl TradingTerminal {
    pub(super) fn start_wallet_cluster_execution(
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
            self.invalidate_spot_balances_after_exchange_dispatch(
                &leg.member.address,
                leg.market_type,
            );
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

    pub(super) fn apply_wallet_cluster_order_result(
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

    pub(super) fn apply_wallet_cluster_order_status_result(
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
}

#[cfg(test)]
mod tests;
