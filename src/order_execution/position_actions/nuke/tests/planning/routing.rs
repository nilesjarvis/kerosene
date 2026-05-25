use super::*;

#[test]
fn planner_mixes_ready_and_skipped_positions_without_dropping_skips() {
    let plan = plan_or_panic(
        plan_nuke_positions_from_inputs(
            vec![
                nuke_input("BTC", "2.5", true, Some(perp_sym()), Some(100.0)),
                nuke_input("SHIB", "1000", true, Some(perp_sym()), None),
                nuke_input("DOGE", "3", true, None, Some(0.1)),
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
                nuke_input("SHIB", "1000", true, Some(perp_sym()), None),
                nuke_input("DOGE", "3", true, None, Some(0.1)),
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
fn planner_ignores_hidden_or_muted_positions_before_size_parsing() {
    let plan = plan_or_panic(
        plan_nuke_positions_from_inputs(
            vec![
                nuke_input(
                    "HIDDEN",
                    "not-a-number",
                    false,
                    Some(perp_sym()),
                    Some(10.0),
                ),
                nuke_input("MUTED", "NaN", false, Some(perp_sym()), Some(10.0)),
                nuke_input("BTC", "2.5", true, Some(perp_sym()), Some(100.0)),
            ],
            DEFAULT_MARKET_SLIPPAGE,
        ),
        "hidden/muted malformed sizes must not abort visible NUKE planning",
    );

    assert_eq!(plan.format_ready_list(), "BTC");
    assert!(plan.skipped.is_empty());
}

#[test]
fn visible_malformed_position_size_still_aborts_the_plan() {
    let err = plan_error_or_panic(
        plan_nuke_positions_from_inputs(
            vec![
                nuke_input(
                    "HIDDEN",
                    "not-a-number",
                    false,
                    Some(perp_sym()),
                    Some(10.0),
                ),
                nuke_input("BTC", "NaN", true, Some(perp_sym()), Some(100.0)),
            ],
            DEFAULT_MARKET_SLIPPAGE,
        ),
        "visible malformed size should fail closed",
    );

    assert_eq!(err, "NUKE aborted: non-finite position size for BTC");
}
