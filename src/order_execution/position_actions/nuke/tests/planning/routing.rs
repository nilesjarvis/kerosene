use super::*;

#[test]
fn planner_mixes_ready_and_skipped_positions_without_dropping_skips() {
    let plan = plan_or_panic(
        plan_nuke_positions_from_inputs(
            vec![
                nuke_input("BTC", "2.5", false, Some(perp_sym()), Some(100.0)),
                nuke_input("SHIB", "1000", false, Some(perp_sym()), None),
                nuke_input("DOGE", "3", false, None, Some(0.1)),
            ],
            DEFAULT_MARKET_SLIPPAGE,
        ),
        "mixed plan should not error",
    );

    assert_eq!(plan.format_ready_list(), "BTC");
    assert_eq!(
        plan.skipped,
        vec![
            ("SHIB".to_string(), NukeSkipReason::NoMidPrice),
            ("DOGE".to_string(), NukeSkipReason::UnknownAsset),
        ]
    );
}

#[test]
fn planner_reports_all_unrouteable_visible_positions_without_ready_orders() {
    let plan = plan_or_panic(
        plan_nuke_positions_from_inputs(
            vec![
                nuke_input("SHIB", "1000", false, Some(perp_sym()), None),
                nuke_input("DOGE", "3", false, None, Some(0.1)),
            ],
            DEFAULT_MARKET_SLIPPAGE,
        ),
        "skip-only plan should not error",
    );

    assert!(plan.ready.is_empty());
    assert_eq!(plan.skipped.len(), 2);
    assert_eq!(
        plan.format_skip_list(),
        "SHIB (no mid price), DOGE (unknown asset)"
    );
}

#[test]
fn planner_includes_hidden_positions_in_emergency_plan() {
    let plan = plan_or_panic(
        plan_nuke_positions_from_inputs(
            vec![
                nuke_input("HIDDEN", "1.5", true, Some(perp_sym()), Some(10.0)),
                nuke_input("MUTED", "-2.0", true, Some(perp_sym()), Some(10.0)),
                nuke_input("BTC", "2.5", false, Some(perp_sym()), Some(100.0)),
            ],
            DEFAULT_MARKET_SLIPPAGE,
        ),
        "hidden/muted positions should be routed during NUKE planning",
    );

    assert_eq!(plan.format_ready_list(), "HIDDEN, MUTED, BTC");
    assert!(plan.skipped.is_empty());
    assert!(plan.hidden_skipped.is_empty());
}

#[test]
fn hidden_malformed_position_size_aborts_the_plan() {
    let err = plan_error_or_panic(
        plan_nuke_positions_from_inputs(
            vec![
                nuke_input("HIDDEN", "not-a-number", true, Some(perp_sym()), Some(10.0)),
                nuke_input("BTC", "2.5", false, Some(perp_sym()), Some(100.0)),
            ],
            DEFAULT_MARKET_SLIPPAGE,
        ),
        "hidden malformed size should fail closed",
    );

    assert!(err.starts_with("NUKE aborted: invalid position size for HIDDEN"));
}

#[test]
fn planner_tracks_hidden_unrouteable_positions_for_execution_abort() {
    let plan = plan_or_panic(
        plan_nuke_positions_from_inputs(
            vec![
                nuke_input("HIDDEN", "1.5", true, Some(perp_sym()), None),
                nuke_input("BTC", "2.5", false, Some(perp_sym()), Some(100.0)),
            ],
            DEFAULT_MARKET_SLIPPAGE,
        ),
        "hidden unrouteable position should be a plan skip",
    );

    assert_eq!(plan.format_ready_list(), "BTC");
    assert_eq!(
        plan.skipped,
        vec![("HIDDEN".to_string(), NukeSkipReason::NoMidPrice)]
    );
    assert_eq!(plan.hidden_skipped, plan.skipped);
    assert_eq!(plan.format_hidden_skip_list(), "HIDDEN (no mid price)");
}
