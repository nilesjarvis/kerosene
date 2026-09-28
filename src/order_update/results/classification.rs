use crate::helpers::redact_sensitive_response_text;
use crate::signing::ExchangeResponse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionOutcomeKind {
    AcceptedResting,
    Filled,
    Cancelled,
    Rejected,
    Ambiguous,
    TransportUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionOutcome {
    pub(crate) kind: ExecutionOutcomeKind,
    pub(crate) status: String,
    pub(crate) is_error: bool,
    pub(crate) refresh_account: bool,
}

pub(crate) fn classify_execution_result(
    result: Result<ExchangeResponse, String>,
) -> ExecutionOutcome {
    match result {
        Ok(response) => {
            let status = response.summary();
            let response_is_error = response.is_error();
            let response_may_have_committed_order = response.has_potential_order_effect();
            let status = if response_is_error {
                redact_sensitive_response_text(&status)
            } else {
                status
            };
            let kind = if response_is_error {
                if response_may_have_committed_order {
                    ExecutionOutcomeKind::Ambiguous
                } else {
                    ExecutionOutcomeKind::Rejected
                }
            } else if status == "Cancelled" {
                ExecutionOutcomeKind::Cancelled
            } else if response.is_ambiguous_order_result() {
                ExecutionOutcomeKind::Ambiguous
            } else if response.is_fully_filled() {
                ExecutionOutcomeKind::Filled
            } else {
                ExecutionOutcomeKind::AcceptedResting
            };
            let is_error = response_is_error || kind == ExecutionOutcomeKind::Ambiguous;
            ExecutionOutcome {
                kind,
                status,
                is_error,
                refresh_account: !response_is_error || response_may_have_committed_order,
            }
        }
        Err(error) => ExecutionOutcome {
            kind: ExecutionOutcomeKind::TransportUnknown,
            status: redact_sensitive_response_text(&error),
            is_error: true,
            refresh_account: true,
        },
    }
}

pub(in crate::order_update) fn result_requires_account_refresh(
    result: &Result<ExchangeResponse, String>,
) -> bool {
    match result {
        Ok(response) => !response.is_error() || response.has_potential_order_effect(),
        // Signed exchange requests can fail locally after the exchange has
        // already accepted the action. Reconcile account state on transport,
        // response-body, or parse failures so basic order paths fail closed
        // instead of leaving open orders/positions stale.
        Err(_) => true,
    }
}
