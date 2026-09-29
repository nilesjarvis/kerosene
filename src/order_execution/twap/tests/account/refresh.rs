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
