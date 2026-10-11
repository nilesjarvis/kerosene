use super::*;
use crate::order_execution::twap::helpers::TwapAccountRefresh;

#[test]
fn twap_refresh_preserves_policy_account_and_loading_behavior() {
    for policy in [
        TwapAccountRefresh::None,
        TwapAccountRefresh::OnTerminal,
        TwapAccountRefresh::Immediate,
    ] {
        for status in [None, Some(TwapStatus::Running), Some(TwapStatus::Error)] {
            for connected_to_owner in [false, true] {
                for loading in [false, true] {
                    let mut terminal = if connected_to_owner {
                        origin_account_terminal()
                    } else {
                        switched_account_terminal()
                    };
                    terminal.account_loading = loading;
                    terminal.account_reconciliation_required = false;
                    terminal.account_refresh_followup_pending = false;
                    if let Some(status) = status {
                        let mut twap = test_twap(1, CLOID, Instant::now());
                        twap.status = status;
                        terminal.twap_orders.insert(1, twap);
                    }

                    let task = terminal.refresh_after_twap_result(policy, 1);

                    let refresh = status.is_some()
                        && (policy == TwapAccountRefresh::Immediate
                            || policy == TwapAccountRefresh::OnTerminal
                                && status == Some(TwapStatus::Error));
                    assert_eq!(
                        task.units() > 0,
                        refresh && (!connected_to_owner || !loading)
                    );
                    assert_eq!(
                        terminal.account_loading,
                        loading || refresh && connected_to_owner
                    );
                    assert_eq!(
                        terminal.account_refresh_followup_pending,
                        refresh && connected_to_owner && loading
                    );
                    assert_eq!(
                        terminal.account_reconciliation_required,
                        refresh && connected_to_owner
                    );
                    assert_eq!(terminal.twap_orders.get(&1).map(|twap| twap.status), status);
                }
            }
        }
    }
}

#[test]
fn local_builder_rejections_stop_spot_twap_without_consuming_slice_or_checking_order_status() {
    for message in [
        "Builder fee is not approved",
        "Builder approval check failed with HTTP 429",
        "Builder address must be a nonzero 0x-prefixed 20-byte address",
        "Builder approval check timed out; check approval in Settings before retrying",
    ] {
        let mut terminal = origin_account_terminal();
        let mut data = empty_account_data();
        data.mark_spot_balances_fetched_at(TradingTerminal::now_ms());
        set_account_data_for_connected_account(&mut terminal, data);
        terminal.invalidate_spot_balances_after_exchange_dispatch(
            ORIGIN_ADDRESS,
            crate::api::MarketType::Spot,
        );
        let mut twap = pending_twap(1, CLOID, Instant::now());
        twap.is_spot = true;
        twap.slices_attempted = 1;
        twap.slices_sent = 1;
        let next_slice_due = twap.next_slice_due;
        let remaining_size = twap.remaining_size;
        terminal.twap_orders.insert(1, twap);

        let task = terminal.handle_twap_slice_result(
            1,
            Ok(crate::signing::ExchangeResponse::local_rejection(message)),
        );

        assert!(task.units() > 0);
        assert!(terminal.account_loading);
        assert!(terminal.account_reconciliation_required);
        assert!(
            !terminal
                .account_data
                .as_ref()
                .expect("snapshot")
                .completeness
                .spot_balances_complete
        );
        let twap = twap_by_id(&terminal, 1);
        assert_eq!(twap.status, TwapStatus::Error);
        assert_eq!(twap.next_slice_due, next_slice_due);
        assert_eq!(twap.remaining_size, remaining_size);
        assert_eq!(twap.filled_size, 0.0);
        assert_eq!(twap.slices_attempted, 1);
        assert!(twap.retry_slice.is_none());
        assert_eq!(twap.child_orders[0].status, TwapChildStatus::Rejected);
        assert_eq!(twap.child_orders[0].retry_count, 0);
        assert!(twap.status_check_cloid.is_none());
        assert!(twap.reconciliation_deadline.is_none());
        assert!(twap.child_orders[0].oid.is_none());
    }
}
